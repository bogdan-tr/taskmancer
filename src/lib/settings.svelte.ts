import { getSettings, saveSettings } from "./api";
import type { Settings } from "./types";

/**
 * Boxed in an object because Svelte 5 forbids exporting a reassigned `$state`
 * binding directly from a module — only its properties may be mutated.
 *
 * `current` is `undefined` until the first successful [`refreshSettings`]
 * call.
 */
export const settingsState = $state<{ current: Settings | undefined }>({ current: undefined });

/**
 * Loads global settings from the backend. On failure the previously loaded
 * settings (if any) are left in place so the app keeps working with stale
 * data rather than losing its settings entirely.
 */
export async function refreshSettings(): Promise<void> {
  try {
    settingsState.current = await getSettings();
  } catch {
    // Keep the previously loaded settings.
  }
}

/**
 * Persists `next` as the new global settings and updates local state.
 *
 * Unlike `refreshSettings`, failures are not swallowed: this rejects so the
 * caller (a settings form) can show the user their save did not go through.
 */
export async function persistSettings(next: Settings): Promise<void> {
  settingsState.current = await saveSettings(next);
}
