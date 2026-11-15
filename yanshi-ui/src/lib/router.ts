// Unified SPA client-side router supporting both:
// 1. Web gateway / Vite dev (`/ui/*` pathname via History API pushState)
// 2. Embedded Tauri desktop/mobile bundle (`/index.html` root via `#/ui/*` hash + memory state,
//    avoiding full-page asset 404s when navigating or reloading without a server rewrite)

import { useEffect, useState } from 'react';

export interface RouteLocation {
  pathname: string;
  search: string;
}

export interface NavigateOptions {
  replace?: boolean;
}

export interface LinkClickLike {
  button: number;
  metaKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
  defaultPrevented: boolean;
}

type RouteListener = () => void;

const listeners = new Set<RouteListener>();
let hostModeCache: 'history' | 'embedded' | null = null;
let embeddedRoute: RouteLocation = { pathname: '/ui/', search: '' };

/** Normalize a UI navigation target so bare `/ui` never hits Vite's `/ui/` base error. */
export const normalizeUiTarget = (to: string): string => {
  const trimmed = to.trim();
  if (!trimmed || trimmed === '/' || trimmed === '/ui') {
    return '/ui/';
  }
  if (trimmed.startsWith('?')) {
    return `/ui/${trimmed}`;
  }
  return trimmed.startsWith('/') ? trimmed : `/${trimmed}`;
};

/** Parse a target URL string into `{ pathname, search }`. */
export const parseTargetLocation = (to: string): RouteLocation => {
  const normalized = normalizeUiTarget(to);
  const hashIndex = normalized.indexOf('#');
  const withoutHash = hashIndex >= 0 ? normalized.slice(0, hashIndex) : normalized;
  const queryIndex = withoutHash.indexOf('?');
  if (queryIndex < 0) {
    return { pathname: withoutHash, search: '' };
  }
  return {
    pathname: withoutHash.slice(0, queryIndex) || '/ui/',
    search: withoutHash.slice(queryIndex),
  };
};

const detectHostMode = (): 'history' | 'embedded' => {
  if (hostModeCache !== null) {
    return hostModeCache;
  }
  if (typeof window === 'undefined') {
    return 'embedded';
  }
  const initialPath = window.location.pathname;
  hostModeCache =
    initialPath === '/ui' || initialPath.startsWith('/ui/') ? 'history' : 'embedded';
  return hostModeCache;
};

/** Read the active route across both `/ui/*` HTTP origins and embedded Tauri shells. */
export const getCurrentRoute = (): RouteLocation => {
  if (typeof window === 'undefined') {
    return embeddedRoute;
  }
  if (detectHostMode() === 'history') {
    return {
      pathname: window.location.pathname,
      search: window.location.search,
    };
  }
  const rawHash = window.location.hash;
  if (rawHash.startsWith('#/')) {
    return parseTargetLocation(rawHash.slice(1));
  }
  return embeddedRoute;
};

const notifyListeners = (): void => {
  for (const listener of listeners) {
    listener();
  }
};

/** Subscribe to client-side route transitions. Returns an unsubscribe callback. */
export const subscribeRouteChange = (listener: RouteListener): (() => void) => {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
};

/** Navigate within the SPA without triggering a full-page browser/WebView reload. */
export const navigateTo = (to: string, options: NavigateOptions = {}): void => {
  const normalized = normalizeUiTarget(to);
  const parsed = parseTargetLocation(normalized);
  embeddedRoute = parsed;

  if (typeof window !== 'undefined') {
    if (detectHostMode() === 'history') {
      if (options.replace) {
        window.history.replaceState(null, '', normalized);
      } else {
        window.history.pushState(null, '', normalized);
      }
    } else {
      const hashUrl = `#${normalized}`;
      try {
        if (options.replace) {
          window.history.replaceState(null, '', hashUrl);
        } else {
          window.history.pushState(null, '', hashUrl);
        }
      } catch {
        // Fallback if custom scheme rejects history state updates; `embeddedRoute` still drives UI.
      }
    }
  }

  notifyListeners();
};

/** Decide whether a mouse click on `<SpaLink>` should be handled by the SPA router. */
export const shouldInterceptLinkClick = (event: LinkClickLike, target?: string): boolean => {
  if (event.defaultPrevented) {
    return false;
  }
  if (event.button !== 0) {
    return false;
  }
  if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) {
    return false;
  }
  if (target && target !== '_self') {
    return false;
  }
  return true;
};

/** React hook returning `{ pathname, search }` and re-rendering on SPA or browser history navigation. */
export const useCurrentRoute = (): RouteLocation => {
  const [route, setRoute] = useState<RouteLocation>(() => getCurrentRoute());

  useEffect(() => {
    const sync = () => {
      const next = getCurrentRoute();
      setRoute((prev) =>
        prev.pathname === next.pathname && prev.search === next.search ? prev : next,
      );
    };
    const unsubscribe = subscribeRouteChange(sync);
    window.addEventListener('popstate', sync);
    window.addEventListener('hashchange', sync);
    return () => {
      unsubscribe();
      window.removeEventListener('popstate', sync);
      window.removeEventListener('hashchange', sync);
    };
  }, []);

  return route;
};

/** Reset cached host-mode detection between unit tests. */
export const resetRouterForTests = (): void => {
  hostModeCache = null;
  embeddedRoute = { pathname: '/ui/', search: '' };
  listeners.clear();
};
