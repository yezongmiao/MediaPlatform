import type { ThemeMode } from './types';

const STORAGE_KEY = 'mediadesk-theme';

export function readStoredTheme(): ThemeMode {
  const value = localStorage.getItem(STORAGE_KEY);
  return value === 'light' || value === 'dark' || value === 'system' ? value : 'system';
}

export function systemPrefersDark(): boolean {
  return window.matchMedia('(prefers-color-scheme: dark)').matches;
}

export function effectiveTheme(mode: ThemeMode): 'light' | 'dark' {
  return mode === 'system' ? (systemPrefersDark() ? 'dark' : 'light') : mode;
}

export function applyTheme(mode: ThemeMode): void {
  const effective = effectiveTheme(mode);
  document.documentElement.dataset.theme = effective;
  document.documentElement.style.colorScheme = effective;
  const themeColor = effective === 'dark' ? '#171512' : '#f2ede4';
  document.querySelector('meta[name="theme-color"]')?.setAttribute('content', themeColor);
}

export function persistTheme(mode: ThemeMode): void {
  localStorage.setItem(STORAGE_KEY, mode);
  applyTheme(mode);
}
