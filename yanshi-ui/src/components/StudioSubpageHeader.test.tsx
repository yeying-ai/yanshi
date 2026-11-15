import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { SpaLink, StudioSubpageHeader } from './StudioSubpageHeader';

describe('StudioSubpageHeader & SpaLink', () => {
  it('renders unified back-to-Studio link, active nav state, and custom action slot', () => {
    const html = renderToStaticMarkup(
      <StudioSubpageHeader
        eyebrow="Yanshi / Harness"
        title="Sessions"
        activeSection="harness"
        actions={
          <SpaLink data-testid="harness-new-link" className="topbar-link topbar-cta" to="/ui/harness/sessions/new">
            + New session
          </SpaLink>
        }
      />,
    );

    expect(html).toContain('data-testid="subpage-back-studio"');
    expect(html).toContain('href="/ui/"');
    expect(html).toContain('aria-current="page"');
    expect(html).toContain('data-testid="harness-new-link"');
    expect(html).toContain('href="/ui/harness/sessions/new"');
  });

  it('normalizes bare /ui in SpaLink href to /ui/', () => {
    const html = renderToStaticMarkup(<SpaLink to="/ui">← Run console</SpaLink>);
    expect(html).toContain('href="/ui/"');
  });
});
