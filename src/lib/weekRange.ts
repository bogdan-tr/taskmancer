import type { WeekStartsOn } from "./displaySettings.svelte";

/** Formats a Date as a local "YYYY-MM-DD" string (no timezone conversion). */
export function formatDateISO(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

/** Returns a new Date offset by `days` (may be negative), preserving local time-of-day. */
export function addDays(date: Date, days: number): Date {
  const result = new Date(date);
  result.setDate(result.getDate() + days);
  return result;
}

/** Returns a new Date offset by `weeks` (may be negative). */
export function addWeeks(date: Date, weeks: number): Date {
  return addDays(date, weeks * 7);
}

/** Returns the first day of the week containing `date`, per `weekStartsOn`. */
export function startOfWeek(date: Date, weekStartsOn: WeekStartsOn): Date {
  const dayOfWeek = date.getDay();
  const offset = weekStartsOn === "monday" ? (dayOfWeek + 6) % 7 : dayOfWeek;
  return addDays(date, -offset);
}

/** Returns the 7 dates of the week starting at `weekStart`, in order. */
export function weekDates(weekStart: Date): Date[] {
  return Array.from({ length: 7 }, (_, index) => addDays(weekStart, index));
}
