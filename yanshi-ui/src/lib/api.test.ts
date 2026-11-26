import { afterEach, describe, expect, it } from 'vitest';
import {
  apiFetch,
  formatApiError,
  getEmbeddedApiToken,
  isLocalDesktopEngine,
  openDesktopArtifact,
  pickDesktopDirectory,
  resolveApiUrl,
  resolveEffectiveApiToken,
} from './api';
import { requiresHeaderMediaFetch } from '../components/AuthorizedMedia';
import { makePreferenceFetcher } from '../usePreferenceSync';

afterEach(() => {
  delete (globalThis as unknown as { window?: unknown }).window;
});

describe('api helpers & preference fetcher', () => {
  it('resolves relative gateway paths against window.__YANSHI_API_BASE__ (Tauri desktop)', () => {
    (globalThis as unknown as { window?: unknown }).window = {
      __YANSHI_API_BASE__: 'http://127.0.0.1:19421/',
    };
    expect(resolveApiUrl('/v1/preferences')).toBe('http://127.0.0.1:19421/v1/preferences');
    expect(resolveApiUrl('v1/diagnostics')).toBe('http://127.0.0.1:19421/v1/diagnostics');
  });

  it('routes makePreferenceFetcher through apiFetch so Tauri base URL, token, and tenant headers are preserved', async () => {
    (globalThis as unknown as { window?: unknown }).window = {
      __YANSHI_API_BASE__: 'http://127.0.0.1:19421',
    };
    const originalFetch = globalThis.fetch;
    let capturedUrl = '';
    let capturedAuth = '';
    let capturedTenant = '';

    globalThis.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
      capturedUrl = String(input);
      const headers = new Headers(init?.headers);
      capturedAuth = headers.get('Authorization') ?? '';
      capturedTenant = headers.get('X-Yanshi-Tenant') ?? '';
      return new Response(JSON.stringify({ preferences: {} }), { status: 200 });
    }) as typeof fetch;

    try {
      const fetcher = makePreferenceFetcher('tok-xyz');
      await fetcher('/v1/preferences', {
        headers: { 'X-Yanshi-Tenant': 'team-1' },
      });
      expect(capturedUrl).toBe('http://127.0.0.1:19421/v1/preferences');
      expect(capturedAuth).toBe('Bearer tok-xyz');
      expect(capturedTenant).toBe('team-1');

      await apiFetch('/v1/runs', 'tok-xyz', { method: 'POST' }, 'team-2');
      expect(capturedUrl).toBe('http://127.0.0.1:19421/v1/runs');
      expect(capturedTenant).toBe('team-2');
    } finally {
      globalThis.fetch = originalFetch;
    }
  });

  it('formats network connection errors with actionable gateway guidance', () => {
    const msg = formatApiError(new TypeError('Failed to fetch'));
    expect(msg).toContain('yanshi serve');
    expect(formatApiError(new Error('HTTP 401 unauthorized'))).toBe('HTTP 401 unauthorized');
  });

  it('falls back to window.__YANSHI_API_TOKEN__ across apiFetch and AuthorizedMedia', async () => {
    (globalThis as unknown as { window?: unknown }).window = {
      __YANSHI_API_BASE__: 'http://127.0.0.1:19421',
      __YANSHI_API_TOKEN__: '  desktop-loopback-token  ',
    };
    expect(getEmbeddedApiToken()).toBe('desktop-loopback-token');
    expect(resolveEffectiveApiToken('')).toBe('desktop-loopback-token');
    expect(resolveEffectiveApiToken('explicit-token')).toBe('explicit-token');
    expect(requiresHeaderMediaFetch('', 'default')).toBe(true);

    const originalFetch = globalThis.fetch;
    let seenAuth = '';
    globalThis.fetch = (async (_input: RequestInfo | URL, init?: RequestInit) => {
      seenAuth = new Headers(init?.headers).get('Authorization') ?? '';
      return new Response('{}', { status: 200 });
    }) as typeof fetch;
    try {
      await apiFetch('/v1/notebooks', '');
      expect(seenAuth).toBe('Bearer desktop-loopback-token');
    } finally {
      globalThis.fetch = originalFetch;
    }
  });

  it('detects local desktop engine and invokes native directory picker / artifact opener helpers', async () => {
    expect(isLocalDesktopEngine()).toBe(false);
    (globalThis as unknown as { window?: unknown }).window = {
      __YANSHI_API_BASE__: 'http://127.0.0.1:19421',
      __YANSHI_API_TOKEN__: 'desktop-tok',
    };
    expect(isLocalDesktopEngine()).toBe(true);

    const originalFetch = globalThis.fetch;
    const calls: Array<{ url: string; body: string }> = [];
    globalThis.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      calls.push({ url, body: String(init?.body ?? '') });
      if (url.endsWith('/v1/desktop/pick-directory')) {
        return new Response(JSON.stringify({ path: '/Users/demo/project', cancelled: false }), {
          status: 200,
        });
      }
      return new Response(JSON.stringify({ ok: true }), { status: 200 });
    }) as typeof fetch;

    try {
      const picked = await pickDesktopDirectory('', '/Users/demo');
      expect(picked).toEqual({ path: '/Users/demo/project', cancelled: false });
      await openDesktopArtifact('', '/v1/runs/123/preview?path=a.pdf', 'reveal', 'team-a');
      expect(calls).toHaveLength(2);
      expect(calls[0].url).toBe('http://127.0.0.1:19421/v1/desktop/pick-directory');
      expect(calls[1].url).toBe('http://127.0.0.1:19421/v1/desktop/open-artifact');
      expect(JSON.parse(calls[1].body)).toEqual({
        api_path: '/v1/runs/123/preview?path=a.pdf',
        mode: 'reveal',
      });
    } finally {
      globalThis.fetch = originalFetch;
    }
  });
});
