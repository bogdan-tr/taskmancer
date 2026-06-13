const STORAGE_KEY = "taskmancer:sidebar-collapsed";

/**
 * Boxed in an object because Svelte 5 forbids exporting a reassigned `$state`
 * binding directly from a module — only its properties may be mutated.
 */
export const sidebarState = $state<{ collapsed: boolean }>({ collapsed: false });

/** Sets whether the sidebar is collapsed and persists the choice. */
export function setSidebarCollapsed(collapsed: boolean): void {
  sidebarState.collapsed = collapsed;
  try {
    localStorage.setItem(STORAGE_KEY, collapsed ? "true" : "false");
  } catch {
    // Persistence is best-effort; the choice still applies for this session.
  }
}

/** Toggles the sidebar's collapsed state. */
export function toggleSidebar(): void {
  setSidebarCollapsed(!sidebarState.collapsed);
}

/** Restores the previously persisted collapsed state, defaulting to expanded. */
export function initSidebar(): void {
  let stored: string | null = null;
  try {
    stored = localStorage.getItem(STORAGE_KEY);
  } catch {
    // Fall back to the default (expanded) below.
  }
  sidebarState.collapsed = stored === "true";
}
