import { describe, expect, it } from 'vitest';
import {
  INSTANT_STEP_DEBUG_YAML,
  inspectYamlLocally,
  TECH_SLIDES_VIDEO_YAML,
} from './presets';

describe('inspectYamlLocally', () => {
  it('extracts typed inputs, edges, sub_nodes, and topological levels for tech_slides_video', () => {
    const res = inspectYamlLocally(TECH_SLIDES_VIDEO_YAML);
    expect(res.valid).toBe(true);
    expect(res.name).toBe('tech_slides_video');
    expect(res.nodes.length).toBe(23);
    expect(res.inputs.find((i) => i.name === 'slide_count')?.kind).toBe('number');
    expect(res.inputs.find((i) => i.name === 'step_pause_ms')?.kind).toBe('number');
    expect(res.inputs.find((i) => i.name === 'presenter_mode')?.kind).toBe('string');
    expect(res.inputs.find((i) => i.name === 'export_pptx')?.kind).toBe('boolean');

    const narrate = res.nodes.find((n) => n.id === 'narrate');
    expect(narrate?.sub_nodes?.length).toBe(1);
    expect(narrate?.sub_nodes?.[0].type).toBe('tts');

    expect(res.edges.some((e) => e.kind === 'condition' && e.to === 'deck_pptx')).toBe(true);
    expect(res.edges.some((e) => e.kind === 'condition' && e.to === 'presenter_portrait')).toBe(true);
    expect(res.levels.length).toBeGreaterThanOrEqual(6);
  });

  it('extracts conditional edges for instant_step_debug_demo', () => {
    const res = inspectYamlLocally(INSTANT_STEP_DEBUG_YAML);
    expect(res.valid).toBe(true);
    expect(res.nodes.map((n) => n.id)).toEqual([
      'normalize_brief',
      'core_outline',
      'quiz_branch',
      'assemble_handout',
    ]);
    expect(res.edges.some((e) => e.from === 'normalize_brief' && e.to === 'quiz_branch' && e.kind === 'condition')).toBe(true);
  });

  it('reports cycle diagnostics cleanly without throwing', () => {
    const cyclic = `name: bad_cycle
nodes:
  - id: a
    type: template
    dependencies: ["b"]
  - id: b
    type: template
    dependencies: ["a"]
`;
    const res = inspectYamlLocally(cyclic);
    expect(res.valid).toBe(false);
    expect(res.diagnostics.issues.join(' ')).toContain('循环依赖');
  });
});
