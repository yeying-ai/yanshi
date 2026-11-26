import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { usePaneLayout } from './usePaneLayout';

function Harness(props: Parameters<typeof usePaneLayout>[0]) {
  const layout = usePaneLayout(props);
  return (
    <div
      data-left={layout.leftWidth}
      data-right={layout.rightWidth}
      data-lc={String(layout.leftCollapsed)}
      data-rc={String(layout.rightCollapsed)}
    />
  );
}

describe('usePaneLayout', () => {
  it('initializes with clamped defaults when storage is absent', () => {
    const html = renderToStaticMarkup(
      <Harness
        storageKey="test.pane.layout"
        defaultLeft={320}
        defaultRight={410}
        minLeft={260}
        maxLeft={520}
      />,
    );
    expect(html).toContain('data-left="320"');
    expect(html).toContain('data-right="410"');
    expect(html).toContain('data-lc="false"');
    expect(html).toContain('data-rc="false"');
  });
});
