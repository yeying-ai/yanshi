// HTTP / SSE helpers shared by Web, Tauri Desktop, and Tauri Mobile surfaces.

import { parseJson } from '../schemas';
import { apiBaseUrlKey, readStorage, writeStorage } from './storage';

declare global {
  interface Window {
    /** Injected by the Tauri Desktop shell when its embedded loopback engine boots. */
    __YANSHI_API_BASE__?: string;
    /** True when running inside a Tauri v2 WebView shell. */
    __TAURI_INTERNALS__?: unknown;
  }
}

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
  const trimmed = token.trim();
  if (trimmed) {
    headers.set('Authorization', `Bearer ${trimmed}`);
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
