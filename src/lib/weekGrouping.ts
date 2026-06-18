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

/**
 * Groups `tasks` into per-day bars for each ISO date in `weekDates`.
 *
 * A task appears once per matching `scheduled`/`due` date within the week —
 * a task with both falling in the week gets one bar for each. Within a day,
 * scheduled bars are listed before due bars, each sorted by priority (highest
 * first) then alphabetically by title (see `compareByPriorityThenTitle`).
 */
export function groupTasksByWeek(
  tasks: Task[],
  weekDates: string[],
  priorities: PriorityLevel[],
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
    return [...scheduledBars, ...dueBars];
  });
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
