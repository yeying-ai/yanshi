import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';
import { ExpandableDrawer } from './ExpandableDrawer';

describe('ExpandableDrawer', () => {
  it('renders nothing when closed', () => {
    const html = renderToStaticMarkup(
      <ExpandableDrawer open={false} onClose={vi.fn()} title="Hidden Drawer">
        <p>secret body</p>
      </ExpandableDrawer>,
    );
    expect(html).toBe('');
  });

  it('renders a native <dialog> with closedby="any", side/size classes, and no redundant ARIA dialog role', () => {
    const html = renderToStaticMarkup(
      <ExpandableDrawer
        open
        onClose={vi.fn()}
        title="节点配置"
        badge="llm"
        side="right"
        defaultSize="drawer"
        testId="node-drawer"
      >
        <div data-testid="drawer-body-content">Prompt editor</div>
      </ExpandableDrawer>,
    );
    expect(html).toContain('<dialog');
    expect(html).toContain('closedby="any"');
    expect(html).toContain('ys-drawer--right');
    expect(html).toContain('ys-drawer--drawer');
    expect(html).toContain('data-testid="node-drawer"');
    expect(html).toContain('data-testid="drawer-toggle-size"');
    expect(html).toContain('Prompt editor');
    expect(html).not.toContain('role="dialog"');
    expect(html).not.toContain('aria-modal=');
  });

  it('omits resize toggle when allowResize is false', () => {
    const html = renderToStaticMarkup(
      <ExpandableDrawer
        open
        onClose={vi.fn()}
        title="确认弹窗"
        side="center"
        allowResize={false}
      >
        <p>body</p>
      </ExpandableDrawer>,
    );
    expect(html).toContain('ys-drawer--center');
    expect(html).not.toContain('drawer-toggle-size');
  });
});
