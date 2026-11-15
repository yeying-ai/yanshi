import { describe, expect, it } from 'vitest';
import { WORKFLOW_PRESETS } from './presets';
import {
  applyYamlEditorKeyCommand,
  reconstructFromTokens,
  tokenizeYaml,
} from './yamlHighlight';

describe('tokenizeYaml', () => {
  it('preserves every character losslessly across all built-in workflow presets and edge cases', () => {
    for (const preset of WORKFLOW_PRESETS) {
      const lines = tokenizeYaml(preset.yaml);
      expect(reconstructFromTokens(lines)).toBe(preset.yaml);
    }

    const edgeCases = [
      '',
      '\n',
      '\n\n',
      'name: "unclosed string {{ topic }\n  still_editing: 123 # trailing',
      '---\n# header comment\ncrlf: true\r\nnext: 42\r\n...',
      'anchor_def: &base\n  retries: 3\nuse_anchor: *base\ncustom_tag: !env API_KEY',
    ];

    for (const sample of edgeCases) {
      const lines = tokenizeYaml(sample);
      expect(reconstructFromTokens(lines)).toBe(sample);
    }
  });

  it('classifies schema keys, user keys, node types, booleans, numbers, and comments accurately', () => {
    const sample = [
      '# Workflow header comment',
      'name: demo_flow',
      'inputs:',
      '  slide_count:',
      '    default: 6',
      '    required: true',
      'nodes:',
      '  - id: render_step # inline comment',
      '    type: render_slides',
      '    timeout_secs: 30.5',
    ].join('\n');

    const lines = tokenizeYaml(sample);
    expect(reconstructFromTokens(lines)).toBe(sample);

    // Line 0: full-line comment
    expect(lines[0]).toEqual([{ kind: 'comment', text: '# Workflow header comment' }]);

    // Line 1: `name` is a schema-key
    expect(lines[1].some((t) => t.kind === 'schema-key' && t.text === 'name')).toBe(true);

    // Line 3: `slide_count` is a user-defined key
    expect(lines[3].some((t) => t.kind === 'key' && t.text === 'slide_count')).toBe(true);

    // Line 4 & 5: number `6` and boolean `true`
    expect(lines[4].some((t) => t.kind === 'number' && t.text === '6')).toBe(true);
    expect(lines[5].some((t) => t.kind === 'boolean' && t.text === 'true')).toBe(true);

    // Line 7: list `-`, schema-key `id`, inline comment
    expect(lines[7].some((t) => t.kind === 'punctuation' && t.text === '-')).toBe(true);
    expect(lines[7].some((t) => t.kind === 'schema-key' && t.text === 'id')).toBe(true);
    expect(lines[7].some((t) => t.kind === 'comment' && t.text === '# inline comment')).toBe(true);

    // Line 8 & 9: node-type `render_slides` and float `30.5`
    expect(lines[8].some((t) => t.kind === 'node-type' && t.text === 'render_slides')).toBe(true);
    expect(lines[9].some((t) => t.kind === 'number' && t.text === '30.5')).toBe(true);
  });

  it('highlights Tera {{ ... }} and {% ... %} expressions inside quoted strings and multiline | block scalars without mistaking # for YAML comments', () => {
    const sample = [
      'nodes:',
      '  - id: handout',
      '    type: template',
      '    input_mapping:',
      '      headline: "{{ nodes.brief.outputs.headline }} # not a comment"',
      '    parameters:',
      '      template: |',
      '        # 课程讲义汇总: {{ headline }}',
      '        {% if quiz is defined %}{{ quiz }}{% endif %}',
      '  - id: next_node',
      '    type: llm',
    ].join('\n');

    const lines = tokenizeYaml(sample);
    expect(reconstructFromTokens(lines)).toBe(sample);

    // Quoted value on line 4 contains template token AND keeps `# not a comment` as string
    const mappingLine = lines[4];
    expect(
      mappingLine.some(
        (t) => t.kind === 'template' && t.text === '{{ nodes.brief.outputs.headline }}',
      ),
    ).toBe(true);
    expect(mappingLine.some((t) => t.kind === 'comment')).toBe(false);

    // Block scalar indicator `|` on line 6
    expect(lines[6].some((t) => t.kind === 'punctuation' && t.text === '|')).toBe(true);

    // Line 7 is inside `|` block scalar: `# 课程讲义汇总:` is string + template, NOT comment or key!
    expect(lines[7].some((t) => t.kind === 'comment')).toBe(false);
    expect(lines[7].some((t) => t.kind === 'key')).toBe(false);
    expect(lines[7].some((t) => t.kind === 'template' && t.text === '{{ headline }}')).toBe(true);

    // Line 8 highlights both `{% if ... %}` and `{{ quiz }}`
    const teraTokens = lines[8].filter((t) => t.kind === 'template').map((t) => t.text);
    expect(teraTokens).toEqual(['{% if quiz is defined %}', '{{ quiz }}', '{% endif %}']);

    // Line 9 outdents out of block scalar back to `- id: next_node`
    expect(lines[9].some((t) => t.kind === 'schema-key' && t.text === 'id')).toBe(true);
    expect(lines[10].some((t) => t.kind === 'node-type' && t.text === 'llm')).toBe(true);
  });

  it('tokenizes inline flow sequences and flow mappings', () => {
    const sample = [
      'dependencies: ["slides", "narrate"]',
      'title: { type: string, maxLength: 80 }',
    ].join('\n');

    const lines = tokenizeYaml(sample);
    expect(reconstructFromTokens(lines)).toBe(sample);
    expect(lines[0].filter((t) => t.kind === 'string').map((t) => t.text)).toEqual([
      '"slides"',
      '"narrate"',
    ]);
    expect(lines[1].filter((t) => t.kind === 'key').map((t) => t.text)).toContain('maxLength');
    expect(lines[1].some((t) => t.kind === 'number' && t.text === '80')).toBe(true);
  });
});

describe('applyYamlEditorKeyCommand', () => {
  it('indents and outdents 2 spaces with Tab and Shift+Tab', () => {
    const src = 'nodes:\n  - id: a\n    type: llm';
    // Single cursor Tab inserts 2 spaces
    const inserted = applyYamlEditorKeyCommand(src, 7, 7, 'Tab', false);
    expect(inserted).toEqual({
      value: 'nodes:\n    - id: a\n    type: llm',
      selectionStart: 9,
      selectionEnd: 9,
    });

    // Multi-line selection Tab indents both lines
    const multiIndent = applyYamlEditorKeyCommand(src, 9, 24, 'Tab', false);
    expect(multiIndent?.value).toBe('nodes:\n    - id: a\n      type: llm');

    // Shift+Tab outdents both lines by 2 spaces
    const outdented = applyYamlEditorKeyCommand(
      multiIndent!.value,
      multiIndent!.selectionStart,
      multiIndent!.selectionEnd,
      'Tab',
      true,
    );
    expect(outdented?.value).toBe(src);
  });

  it('auto-indents on Enter and adds 2 spaces after mapping colon', () => {
    const src = 'nodes:';
    const res = applyYamlEditorKeyCommand(src, 6, 6, 'Enter', false);
    expect(res).toEqual({
      value: 'nodes:\n  ',
      selectionStart: 9,
      selectionEnd: 9,
    });

    const nested = '  parameters:\n    width: 1920';
    const res2 = applyYamlEditorKeyCommand(nested, nested.length, nested.length, 'Enter', false);
    expect(res2).toEqual({
      value: '  parameters:\n    width: 1920\n    ',
      selectionStart: nested.length + 5,
      selectionEnd: nested.length + 5,
    });
  });
});
