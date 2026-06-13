import { listProjects } from "./api";
import type { Project } from "./types";

/**
 * Boxed in an object because Svelte 5 forbids exporting a reassigned `$state`
 * binding directly from a module — only its properties may be mutated.
 */
export const projectsState = $state<{ items: Project[] }>({ items: [] });

/**
 * Reloads the project list from the backend. On failure the previously
 * loaded projects are left in place so the sidebar keeps working with
 * stale data rather than going blank.
 */
export async function refreshProjects(): Promise<void> {
  try {
    projectsState.items = await listProjects();
  } catch {
    // Keep the previously loaded projects.
  }
}
