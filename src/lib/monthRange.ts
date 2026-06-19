import type { WeekStartsOn } from "./displaySettings.svelte";
import { addDays, startOfWeek } from "./weekRange";

/** Returns the 1st day of the month containing `date`. */
export function startOfMonth(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), 1);
}

/**
 * Returns a new Date offset by `months` (may be negative). Intended for
 * day-1-anchored dates (e.g. `startOfMonth`'s output): the `Date`
 * constructor normalizes an out-of-range month into a year rollover, so
 * there's no risk of the day-of-month overflow that a naive
 * `tomorrow + 1 month` calculation can hit (e.g. Jan 31 has no Feb 31) —
 * every month has a 1st.
 */
export function addMonths(date: Date, months: number): Date {
  return new Date(date.getFullYear(), date.getMonth() + months, date.getDate());
}

/**
 * Returns every date needed to render `monthStart`'s month as a grid of
 * complete weeks: the month's own days, plus enough leading days from the
 * previous month and trailing days from the next to fill out the first and
 * last week per `weekStartsOn`. Always a multiple of 7 (28-42, depending on
 * the month's length and how it aligns with the configured week start).
 */
export function monthDates(monthStart: Date, weekStartsOn: WeekStartsOn): Date[] {
  const lastOfMonth = new Date(monthStart.getFullYear(), monthStart.getMonth() + 1, 0);
  const gridStart = startOfWeek(monthStart, weekStartsOn);
  const gridEnd = addDays(startOfWeek(lastOfMonth, weekStartsOn), 6);

  const dates: Date[] = [];
  for (let date = gridStart; date.getTime() <= gridEnd.getTime(); date = addDays(date, 1)) {
    dates.push(date);
  }
  return dates;
}
