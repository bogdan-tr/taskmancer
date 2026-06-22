import { listTasks } from "./api";
import type { Task } from "./types";

/**
 * Boxed in an object because Svelte 5 forbids exporting a reassigned `$state`
 * binding directly from a module — only its properties may be mutated.
 */
export const tasksState = $state<{ items: Task[] }>({ items: [] });

/**
 * Reloads the full task list from the backend — the source `Sidebar`/
 * `ProjectTreeNode` use to hide subtask container projects (via
 * `containerOwner`) and the project page uses for its breadcrumb override,
 * since neither otherwise loads tasks at all. On failure the previously
 * loaded tasks are left in place.
 */
export async function refreshTasks(): Promise<void> {
  try {
    tasksState.items = await listTasks();
  } catch {
    // Keep the previously loaded tasks.
  }
}
