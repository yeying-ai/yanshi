import { afterEach, describe, expect, it } from 'vitest';
import {
  getCurrentRoute,
  navigateTo,
  normalizeUiTarget,
  parseTargetLocation,
  resetRouterForTests,
  shouldInterceptLinkClick,
  subscribeRouteChange,
} from './router';

interface FakeWindowEnv {
  pathname: string;
  search: string;
  hash: string;
  pushed: string[];
  replaced: string[];
}

function installFakeWindow(initialPath: string, initialSearch = '', initialHash = ''): FakeWindowEnv {
  const env: FakeWindowEnv = {
    pathname: initialPath,
    search: initialSearch,
    hash: initialHash,
    pushed: [],
    replaced: [],
  };

  const applyUrl = (url: string) => {
    if (url.startsWith('#')) {
      env.hash = url;
      return;
    }
    const hashIdx = url.indexOf('#');
    const withoutHash = hashIdx >= 0 ? url.slice(0, hashIdx) : url;
    env.hash = hashIdx >= 0 ? url.slice(hashIdx) : '';
    const qIdx = withoutHash.indexOf('?');
    env.pathname = qIdx >= 0 ? withoutHash.slice(0, qIdx) : withoutHash;
    env.search = qIdx >= 0 ? withoutHash.slice(qIdx) : '';
  };

  const fakeWindow = {
    location: {
      get pathname() {
        return env.pathname;
      },
      get search() {
        return env.search;
      },
      get hash() {
        return env.hash;
      },
    },
    history: {
      pushState(_data: unknown, _unused: string, url?: string | URL | null) {
        if (typeof url === 'string') {
          env.pushed.push(url);
          applyUrl(url);
        }
      },
      replaceState(_data: unknown, _unused: string, url?: string | URL | null) {
        if (typeof url === 'string') {
          env.replaced.push(url);
          applyUrl(url);
        }
      },
    },
    addEventListener: () => {},
    removeEventListener: () => {},
  };

  (globalThis as unknown as { window?: unknown }).window = fakeWindow;
  resetRouterForTests();
  return env;
}

afterEach(() => {
  delete (globalThis as unknown as { window?: unknown }).window;
  resetRouterForTests();
});

describe('router', () => {
  it('normalizes bare /ui and root / to /ui/ while preserving /ui?run=... deep links', () => {
    expect(normalizeUiTarget('')).toBe('/ui/');
    expect(normalizeUiTarget('/')).toBe('/ui/');
    expect(normalizeUiTarget('/ui')).toBe('/ui/');
    expect(normalizeUiTarget('/ui/')).toBe('/ui/');
    expect(normalizeUiTarget('/ui?run=abc-123')).toBe('/ui?run=abc-123');
    expect(normalizeUiTarget('/ui/harness/sessions')).toBe('/ui/harness/sessions');
    expect(normalizeUiTarget('ui/diagnostics')).toBe('/ui/diagnostics');
  });

  it('parses pathname and query string cleanly', () => {
    expect(parseTargetLocation('/ui')).toEqual({ pathname: '/ui/', search: '' });
    expect(parseTargetLocation('/ui?run=abc')).toEqual({ pathname: '/ui', search: '?run=abc' });
    expect(parseTargetLocation('/ui/harness/sessions/id-1?tenant=team-a')).toEqual({
      pathname: '/ui/harness/sessions/id-1',
      search: '?tenant=team-a',
    });
  });

  it('uses history.pushState directly when hosted under /ui/* (web gateway & Vite dev)', () => {
    const env = installFakeWindow('/ui/');
    let notifications = 0;
    const unsubscribe = subscribeRouteChange(() => {
      notifications += 1;
    });

    navigateTo('/ui/harness/sessions');
    expect(env.pushed).toEqual(['/ui/harness/sessions']);
    expect(getCurrentRoute()).toEqual({ pathname: '/ui/harness/sessions', search: '' });
    expect(notifications).toBe(1);

    // Navigating to bare /ui normalizes to /ui/ so Vite base '/ui/' never fails
    navigateTo('/ui');
    expect(env.pushed.at(-1)).toBe('/ui/');
    expect(getCurrentRoute()).toEqual({ pathname: '/ui/', search: '' });

    // Preserves /ui?run=<uuid> for E2E contract compatibility
    navigateTo('/ui?run=1234-5678', { replace: true });
    expect(env.replaced).toEqual(['/ui?run=1234-5678']);
    expect(getCurrentRoute()).toEqual({ pathname: '/ui', search: '?run=1234-5678' });

    unsubscribe();
  });

  it('uses hash + memory routing when hosted at root index.html (Tauri embedded bundle)', () => {
    const env = installFakeWindow('/index.html');
    expect(getCurrentRoute()).toEqual({ pathname: '/ui/', search: '' });

    navigateTo('/ui/diagnostics');
    // Does not overwrite pathname /index.html (prevents Tauri asset 404 on reload)
    expect(env.pathname).toBe('/index.html');
    expect(env.hash).toBe('#/ui/diagnostics');
    expect(getCurrentRoute()).toEqual({ pathname: '/ui/diagnostics', search: '' });

    navigateTo('/ui/harness/sessions/sess-1?tenant=alpha');
    expect(env.hash).toBe('#/ui/harness/sessions/sess-1?tenant=alpha');
    expect(getCurrentRoute()).toEqual({
      pathname: '/ui/harness/sessions/sess-1',
      search: '?tenant=alpha',
    });
  });

  it('intercepts plain primary clicks while allowing modified/new-tab clicks', () => {
    const baseClick = {
      button: 0,
      metaKey: false,
      ctrlKey: false,
      shiftKey: false,
      altKey: false,
      defaultPrevented: false,
    };
    expect(shouldInterceptLinkClick(baseClick)).toBe(true);
    expect(shouldInterceptLinkClick(baseClick, '_self')).toBe(true);
    expect(shouldInterceptLinkClick(baseClick, '_blank')).toBe(false);
    expect(shouldInterceptLinkClick({ ...baseClick, metaKey: true })).toBe(false);
    expect(shouldInterceptLinkClick({ ...baseClick, ctrlKey: true })).toBe(false);
    expect(shouldInterceptLinkClick({ ...baseClick, button: 1 })).toBe(false);
    expect(shouldInterceptLinkClick({ ...baseClick, defaultPrevented: true })).toBe(false);
  });
});
