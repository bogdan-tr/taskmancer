const CONFIRM_TASK_DELETION_KEY = "taskmancer:confirm-task-deletion";

export const DEFAULT_CONFIRM_TASK_DELETION = true;

/**
 * Boxed in an object because Svelte 5 forbids exporting a reassigned `$state`
 * binding directly from a module — only its properties may be mutated.
 */
export const generalState = $state<{
  confirmTaskDeletion: boolean;
}>({
  confirmTaskDeletion: DEFAULT_CONFIRM_TASK_DELETION,
});

/** Sets whether deleting a task shows a confirmation dialog first, and persists it. */
export function setConfirmTaskDeletion(value: boolean): void {
  generalState.confirmTaskDeletion = value;
  try {
    localStorage.setItem(CONFIRM_TASK_DELETION_KEY, value ? "true" : "false");
  } catch {
    // Persistence is best-effort; the choice still applies for this session.
  }
}

/** Restores previously persisted general settings, falling back to defaults. */
export function initGeneral(): void {
  let stored: string | null = null;
  try {
    stored = localStorage.getItem(CONFIRM_TASK_DELETION_KEY);
  } catch {
    // Fall back to the default below.
  }

  setConfirmTaskDeletion(stored === null ? DEFAULT_CONFIRM_TASK_DELETION : stored === "true");
}
