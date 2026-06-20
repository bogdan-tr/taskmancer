export interface HoursMinutes {
  hours: number;
  minutes: number;
}

/** Combines `hours`/`minutes` into a single total-minutes value. Negative/non-finite inputs are clamped to zero. */
export function minutesFromHoursAndMinutes(hours: number, minutes: number): number {
  const safeHours = Number.isFinite(hours) ? hours : 0;
  const safeMinutes = Number.isFinite(minutes) ? minutes : 0;
  return Math.max(0, safeHours) * 60 + Math.max(0, safeMinutes);
}

/** Splits a total-minutes value into hours and a 0-59 minutes remainder. A negative/fractional/non-finite total is clamped/floored to zero. */
export function hoursAndMinutesFromMinutes(totalMinutes: number): HoursMinutes {
  const safeTotal = Number.isFinite(totalMinutes) ? Math.max(0, Math.floor(totalMinutes)) : 0;
  return { hours: Math.floor(safeTotal / 60), minutes: safeTotal % 60 };
}

/**
 * Normalizes an hrs/mins pair so minutes always lands in `0..59`, rolling
 * any excess into hours — e.g. `(0, 90)` becomes `{ hours: 1, minutes: 30 }`.
 * Used by the estimated-time input widget so typing 90 into "mins" reads
 * back as 1h 30m rather than displaying a raw 90.
 */
export function normalizeHoursMinutes(hours: number, minutes: number): HoursMinutes {
  return hoursAndMinutesFromMinutes(minutesFromHoursAndMinutes(hours, minutes));
}

/** Formats a total-minutes value as `"1h 30m"`/`"2h"`/`"45m"`/`"0m"` for display. */
export function formatMinutes(totalMinutes: number): string {
  const { hours, minutes } = hoursAndMinutesFromMinutes(totalMinutes);
  if (hours === 0 && minutes === 0) return "0m";

  const parts: string[] = [];
  if (hours > 0) parts.push(`${hours}h`);
  if (minutes > 0) parts.push(`${minutes}m`);
  return parts.join(" ");
}
