import { describe, expect, it } from 'vitest';
import { inspectYamlLocally } from './presets';
import {
  NODE_PALETTE,
  addNodeToYaml,
  connectNodesInYaml,
  deleteNodeFromYaml,
  disconnectEdgeInYaml,
  updateNodeInYaml,
} from './yamlGraphMutator';

const BASE_WORKFLOW = `# 顶部注释应当保留
name: demo_builder
inputs:
  topic:
    default: "Rust Async"
nodes:
  - id: draft
    type: template
    parameters:
      template: "Outline for {{ topic }}"
  - id: polish
    type: llm
    dependencies:
      - draft
    input_mapping:
      draft_text: "{{ nodes.draft.outputs.output }}"
    parameters:
      prompt: "Polish: {{ draft_text }}"
`;

describe('yamlGraphMutator', () => {
  it('exposes categorized node palette entries with valid default YAML', () => {
    expect(NODE_PALETTE.length).toBeGreaterThanOrEqual(16);
    for (const item of NODE_PALETTE) {
      const res = addNodeToYaml('name: test\nnodes: []\n', item.type);
      expect(res.ok).toBe(true);
      if (res.ok) {
        const inspected = inspectYamlLocally(res.yaml);
        expect(inspected.valid).toBe(true);
        expect(inspected.nodes[0]?.type).toBe(item.type);
      }
    }
  });

  it('adds nodes with collision-free IDs, preserves comments, and supports chaining after an existing node', () => {
    const first = addNodeToYaml(BASE_WORKFLOW, 'render_markdown', {
      afterNodeId: 'polish',
    });
    expect(first.ok).toBe(true);
    if (!first.ok) return;

    expect(first.yaml).toContain('# 顶部注释应当保留');
    expect(first.createdNodeId).toBe('render_markdown_1');

    const second = addNodeToYaml(first.yaml, 'render_markdown');
    expect(second.ok).toBe(true);
    if (!second.ok) return;
    expect(second.createdNodeId).toBe('render_markdown_2');

    const inspected = inspectYamlLocally(second.yaml);
    expect(inspected.valid).toBe(true);
    const chained = inspected.nodes.find((n) => n.id === 'render_markdown_1');
    expect(chained?.dependencies).toContain('polish');
    expect(Object.values(chained?.input_mapping ?? {}).some((v) => v.includes('nodes.polish.outputs'))).toBe(
      true,
    );
  });

  it('connects two nodes with smart input_mapping and rejects self-loops or cycles', () => {
    const added = addNodeToYaml(BASE_WORKFLOW, 'render_document');
    expect(added.ok).toBe(true);
    if (!added.ok || !added.createdNodeId) return;

    const connected = connectNodesInYaml(added.yaml, 'polish', added.createdNodeId);
    expect(connected.ok).toBe(true);
    if (!connected.ok) return;

    const inspected = inspectYamlLocally(connected.yaml);
    expect(inspected.valid).toBe(true);
    expect(inspected.edges.some((e) => e.from === 'polish' && e.to === added.createdNodeId)).toBe(
      true,
    );

    // Self-loop must be rejected
    const selfLoop = connectNodesInYaml(connected.yaml, 'draft', 'draft');
    expect(selfLoop.ok).toBe(false);

    // Back-edge `render_document_1 -> draft` creates a cycle `draft -> polish -> render_document_1 -> draft`
    const cycle = connectNodesInYaml(connected.yaml, added.createdNodeId, 'draft');
    expect(cycle.ok).toBe(false);
    if (!cycle.ok) {
      expect(cycle.error).toContain('环');
    }
  });

  it('disconnects an edge and cleans both dependencies and input_mapping', () => {
    const res = disconnectEdgeInYaml(BASE_WORKFLOW, 'draft', 'polish');
    expect(res.ok).toBe(true);
    if (!res.ok) return;

    const inspected = inspectYamlLocally(res.yaml);
    expect(inspected.valid).toBe(true);
    expect(inspected.edges).toHaveLength(0);
    const polish = inspected.nodes.find((n) => n.id === 'polish');
    expect(polish?.dependencies).toEqual([]);
    expect(polish?.input_mapping).toEqual({});
  });

  it('deletes a node and cascades cleanup to downstream dependencies, input_mapping, and run_if', () => {
    const withCond = `${BASE_WORKFLOW}  - id: gated
    type: template
    dependencies:
      - draft
    run_if: "{{ nodes.draft.outputs.output != '' }}"
    parameters:
      template: "ok"
`;
    const deleted = deleteNodeFromYaml(withCond, 'draft');
    expect(deleted.ok).toBe(true);
    if (!deleted.ok) return;

    const inspected = inspectYamlLocally(deleted.yaml);
    expect(inspected.valid).toBe(true);
    expect(inspected.nodes.map((n) => n.id)).toEqual(['polish', 'gated']);
    const polish = inspected.nodes.find((n) => n.id === 'polish');
    const gated = inspected.nodes.find((n) => n.id === 'gated');
    expect(polish?.dependencies).toEqual([]);
    expect(polish?.input_mapping).toEqual({});
    expect(gated?.dependencies).toEqual([]);
    expect(gated?.run_if).toBeNull();
  });

  it('renames a node with cascading reference updates and preserves map/while sub-pipelines when updating parameters', () => {
    const mapYaml = `name: map_preserve
nodes:
  - id: source_items
    type: template
    parameters:
      template: '["a", "b"]'
  - id: fanout
    type: map
    dependencies:
      - source_items
    input_mapping:
      items: "{{ nodes.source_items.outputs.output }}"
    parameters:
      parallel: false
      template:
        - id: sub_step
          type: template
          parameters:
            template: "item={{ item }}"
`;
    const renamed = updateNodeInYaml(mapYaml, 'source_items', {
      newId: 'prepared_items',
      timeout_ms: 15000,
      max_retries: 2,
    });
    expect(renamed.ok).toBe(true);
    if (!renamed.ok) return;
    expect(renamed.renamedTo).toBe('prepared_items');

    // Update `fanout` parameters (where InspectedNode omits `template` from `parameters`)
    const updatedMap = updateNodeInYaml(renamed.yaml, 'fanout', {
      parameters: { parallel: true, max_concurrent: 4 },
      run_if: '{{ nodes.prepared_items.outputs.output }}',
    });
    expect(updatedMap.ok).toBe(true);
    if (!updatedMap.ok) return;

    const inspected = inspectYamlLocally(updatedMap.yaml);
    expect(inspected.valid).toBe(true);
    const fanout = inspected.nodes.find((n) => n.id === 'fanout');
    expect(fanout?.dependencies).toEqual(['prepared_items']);
    expect(fanout?.input_mapping.items).toBe('{{ nodes.prepared_items.outputs.output }}');
    expect(fanout?.sub_nodes).toHaveLength(1);
    expect(fanout?.sub_nodes?.[0]?.id).toBe('sub_step');
    expect((fanout?.parameters as Record<string, unknown>).parallel).toBe(true);
    expect((fanout?.parameters as Record<string, unknown>).max_concurrent).toBe(4);
  });
});
