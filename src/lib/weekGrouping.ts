import type { Task } from "./types";

/** Whether a week-view bar represents a task's scheduled date or its due date. */
export type WeekBarType = "scheduled" | "due";

/** A single task occurrence rendered on one day of the week view. */
export interface WeekBar {
  task: Task;
  type: WeekBarType;
  date: string;
}

/**
 * Groups `tasks` into per-day bars for each ISO date in `weekDates`.
 *
 * A task appears once per matching `scheduled`/`due` date within the week —
 * a task with both falling in the week gets one bar for each. Within a day,
 * scheduled bars are listed before due bars, each preserving `tasks` order.
 */
export function groupTasksByWeek(tasks: Task[], weekDates: string[]): WeekBar[][] {
  return weekDates.map((date) => {
    const scheduledBars: WeekBar[] = tasks
      .filter((task) => task.scheduled === date)
      .map((task) => ({ task, type: "scheduled", date }));
    const dueBars: WeekBar[] = tasks
      .filter((task) => task.due === date)
      .map((task) => ({ task, type: "due", date }));
    return [...scheduledBars, ...dueBars];
  });
}
