const STORAGE_KEY = "taskmancer:project-tree-expanded";

/** Boxed in an object because Svelte 5 forbids exporting a reassigned `$state` binding directly from a module — only its properties may be mutated. */
export const projectTreeState = $state<{ expanded: Record<string, boolean> }>({ expanded: {} });

function persist(): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(projectTreeState.expanded));
  } catch {
    // Persistence is best-effort; the choice still applies for this session.
  }
}

/** Returns whether `projectId`'s subproject list should render expanded. Defaults to collapsed for a project never explicitly toggled. */
export function isExpanded(projectId: string): boolean {
  return projectTreeState.expanded[projectId] === true;
}

/** Sets whether `projectId`'s subproject list is expanded, persisting the choice. */
export function setExpanded(projectId: string, expanded: boolean): void {
  projectTreeState.expanded = { ...projectTreeState.expanded, [projectId]: expanded };
  persist();
}

export function toggleExpanded(projectId: string): void {
  setExpanded(projectId, !isExpanded(projectId));
}

/**
 * Marks `projectId` expanded only if it has no explicit recorded
 * preference yet — called the moment a project gains its first subproject,
 * so a newly created subproject is immediately visible without a manual
 * toggle, while never overriding a preference the user already set
 * (including an explicit collapse).
 */
export function expandIfUnset(projectId: string): void {
  if (projectId in projectTreeState.expanded) return;
  setExpanded(projectId, true);
}

/** Restores previously persisted expand state, falling back to all-collapsed. */
export function initProjectTree(): void {
  let stored: string | null = null;
  try {
    stored = localStorage.getItem(STORAGE_KEY);
  } catch {
    return;
  }
  if (!stored) return;

  try {
    const parsed = JSON.parse(stored);
    if (parsed && typeof parsed === "object") {
      projectTreeState.expanded = parsed;
    }
  } catch {
    // Fall back to the default (all collapsed).
  }
}
