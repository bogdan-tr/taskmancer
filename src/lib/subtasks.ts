import { MAX_SUGGESTIONS } from "./autocomplete";
import { isVisibleOnBoard } from "./boardVisibility";
import type { Task } from "./types";

/**
 * Whether `task` is itself a subtask — i.e. some other task's
 * `subtask_project_id` names `task`'s own `project_id`. Guards against a
 * task with no `project_id` at all (shouldn't happen for any real task —
 * see `Task.project_id`'s own doc comment — but `undefined === undefined`
 * would otherwise match every other container-less task, a false
 * positive worth defending against explicitly).
 */
export function isSubtask(task: Task, allTasks: Task[]): boolean {
  if (!task.project_id) return false;
  return allTasks.some((t) => t.subtask_project_id === task.project_id);
}

/**
 * The task that owns the subtask container identified by `projectId`, if
 * any — the reverse lookup this app uses instead of a back-pointer stored
 * on the container project itself (see the Subtasks design spec's "no
 * back-pointer" decision). Used to hide a container from the sidebar
 * project tree and to scope the all-done popup/header button to its own
 * board.
 */
export function containerOwner(projectId: string, tasks: Task[]): Task | undefined {
  return tasks.find((t) => t.subtask_project_id === projectId);
}

/** Every subtask of `parentTask` — the tasks filed under its subtask container. Empty if it has no container yet. */
export function subtasksOf(parentTask: Task, allTasks: Task[]): Task[] {
  if (!parentTask.subtask_project_id) return [];
  return allTasks.filter((t) => t.project_id === parentTask.subtask_project_id);
}

/**
 * `parentTask`'s subtasks, collapsed to one entry per ongoing concern: a
 * one-off subtask passes through unchanged, but a *recurring* subtask's
 * container holds every occurrence its own `Series` has pre-generated via
 * lookahead (potentially dozens, spanning months) — grouped here by
 * `series_id` and reduced to the single occurrence that actually matters
 * right now, mirroring `isVisibleOnBoard`'s "is this on the board today"
 * rule. Within a series group, that's the latest occurrence that's
 * actually due (`scheduled` on or before `today`) when one exists —
 * picking the latest rather than the earliest matters when a previous
 * day's occurrence is still sitting un-finished — and otherwise the
 * earliest upcoming one, so a recurring subtask always shows as exactly
 * one card/row instead of every generated future instance. Used wherever
 * subtasks are shown or counted as "current work" (card previews, the
 * progress badge, the all-done check) — never for cascading deletes,
 * which must still reach every generated occurrence (see `subtasksOf`).
 */
export function relevantSubtasksOf(parentTask: Task, allTasks: Task[], today: string): Task[] {
  const all = subtasksOf(parentTask, allTasks);
  const bySeries = new Map<string, Task[]>();
  const standalone: Task[] = [];

  for (const task of all) {
    if (!task.series_id) {
      standalone.push(task);
      continue;
    }
    const group = bySeries.get(task.series_id) ?? [];
    group.push(task);
    bySeries.set(task.series_id, group);
  }

  const picked = [...standalone];
  for (const group of bySeries.values()) {
    const due = group.filter((task) => isVisibleOnBoard(task, today));
    if (due.length > 0) {
      picked.push(due.reduce((latest, task) => ((task.scheduled ?? "") > (latest.scheduled ?? "") ? task : latest)));
    } else {
      picked.push(
        group.reduce((earliest, task) => ((task.scheduled ?? "") < (earliest.scheduled ?? "") ? task : earliest)),
      );
    }
  }

  return picked;
}

/**
 * How many of `parentTask`'s subtasks are done, out of how many count at
 * all — both counts exclude cancelled subtasks, so a cancelled item
 * neither reads as completed work nor inflates the denominator with
 * abandoned work. Operates on `relevantSubtasksOf` (see its own doc
 * comment), so a recurring subtask counts once for its currently due
 * occurrence, not once per every occurrence ever generated.
 */
export function subtaskProgress(
  parentTask: Task,
  allTasks: Task[],
  doneStatusId: string | undefined,
  cancelledStatusId: string | undefined,
  today: string,
): { done: number; total: number } {
  const active = relevantSubtasksOf(parentTask, allTasks, today).filter((t) => t.status !== cancelledStatusId);
  return { done: active.filter((t) => t.status === doneStatusId).length, total: active.length };
}

/**
 * Whether every one of `parentTask`'s non-cancelled subtasks is done —
 * requires at least one to exist, so a task with zero active subtasks
 * (none yet, or all of them cancelled) is never vacuously "done"; that
 * would misleadingly trigger the all-done popup for work that was never
 * actually completed.
 */
export function allSubtasksDone(
  parentTask: Task,
  allTasks: Task[],
  doneStatusId: string | undefined,
  cancelledStatusId: string | undefined,
  today: string,
): boolean {
  const { done, total } = subtaskProgress(parentTask, allTasks, doneStatusId, cancelledStatusId, today);
  return total > 0 && done === total;
}

/**
 * `parentTask`'s effective estimated time in minutes, rolling up its
 * subtasks' estimates — a live display computation only, never written
 * back to any stored field (see `Settings.parent_estimate_includes_own_value`'s
 * own doc comment). Falls back to `parentTask.estimated_minutes` unchanged
 * when it has no subtasks at all, so a plain task's estimate is never
 * affected by this. Sums `relevantSubtasksOf` (one entry per recurring
 * series, see its own doc comment) excluding cancelled subtasks — a
 * cancelled subtask's estimate no more counts toward the total than its
 * completion counts toward `subtaskProgress`. `includeOwnEstimate` adds
 * `parentTask`'s own estimate on top of that sum instead of being replaced
 * by it. Returns `undefined` (no chip to show) rather than `0` when
 * nothing anywhere actually has an estimate set, so adding subtasks to a
 * task that never had a time estimate doesn't conjure up a misleading
 * "0m" badge.
 */
export function effectiveEstimatedMinutes(
  parentTask: Task,
  allTasks: Task[],
  today: string,
  cancelledStatusId: string | undefined,
  includeOwnEstimate: boolean,
): number | undefined {
  const subtasks = relevantSubtasksOf(parentTask, allTasks, today).filter((t) => t.status !== cancelledStatusId);
  if (subtasks.length === 0) return parentTask.estimated_minutes;

  const hasAnyEstimate =
    subtasks.some((t) => t.estimated_minutes !== undefined) ||
    (includeOwnEstimate && parentTask.estimated_minutes !== undefined);
  if (!hasAnyEstimate) return undefined;

  const subtaskSum = subtasks.reduce((sum, t) => sum + (t.estimated_minutes ?? 0), 0);
  return includeOwnEstimate ? subtaskSum + (parentTask.estimated_minutes ?? 0) : subtaskSum;
}

/**
 * Ready-to-insert `sub <name>` autocomplete suggestions for `typedText` —
 * active (non-done, non-cancelled), non-subtask task titles starting with
 * it, quoted when multi-word (mirroring `projectPathSuggestions`'s own
 * insertion-quoting, for the identical reason: a quick-add token can't
 * round-trip a bare multi-word value). No disambiguation logic is needed
 * here unlike the project-path case — `sub` keeps simple first-match-by-
 * name resolution per the Subtasks design spec, so two tasks sharing a
 * title both just suggest the same (quoted, if needed) text. Empty for an
 * empty `typedText`, mirroring `filterSuggestions`'s own behavior.
 */
export function subtaskNameSuggestions(
  tasks: Task[],
  typedText: string,
  doneStatusId: string | undefined,
  cancelledStatusId: string | undefined,
): string[] {
  if (typedText === "") return [];

  const lowerTyped = typedText.toLowerCase();
  const candidates = tasks.filter(
    (t) =>
      !t.hidden &&
      t.status !== doneStatusId &&
      t.status !== cancelledStatusId &&
      !isSubtask(t, tasks) &&
      t.title.toLowerCase().startsWith(lowerTyped),
  );

  const inserts = candidates.map((t) => (/\s/.test(t.title) ? `"${t.title}"` : t.title));
  return [...new Set(inserts)].sort((a, b) => a.localeCompare(b)).slice(0, MAX_SUGGESTIONS);
}
