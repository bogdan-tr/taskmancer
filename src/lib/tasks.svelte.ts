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
 * `containerOwner`), since they otherwise load no tasks at all. On failure
 * the previously loaded tasks are left in place.
 */
export async function refreshTasks(): Promise<void> {
  try {
    tasksState.items = await listTasks();
  } catch {
    // Keep the previously loaded tasks.
  }
}

/**
 * Upserts `task` into the global cache in place. `KanbanBoard.svelte` is
 * the sole place every task mutation (create/update) flows through, but it
 * only ever patched its own board-scoped `visibleTasks` directly — this
 * cache is a *different* reactive source, read by every other view's
 * subtask-relationship lookups (`relevantSubtasksOf`, `containerOwner`,
 * etc. — see those call sites' own doc comments for why they must read the
 * global list, never a board-scoped one). Before this existed, the only
 * thing keeping this cache eventually-fresh after an edit was the
 * fire-and-forget `refreshTags`-via-`refreshTasks` call already made
 * alongside every mutation — technically correct once that promise
 * resolved, but an indirect, async round-trip for what should be an
 * immediate, synchronous update (e.g. a subtask's status dot on its
 * *parent's* card, which reads this exact cache, visibly lagging behind a
 * change made through it).
 */
export function upsertCachedTask(task: Task): void {
  const index = tasksState.items.findIndex((t) => t.id === task.id);
  tasksState.items =
    index === -1 ? [...tasksState.items, task] : tasksState.items.map((t) => (t.id === task.id ? task : t));
}

/** Removes a task from the global cache in place — the deletion counterpart to `upsertCachedTask`. Only call this for an actual deletion, never for a task that merely no longer matches some board's own filter (it still exists; only `upsertCachedTask` applies). */
export function removeCachedTask(id: string): void {
  tasksState.items = tasksState.items.filter((t) => t.id !== id);
}
