import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import type { InspectedNode } from '../schemas';
import {
  NodeConfigEditor,
  buildNodePatchFromDraft,
  createNodeEditorDraft,
} from './NodeConfigEditor';

const SAMPLE_NODE: InspectedNode = {
  id: 'polish_copy',
  type: 'llm',
  dependencies: ['draft_outline'],
  input_mapping: {
    outline: '{{ nodes.draft_outline.outputs.output }}',
  },
  run_if: '{{ nodes.draft_outline.outputs.ready }}',
  timeout_ms: 30000,
  max_retries: 2,
  parameters: {
    prompt: 'Refine: {{ outline }}',
    temperature: 0.4,
  },
  level: 1,
};

const UPSTREAM_NODE: InspectedNode = {
  id: 'draft_outline',
  type: 'template',
  dependencies: [],
  input_mapping: {},
  parameters: { template: 'Hello' },
  level: 0,
};

describe('NodeConfigEditor', () => {
  it('renders node identity, input_mapping rows, reliability fields, and prompt editor', () => {
    const html = renderToStaticMarkup(
      <NodeConfigEditor
        node={SAMPLE_NODE}
        allNodes={[UPSTREAM_NODE, SAMPLE_NODE]}
        editable
        onApplyPatch={() => null}
        onDeleteNode={() => {}}
      />,
    );

    expect(html).toContain('data-testid="node-config-editor"');
    expect(html).toContain('polish_copy');
    expect(html).toContain('draft_outline');
    expect(html).toContain('{{ nodes.draft_outline.outputs.output }}');
    expect(html).toContain('Refine: {{ outline }}');
    expect(html).toContain('30000');
  });

  it('builds a validated NodeUpdatePatch from draft state and rejects malformed JSON parameters', () => {
    const draft = createNodeEditorDraft(SAMPLE_NODE);
    draft.idDraft = 'final_polish';
    draft.mappingRows.push({ key: 'tone', expr: '{{ inputs.tone }}' });
    draft.timeoutDraft = '45000';
    draft.retriesDraft = '3';
    draft.quickTextValue = 'Updated prompt: {{ outline }}';

    const built = buildNodePatchFromDraft(SAMPLE_NODE, draft);
    expect(built.error).toBeNull();
    expect(built.patch?.newId).toBe('final_polish');
    expect(built.patch?.timeout_ms).toBe(45000);
    expect(built.patch?.max_retries).toBe(3);
    expect(built.patch?.input_mapping).toEqual({
      outline: '{{ nodes.draft_outline.outputs.output }}',
      tone: '{{ inputs.tone }}',
    });
    expect(built.patch?.parameters).toEqual({
      prompt: 'Updated prompt: {{ outline }}',
      temperature: 0.4,
    });

    const brokenDraft = { ...draft, paramsJsonDraft: '{ broken json' };
    const broken = buildNodePatchFromDraft(SAMPLE_NODE, brokenDraft);
    expect(broken.error).toContain('parameters JSON');
  });
});
