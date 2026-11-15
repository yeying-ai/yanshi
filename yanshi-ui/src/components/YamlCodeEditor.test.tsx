import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { YamlCodeEditor } from './YamlCodeEditor';

describe('YamlCodeEditor', () => {
  it('renders syntax-highlighted tokens, line numbers, and preserves data-testid on the textarea', () => {
    const sample = [
      '# header comment',
      'name: studio_demo',
      'nodes:',
      '  - id: step1',
      '    type: render_slides',
      '    input_mapping:',
      '      title: "{{ inputs.topic }}"',
    ].join('\n');

    const html = renderToStaticMarkup(
      <YamlCodeEditor
        value={sample}
        onChange={() => {}}
        testId="create-workflow"
        ariaLabel="Workflow YAML"
      />,
    );

    expect(html).toContain('data-testid="create-workflow"');
    expect(html).toContain('yaml-tok--comment');
    expect(html).toContain('yaml-tok--schema-key');
    expect(html).toContain('yaml-tok--node-type');
    expect(html).toContain('yaml-tok--template');
    expect(html).toContain('yaml-tok--key');
    // Line gutter renders lines 1..7
    expect(html).toContain('>7<');
  });
});
