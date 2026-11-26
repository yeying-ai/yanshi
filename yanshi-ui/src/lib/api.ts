// HTTP / SSE helpers shared by Web, Tauri Desktop, and Tauri Mobile surfaces.

import { parseJson } from '../schemas';
import { apiBaseUrlKey, readStorage, writeStorage } from './storage';

declare global {
  interface Window {
    /** Injected by the Tauri Desktop shell when its embedded loopback engine boots. */
    __YANSHI_API_BASE__?: string;
    /** Per-launch loopback bearer token injected by Tauri Desktop or `yanshi-studio --web`. */
    __YANSHI_API_TOKEN__?: string;
    /** True when running inside a Tauri v2 WebView shell. */
    __TAURI_INTERNALS__?: unknown;
  }
}

/** Return the per-launch loopback bearer token injected into `window`, if any. */
export const getEmbeddedApiToken = (): string => {
  if (typeof window !== 'undefined' && typeof window.__YANSHI_API_TOKEN__ === 'string') {
    return window.__YANSHI_API_TOKEN__.trim();
  }
  return '';
};

/** Resolve explicit operator token first, falling back to `getEmbeddedApiToken()`. */
export const resolveEffectiveApiToken = (token: string): string => {
  const trimmed = token.trim();
  return trimmed || getEmbeddedApiToken();
};

/** Normalize a gateway base URL (strip trailing slashes). */
export const normalizeApiBaseUrl = (raw: string): string => raw.trim().replace(/\/+$/, '');

/**
 * Return the active gateway origin:
 * 1. Explicit operator override saved in `localStorage` (`yanshi.ui.apiBaseUrl`)
 * 2. Embedded Tauri Desktop loopback server (`window.__YANSHI_API_BASE__`)
 * 3. Same-origin (`""`) when served by `yanshi-server` at `/ui`
 */
export const getApiBaseUrl = (): string => {
  const stored = typeof window !== 'undefined' ? normalizeApiBaseUrl(readStorage(apiBaseUrlKey, '')) : '';
  if (stored) {
    return stored;
  }
  if (typeof window !== 'undefined' && typeof window.__YANSHI_API_BASE__ === 'string') {
    return normalizeApiBaseUrl(window.__YANSHI_API_BASE__);
  }
  return '';
};

export const setApiBaseUrl = (nextUrl: string): void => {
  writeStorage(apiBaseUrlKey, normalizeApiBaseUrl(nextUrl));
};

export const isTauriRuntime = (): boolean =>
  typeof window !== 'undefined' &&
  (Boolean(window.__TAURI_INTERNALS__) || Boolean(window.__YANSHI_API_BASE__));

/**
 * True when connected to the local desktop loopback engine (Tauri GUI or
 * `yanshi-studio --web`) without a remote operator API override.
 */
export const isLocalDesktopEngine = (): boolean => {
  if (typeof window === 'undefined') return false;
  const overrideBase = normalizeApiBaseUrl(readStorage(apiBaseUrlKey, ''));
  if (overrideBase && !/^https?:\/\/(127\.0\.0\.1|localhost|\[::1\])(?::\d+)?$/i.test(overrideBase)) {
    return false;
  }
  return isTauriRuntime() || Boolean(getEmbeddedApiToken());
};

/** Open the OS native directory picker on the desktop loopback engine. */
export const pickDesktopDirectory = async (
  apiToken: string,
  initialDir?: string,
): Promise<{ path: string | null; cancelled: boolean }> => {
  const res = await apiFetch('/v1/desktop/pick-directory', apiToken, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ initial_dir: initialDir?.trim() || undefined }),
  });
  if (!res.ok) {
    const text = await res.text().catch(() => '');
    throw new Error(text || `HTTP ${res.status}`);
  }
  const json = (await res.json()) as { path?: unknown; cancelled?: unknown };
  return {
    path: typeof json.path === 'string' && json.path.trim() ? json.path : null,
    cancelled: Boolean(json.cancelled),
  };
};

/** Open or reveal a run artifact in the OS default app / file manager. */
export const openDesktopArtifact = async (
  apiToken: string,
  apiPath: string,
  mode: 'open' | 'reveal',
  tenantId?: string,
): Promise<void> => {
  const res = await apiFetch(
    '/v1/desktop/open-artifact',
    apiToken,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ api_path: apiPath, mode }),
    },
    tenantId,
  );
  if (!res.ok) {
    const text = await res.text().catch(() => '');
    throw new Error(text || `HTTP ${res.status}`);
  }
};

/** Resolve a gateway path like `/v1/runs` against the active API base URL. */
export const resolveApiUrl = (path: string): string => {
  if (/^https?:\/\//i.test(path)) {
    return path;
  }
  const base = getApiBaseUrl();
  if (!base) {
    return path;
  }
  return path.startsWith('/') ? `${base}${path}` : `${base}/${path}`;
};

/**
 * Attach the operator-supplied bearer token and `X-Yanshi-Tenant` header,
 * resolving relative gateway paths against `getApiBaseUrl()`.
 */
export const apiFetch = (
  path: string,
  token: string,
  init: RequestInit = {},
  tenant?: string,
): Promise<Response> => {
  const headers = new Headers(init.headers);
  const effectiveToken = resolveEffectiveApiToken(token);
  if (effectiveToken) {
    headers.set('Authorization', `Bearer ${effectiveToken}`);
  }
  const trimmedTenant = tenant?.trim();
  if (trimmedTenant) {
    headers.set('X-Yanshi-Tenant', trimmedTenant);
  }
  return fetch(resolveApiUrl(path), {
    ...init,
    headers,
  });
};

/**
 * Format an API error into a clear message; if it's a low-level browser
 * network failure (e.g. `Failed to fetch` when `yanshi serve` isn't running),
 * append actionable setup guidance.
 */
export const formatApiError = (err: unknown): string => {
  const raw = err instanceof Error ? err.message : String(err);
  if (
    /failed to fetch|networkerror|load failed|econnrefused/i.test(raw)
  ) {
    return `无法连接后端网关（${raw}）。请先启动 \`yanshi serve\`，或在 Studio 顶栏「网关配置」中填写服务地址（Tauri 桌面端会自动启动内置引擎）。`;
  }
  return raw;
};

/**
 * Generic SSE chunk parser used by workflow runs (`StreamedEventSchema`)
 * and harness sessions (`HarnessEventSchema`).
 */
export const parseSseChunk = <T,>(
  buffer: string,
  schema: import('zod').ZodType<T>,
  contextLabel: string,
): { events: T[]; rest: string } => {
  const events: T[] = [];
  let cursor = buffer;
  let boundary = cursor.indexOf('\n\n');
  while (boundary >= 0) {
    const raw = cursor.slice(0, boundary);
    cursor = cursor.slice(boundary + 2);
    const data = raw
      .split('\n')
      .filter((line) => line.startsWith('data:'))
      .map((line) => line.slice(5).trimStart())
      .join('\n');
    if (data) {
      events.push(parseJson(schema, JSON.parse(data), contextLabel));
    }
    boundary = cursor.indexOf('\n\n');
  }
  return { events, rest: cursor };
};
