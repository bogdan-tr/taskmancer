/**
 * How often a recurring task repeats — mirrors `RecurrenceFrequency` in
 * `src-tauri/src/series.rs` exactly (including its `kind`/field naming),
 * since this crosses the Tauri IPC boundary as-is, the same way `Task`'s
 * fields are shared verbatim between the Rust and TypeScript sides rather
 * than translated to a different naming convention.
 */
export type RecurrenceFrequency =
  | { kind: "EveryNDays"; interval: number }
  | { kind: "Weekly"; weekdays: number[]; interval_weeks: number }
  | { kind: "MonthlyByDay"; day: number };

/**
 * How far an edit/delete on a recurring task's occurrence should reach:
 * `"this"` affects only that occurrence (severing its link to the series,
 * for an edit); `"future"` also affects every other already-generated
 * occurrence on or after that one's date (and, for an edit, the series
 * template itself, so occurrences generated later inherit it too).
 */
export type SeriesEditScope = "this" | "future";

const WEEKDAY_LABELS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

function ordinal(day: number): string {
  if (day % 100 >= 11 && day % 100 <= 13) return `${day}th`;
  switch (day % 10) {
    case 1:
      return `${day}st`;
    case 2:
      return `${day}nd`;
    case 3:
      return `${day}rd`;
    default:
      return `${day}th`;
  }
}

/** Formats a `RecurrenceFrequency` for display, e.g. "Weekly on Mon, Wed, Fri", "Every other day", "Monthly on the 4th". */
export function formatRecurrenceFrequency(frequency: RecurrenceFrequency): string {
  switch (frequency.kind) {
    case "EveryNDays":
      if (frequency.interval === 1) return "Daily";
      if (frequency.interval === 2) return "Every other day";
      return `Every ${frequency.interval} days`;
    case "Weekly": {
      const days = [...frequency.weekdays].sort((a, b) => a - b);
      const isWeekday = days.length === 5 && days.join(",") === "1,2,3,4,5";
      const isWeekend = days.length === 2 && days.join(",") === "0,6";
      const dayList = isWeekday ? "weekdays" : isWeekend ? "weekends" : days.map((d) => WEEKDAY_LABELS[d]).join(", ");
      return frequency.interval_weeks === 1
        ? `Weekly on ${dayList}`
        : `Every ${frequency.interval_weeks} weeks on ${dayList}`;
    }
    case "MonthlyByDay":
      return `Monthly on the ${ordinal(frequency.day)}`;
  }
}
