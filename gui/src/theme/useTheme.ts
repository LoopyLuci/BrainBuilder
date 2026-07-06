import { useEffect, useState } from 'react';

export type ThemeChoice = 'light' | 'dark';
const STORAGE_KEY = 'brainbuilder.theme';

function systemPrefersDark(): boolean {
  return window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches;
}

function applyTheme(theme: ThemeChoice | null) {
  if (theme) {
    document.documentElement.setAttribute('data-theme', theme);
  } else {
    document.documentElement.removeAttribute('data-theme');
  }
}

// Manual choice (persisted) overrides system preference; with no manual
// choice, theme.css's own `prefers-color-scheme` media query takes over —
// this hook only needs to track/report which one is currently effective.
export function useTheme(): { theme: ThemeChoice; toggle: () => void } {
  const [theme, setTheme] = useState<ThemeChoice>(() => {
    const stored = localStorage.getItem(STORAGE_KEY) as ThemeChoice | null;
    return stored ?? (systemPrefersDark() ? 'dark' : 'light');
  });

  useEffect(() => {
    const stored = localStorage.getItem(STORAGE_KEY) as ThemeChoice | null;
    applyTheme(stored);
  }, []);

  const toggle = () => {
    const next: ThemeChoice = theme === 'dark' ? 'light' : 'dark';
    setTheme(next);
    localStorage.setItem(STORAGE_KEY, next);
    applyTheme(next);
  };

  return { theme, toggle };
}
