import { newlyAllDoneTaskIds } from "./subtaskCompletion";
import type { Task } from "./types";

const PREVIOUSLY_ALL_DONE_KEY = "taskmancer:subtask-all-done-shown";
const PERMANENTLY_DISMISSED_KEY = "taskmancer:subtask-all-done-dismissed";

/** Reads a persisted set of task ids, tolerating a missing/corrupt entry (and an unavailable `localStorage`, e.g. during server-side prerendering) by falling back to an empty set. */
function loadPersistedIdSet(key: string): Set<string> {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return new Set();
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? new Set(parsed.filter((id): id is string => typeof id === "string")) : new Set();
  } catch {
    return new Set();
  }
}

function savePersistedIdSet(key: string, value: Set<string>): void {
  try {
    localStorage.setItem(key, JSON.stringify([...value]));
  } catch {
    // Persistence is best-effort; the in-memory value still applies for this session.
  }
}

/**
 * Module-level (not component-local) state, persisted to `localStorage` —
 * `KanbanBoard.svelte`'s component instance is destroyed and recreated on
 * every navigation between top-level routes, and the whole app restarts
 * between launches. Tracking "already shown for this completion" as
 * purely in-memory state would reset on either of those, re-triggering the
 * popup for a transition that already happened (the user-reported "I see
 * this dialog every time the app opens" bug) — exactly the repeat the
 * Subtasks design spec says must not happen. Deliberately plain (non-
 * `$state`) variables: nothing in the template reads these directly, only
 * the derived `allDoneQueueState` does, and wrapping them in `$state`
 * would make `checkAllDoneTransitions`' own read-then-write of
 * `previouslyAllDone` a self-triggering reactive dependency, looping
 * forever (`effect_update_depth_exceeded`) — see the function's own
 * history for why this exact mistake was already made and fixed once.
 */
let previouslyAllDone = loadPersistedIdSet(PREVIOUSLY_ALL_DONE_KEY);
/** Parent task ids the user has permanently dismissed via "Don't ask again" — never queued again, even after a later un-done-then-redone transition. */
let permanentlyDismissed = loadPersistedIdSet(PERMANENTLY_DISMISSED_KEY);

/** Task ids awaiting the all-done popup, oldest first — shown one at a time so two simultaneous completions don't stack dialogs. */
export const allDoneQueueState = $state<{ items: string[] }>({ items: [] });

/**
 * Re-checks every task with a subtask container against `tasks` and queues
 * the all-done popup for any newly-completed one that hasn't been
 * permanently dismissed. Called from `KanbanBoard.svelte`'s own `$effect`,
 * only while actively viewing that task's own subtask board (see its call
 * site for why — this used to run unconditionally and fire on whichever
 * unrelated view happened to have the parent task loaded).
 */
export function checkAllDoneTransitions(
  tasks: Task[],
  doneStatusId: string | undefined,
  cancelledStatusId: string | undefined,
  today: string,
): void {
  const { newlyDone, stillAllDone } = newlyAllDoneTaskIds(
    previouslyAllDone,
    tasks,
    doneStatusId,
    cancelledStatusId,
    today,
  );
  previouslyAllDone = stillAllDone;
  savePersistedIdSet(PREVIOUSLY_ALL_DONE_KEY, previouslyAllDone);

  const toQueue = newlyDone.filter((id) => !permanentlyDismissed.has(id));
  if (toQueue.length > 0) {
    const alreadyQueued = new Set(allDoneQueueState.items);
    allDoneQueueState.items = [...allDoneQueueState.items, ...toQueue.filter((id) => !alreadyQueued.has(id))];
  }
}

/** Dismisses the front of the queue — called on every dialog action (dismiss, mark done, delete subtasks, or permanent dismissal), not just an explicit "Not now". */
export function dequeueAllDone(): void {
  allDoneQueueState.items = allDoneQueueState.items.slice(1);
}

/** Permanently stops the all-done popup from ever firing again for `taskId` ("Don't ask again"), and dismisses it now. */
export function dismissAllDonePermanently(taskId: string): void {
  permanentlyDismissed = new Set(permanentlyDismissed).add(taskId);
  savePersistedIdSet(PERMANENTLY_DISMISSED_KEY, permanentlyDismissed);
  dequeueAllDone();
}
