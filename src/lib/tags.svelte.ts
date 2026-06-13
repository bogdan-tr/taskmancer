import { listTasks } from "./api";

/**
 * Boxed in an object because Svelte 5 forbids exporting a reassigned `$state`
 * binding directly from a module — only its properties may be mutated.
 */
export const tagsState = $state<{ items: string[] }>({ items: [] });

/**
 * Reloads the set of distinct tags in use across all tasks, for tag
 * autocomplete. On failure the previously loaded tags are left in place.
 */
export async function refreshTags(): Promise<void> {
  try {
    const tasks = await listTasks();
    const unique = new Set<string>();
    for (const task of tasks) {
      for (const tag of task.tags) {
        unique.add(tag);
      }
    }
    tagsState.items = [...unique].sort((a, b) => a.localeCompare(b));
  } catch {
    // Keep the previously loaded tags.
  }
}
