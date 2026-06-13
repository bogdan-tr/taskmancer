import { DEFAULT_THEME, isTheme, type Theme } from "./theme";

const STORAGE_KEY = "taskmancer:theme";

/**
 * Boxed in an object because Svelte 5 forbids exporting a reassigned `$state`
 * binding directly from a module — only its properties may be mutated.
 */
export const themeState = $state<{ current: Theme }>({ current: DEFAULT_THEME });

/** Applies `next` as the active theme: updates reactive state, the DOM, and persisted storage. */
export function setTheme(next: Theme): void {
  themeState.current = next;
  document.documentElement.dataset.theme = next;
  try {
    localStorage.setItem(STORAGE_KEY, next);
  } catch {
    // Persistence is best-effort; the theme still applies for this session.
  }
}

/** Restores the previously persisted theme (or the default) and applies it. */
export function initTheme(): void {
  let stored: string | null = null;
  try {
    stored = localStorage.getItem(STORAGE_KEY);
  } catch {
    // Fall back to the default theme below.
  }
  setTheme(isTheme(stored) ? stored : DEFAULT_THEME);
}
