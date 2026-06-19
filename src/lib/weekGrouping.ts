import { priorityRank } from "./priorities.svelte";
import type { PriorityLevel, Task } from "./types";

/** Whether a week-view bar represents a task's scheduled date or its due date. */
export type WeekBarType = "scheduled" | "due";

/** A single task occurrence rendered on one day of the week view. */
export interface WeekBar {
  task: Task;
  type: WeekBarType;
  date: string;
}

/**
 * Compares two tasks for week-view bar ordering: by priority rank ascending
 * (rank 1 first), then alphabetically by title (case-insensitive). Tasks
 * whose `priority` doesn't match any defined level sort after all recognized
 * priorities.
 */
export function compareByPriorityThenTitle(a: Task, b: Task, priorities: PriorityLevel[]): number {
  const rankA = priorityRank(priorities, a.priority);
  const rankB = priorityRank(priorities, b.priority);
  if (rankA !== rankB) return rankA - rankB;
  return a.title.localeCompare(b.title, undefined, { sensitivity: "base" });
}

/** Returns `true` if `task`'s status is the configured done status or (if set) the cancelled status. */
export function isTaskFinished(
  task: Task,
  doneStatus: string,
  cancelledStatus: string | undefined,
): boolean {
  return task.status === doneStatus || (cancelledStatus !== undefined && task.status === cancelledStatus);
}

/**
 * Groups `tasks` into per-day bars for each ISO date in `weekDates`.
 *
 * A task appears once per matching `scheduled`/`due` date within the week —
 * a task with both falling in the week gets one bar for each. Within a day,
 * scheduled bars are listed before due bars, each sorted by priority (highest
 * first) then alphabetically by title (see `compareByPriorityThenTitle`) —
 * except that finished tasks (see `isTaskFinished`) sink to the very bottom
 * of the day's list (a stable sort, so their relative order among
 * themselves, and among the unfinished bars above them, is unchanged), so a
 * day's column stays focused on what's still actionable.
 */
export function groupTasksByWeek(
  tasks: Task[],
  weekDates: string[],
  priorities: PriorityLevel[],
  doneStatus: string,
  cancelledStatus: string | undefined,
): WeekBar[][] {
  return weekDates.map((date) => {
    const scheduledBars: WeekBar[] = tasks
      .filter((task) => task.scheduled === date)
      .sort((a, b) => compareByPriorityThenTitle(a, b, priorities))
      .map((task) => ({ task, type: "scheduled", date }));
    const dueBars: WeekBar[] = tasks
      .filter((task) => task.due === date)
      .sort((a, b) => compareByPriorityThenTitle(a, b, priorities))
      .map((task) => ({ task, type: "due", date }));
    return [...scheduledBars, ...dueBars].sort((a, b) => {
      const aFinished = isTaskFinished(a.task, doneStatus, cancelledStatus) ? 1 : 0;
      const bFinished = isTaskFinished(b.task, doneStatus, cancelledStatus) ? 1 : 0;
      return aFinished - bFinished;
    });
  });
}

/**
 * Removes one bar for any finished task (see `isTaskFinished`) that has both
 * a scheduled bar and a due bar somewhere across `weekColumns` — keeping
 * only the bar matching `keep` and dropping the other, so a finished task
 * with both dates in the visible week doesn't show up twice. A task with
 * only one of the two dates (or one outside the visible week, so only one
 * bar was ever created) is untouched, since there's nothing to deduplicate.
 * Operates across the whole week (not per-day) because the two bars for the
 * same task can land on different days.
 */
export function dedupeFinishedTaskBars(
  weekColumns: WeekBar[][],
  doneStatus: string,
  cancelledStatus: string | undefined,
  keep: WeekBarType,
): WeekBar[][] {
  const finishedTaskTypes = new Map<string, Set<WeekBarType>>();
  for (const dayBars of weekColumns) {
    for (const bar of dayBars) {
      if (!isTaskFinished(bar.task, doneStatus, cancelledStatus)) continue;
      const types = finishedTaskTypes.get(bar.task.id) ?? new Set<WeekBarType>();
      types.add(bar.type);
      finishedTaskTypes.set(bar.task.id, types);
    }
  }

  const removeType: WeekBarType = keep === "due" ? "scheduled" : "due";
  return weekColumns.map((dayBars) =>
    dayBars.filter((bar) => {
      const types = finishedTaskTypes.get(bar.task.id);
      const hasBothTypes = types !== undefined && types.has("scheduled") && types.has("due");
      return !(hasBothTypes && bar.type === removeType);
    }),
  );
}

/**
 * Groups unfinished tasks whose `scheduled` and/or `due` date is strictly
 * before `weekStartDate` into bars, for the optional "previous weeks"
 * column. Mirrors `groupTasksByWeek`'s one-bar-per-matching-date behavior,
 * but across all of history rather than a fixed 7-day window, and excludes
 * finished tasks (see `isTaskFinished`) since a completed task scheduled
 * last week isn't something that still needs attention. Sorted oldest date
 * first, then by priority/title (see `compareByPriorityThenTitle`).
 */
export function groupPreviousWeeksBars(
  tasks: Task[],
  weekStartDate: string,
  priorities: PriorityLevel[],
  doneStatus: string,
  cancelledStatus: string | undefined,
): WeekBar[] {
  const bars: WeekBar[] = [];
  for (const task of tasks) {
    if (isTaskFinished(task, doneStatus, cancelledStatus)) continue;
    if (task.scheduled && task.scheduled < weekStartDate) {
      bars.push({ task, type: "scheduled", date: task.scheduled });
    }
    if (task.due && task.due < weekStartDate) {
      bars.push({ task, type: "due", date: task.due });
    }
  }
  return bars.sort((a, b) => {
    if (a.date !== b.date) return a.date < b.date ? -1 : 1;
    return compareByPriorityThenTitle(a.task, b.task, priorities);
  });
}

/**
 * Counts distinct unfinished tasks (see `isTaskFinished`) whose `scheduled`
 * and/or `due` date is strictly before `weekStartDate` — i.e. tasks that
 * exist but aren't visible anywhere in the currently-displayed week. Used
 * for the week view header's "N tasks behind this week" indicator.
 */
export function countTasksBeforeWeek(
  tasks: Task[],
  weekStartDate: string,
  doneStatus: string,
  cancelledStatus: string | undefined,
): number {
  return tasks.filter((task) => {
    if (isTaskFinished(task, doneStatus, cancelledStatus)) return false;
    const scheduledBefore = task.scheduled !== undefined && task.scheduled < weekStartDate;
    const dueBefore = task.due !== undefined && task.due < weekStartDate;
    return scheduledBefore || dueBefore;
  }).length;
}
