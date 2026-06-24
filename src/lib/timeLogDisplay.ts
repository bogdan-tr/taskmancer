import { formatMinutes } from "./estimatedTime";

/**
 * Formats an RFC3339 timestamp (as stored in `TimeEntry.started_at`/
 * `ended_at`) as a readable local date/time, e.g. "Jun 15, 2026, 9:00 AM".
 * No existing helper in this codebase covers this: `dueDateDisplay.ts` and
 * `weekRange.ts` both work with bare `YYYY-MM-DD` date-only strings, never a
 * full timestamp with time-of-day. Returns `"Invalid date"` for an
 * unparseable input rather than throwing, mirroring this codebase's general
 * "fail soft in display code" style (see `estimatedTime.ts`'s clamping).
 */
export function formatEntryTimestamp(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "Invalid date";
  return date.toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

/**
 * Formats the duration between two RFC3339 timestamps as `"1h 30m"`-style
 * text via `estimatedTime.ts`'s `formatMinutes`, rounding to the nearest
 * whole minute — the same precision already used for `Task.tracked_minutes`
 * everywhere else in the UI (the card's "tracked" chip), so a completed
 * entry's duration reads consistently with the running total it contributes
 * to. Returns `"0m"` for an unparseable input or a non-positive duration
 * rather than a negative/NaN result.
 */
export function formatEntryDuration(startedAt: string, endedAt: string): string {
  const startMs = Date.parse(startedAt);
  const endMs = Date.parse(endedAt);
  if (Number.isNaN(startMs) || Number.isNaN(endMs) || endMs <= startMs) return formatMinutes(0);
  return formatMinutes(Math.round((endMs - startMs) / 60_000));
}

/**
 * Validates a manual time-entry's proposed start/end before calling the
 * backend, mirroring `add_manual_time_entry`'s own server-side rejection of
 * `ended_at <= started_at` — failing fast here gives a clear inline message
 * instead of a round-trip error for the most common mistake (picking an end
 * time before/equal to the start). Returns an error message, or `undefined`
 * when the range is valid.
 */
export function validateManualEntryRange(startedAt: string, endedAt: string): string | undefined {
  const startMs = Date.parse(startedAt);
  const endMs = Date.parse(endedAt);
  if (Number.isNaN(startMs)) return "Start date/time is invalid";
  if (Number.isNaN(endMs)) return "End date/time is invalid";
  if (endMs <= startMs) return "End must be after start";
  return undefined;
}

/**
 * Converts an RFC3339 timestamp to the value format `<input
 * type="datetime-local">` expects (`YYYY-MM-DDTHH:mm`, local time, no
 * timezone offset/seconds) — used to pre-fill the edit form from an existing
 * entry. Returns `""` for an unparseable input so the input simply renders
 * empty rather than throwing.
 */
export function isoToDatetimeLocalValue(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  const hours = String(date.getHours()).padStart(2, "0");
  const minutes = String(date.getMinutes()).padStart(2, "0");
  return `${year}-${month}-${day}T${hours}:${minutes}`;
}

/**
 * Converts a `<input type="datetime-local">` value (local time, no timezone
 * offset) back to an RFC3339 timestamp suitable for the backend commands
 * (`addManualTimeEntry`/`updateTimeEntry`), which all expect an
 * absolute, timezone-aware instant. Returns `undefined` for an empty or
 * unparseable input.
 */
export function datetimeLocalValueToIso(value: string): string | undefined {
  if (!value) return undefined;
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return undefined;
  return date.toISOString();
}
