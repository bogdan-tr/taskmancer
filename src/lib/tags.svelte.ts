import { refreshTasks, tasksState } from "./tasks.svelte";

/**
 * Boxed in an object because Svelte 5 forbids exporting a reassigned `$state`
 * binding directly from a module — only its properties may be mutated.
 */
export const tagsState = $state<{ items: string[] }>({ items: [] });

/**
 * Reloads the set of distinct tags in use across all tasks, for tag
 * autocomplete. Delegates the actual fetch to `refreshTasks` (rather than
 * calling `listTasks` itself) so every existing call site of this function
 * also keeps the shared `tasksState` fresh, for free. On failure the
 * previously loaded tags are left in place, mirroring `refreshTasks`' own
 * failure behavior.
 */
export async function refreshTags(): Promise<void> {
  await refreshTasks();
  const unique = new Set<string>();
  for (const task of tasksState.items) {
    for (const tag of task.tags) {
      unique.add(tag);
    }
  }
  tagsState.items = [...unique].sort((a, b) => a.localeCompare(b));
}
