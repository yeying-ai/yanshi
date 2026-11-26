import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';
import { ModelConfigDrawer } from './ModelConfigDrawer';

describe('ModelConfigDrawer', () => {
  it('renders nothing when closed', () => {
    const html = renderToStaticMarkup(
      <ModelConfigDrawer open={false} onClose={vi.fn()} apiToken="" />,
    );
    expect(html).toBe('');
  });

  it('renders provider and model tabs inside the native expandable drawer when open', () => {
    const html = renderToStaticMarkup(
      <ModelConfigDrawer open onClose={vi.fn()} apiToken="" />,
    );
    expect(html).toContain('data-testid="model-config-drawer"');
    expect(html).toContain('data-testid="model-config-tab-providers"');
    expect(html).toContain('data-testid="model-config-tab-models"');
    expect(html).toContain('模型与厂商配置');
  });
});
