import { useEffect, useState } from 'preact/hooks';

export type RouteName =
  | 'dashboard'
  | 'ideas'
  | 'projects'
  | 'project'
  | 'library'
  | 'settings'
  | 'notfound';

export interface Route {
  name: RouteName;
  id?: string;
  search: string;
  path: string;
}

const NAV_EVENT = 'mediadesk:navigate';

export function parseRoute(pathname: string, search = ''): Route {
  const path = pathname.replace(/\/+$/, '') || '/';
  const clean = path;
  if (clean === '/') return { name: 'dashboard', search, path: clean };
  if (clean === '/ideas') return { name: 'ideas', search, path: clean };
  if (clean === '/projects') return { name: 'projects', search, path: clean };
  if (clean === '/library') return { name: 'library', search, path: clean };
  if (clean === '/settings') return { name: 'settings', search, path: clean };
  const match = clean.match(/^\/projects\/([^/]+)$/);
  if (match) {
    return { name: 'project', id: decodeURIComponent(match[1]), search, path: clean };
  }
  return { name: 'notfound', search, path: clean };
}

export function useRoute(): Route {
  const [route, setRoute] = useState<Route>(() =>
    parseRoute(window.location.pathname, window.location.search),
  );
  useEffect(() => {
    const update = () =>
      setRoute(parseRoute(window.location.pathname, window.location.search));
    const onNavigate = () => update();
    window.addEventListener('popstate', update);
    window.addEventListener(NAV_EVENT, onNavigate);
    return () => {
      window.removeEventListener('popstate', update);
      window.removeEventListener(NAV_EVENT, onNavigate);
    };
  }, []);
  return route;
}

export function navigate(to: string, options: { replace?: boolean } = {}): void {
  const url = new URL(to, window.location.origin);
  if (options.replace) {
    window.history.replaceState({}, '', url);
  } else {
    window.history.pushState({}, '', url);
  }
  window.dispatchEvent(new Event(NAV_EVENT));
  window.scrollTo({ top: 0, behavior: 'instant' as ScrollBehavior });
}

export function useQueryParam(name: string): string | null {
  const [value, setValue] = useState(() => new URLSearchParams(window.location.search).get(name));
  useEffect(() => {
    const update = () => setValue(new URLSearchParams(window.location.search).get(name));
    window.addEventListener('popstate', update);
    window.addEventListener(NAV_EVENT, update);
    return () => {
      window.removeEventListener('popstate', update);
      window.removeEventListener(NAV_EVENT, update);
    };
  }, [name]);
  return value;
}
