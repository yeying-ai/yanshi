import { describe, expect, it } from 'vitest';
import {
  buildPersonalTemplatesPreferencePatch,
  buildYamlDownloadFilename,
  OFFICIAL_GENERATOR_PRESETS,
  PERSONAL_TEMPLATES_INDEX_KEY,
  personalTemplateYamlKey,
  removePersonalTemplatePatch,
  restorePersonalTemplatesFromPreferences,
} from './generatorCatalog';
import { inspectYamlLocally, WORKFLOW_PRESETS } from './presets';

describe('generatorCatalog & WORKFLOW_PRESETS', () => {
  it('loads all 10 official Content Studio generator templates from examples/generators', () => {
    const expectedIds = [
      'tech_slides_video',
      'slides',
      'video_overview',
      'report',
      'infographic',
      'mindmap',
      'circuit_diagram',
      'audio_overview',
      'flashcards',
      'quiz',
    ];
    expect(OFFICIAL_GENERATOR_PRESETS.map((p) => p.id)).toEqual(expectedIds);

    for (const preset of OFFICIAL_GENERATOR_PRESETS) {
      expect(preset.yaml.trim().length).toBeGreaterThan(100);
      expect(preset.category.length).toBeGreaterThan(0);
      const inspected = inspectYamlLocally(preset.yaml);
      expect(inspected.valid).toBe(true);
      expect(inspected.nodes.length).toBeGreaterThan(0);
      expect(inspected.execution_order.length).toBe(inspected.nodes.length);
    }
  });

  it('exposes official generators plus zero-key instant demos in WORKFLOW_PRESETS', () => {
    const allIds = WORKFLOW_PRESETS.map((p) => p.id);
    expect(allIds).toContain('tech_slides_video');
    expect(allIds).toContain('report');
    expect(allIds).toContain('audio_overview');
    expect(allIds).toContain('instant_slide_render_demo');
    expect(allIds).toContain('instant_step_debug_demo');
    expect(WORKFLOW_PRESETS.length).toBe(12);
  });

  it('builds safe .yml download filenames from workflow names', () => {
    expect(buildYamlDownloadFilename('tech_slides_video')).toBe('tech_slides_video.yml');
    expect(buildYamlDownloadFilename('My Slide Deck / V2!')).toBe('my_slide_deck_v2.yml');
    expect(buildYamlDownloadFilename('   ')).toBe('workflow.yml');
    expect(buildYamlDownloadFilename('already.yaml')).toBe('already.yaml');
  });

  it('round-trips personal workflow templates via per-template preference keys within 16 KiB limit', () => {
    const sampleYaml = 'name: custom_demo\nnodes:\n  - id: step1\n    type: template\n';
    const { nextTemplates, preferencePatch } = buildPersonalTemplatesPreferencePatch(
      [],
      {
        title: '我的常用流程',
        yaml: sampleYaml,
      },
      '2026-11-15T10:00:00.000Z',
    );

    expect(nextTemplates).toHaveLength(1);
    const tpl = nextTemplates[0];
    expect(tpl.title).toBe('我的常用流程');
    expect(preferencePatch[PERSONAL_TEMPLATES_INDEX_KEY]).toBeDefined();
    expect(preferencePatch[personalTemplateYamlKey(tpl.id)]).toEqual({ yaml: sampleYaml });

    const restored = restorePersonalTemplatesFromPreferences(preferencePatch);
    expect(restored).toEqual(nextTemplates);

    const afterRemove = removePersonalTemplatePatch(restored, tpl.id);
    expect(afterRemove.nextTemplates).toEqual([]);
    const restoredAfterRemove = restorePersonalTemplatesFromPreferences({
      ...preferencePatch,
      ...afterRemove.preferencePatch,
    });
    expect(restoredAfterRemove).toEqual([]);
  });

  it('rejects oversized YAML templates above the 15 KiB safe preference envelope ceiling', () => {
    const hugeYaml = `name: huge\n# ${'x'.repeat(16 * 1024)}`;
    expect(() =>
      buildPersonalTemplatesPreferencePatch([], { title: '超长模板', yaml: hugeYaml }),
    ).toThrow(/15 KiB/);
  });
});
