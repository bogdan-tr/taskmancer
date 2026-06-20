import { dueRelativeDateLabel, resolveDueRelativeDate } from "./relativeDates";
import { addDays, formatDateISO } from "./weekRange";

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

/**
 * How a recurring task's due date relates to each occurrence's own
 * scheduled date — mirrors `DueRule` in `src-tauri/src/series.rs` exactly
 * (including its `kind` naming), since this crosses the Tauri IPC boundary
 * as-is. Never a single fixed absolute date, since that wouldn't make
 * sense once there's more than one occurrence — see `naturalLanguage.ts`'s
 * `tryResolveRecurringDuePhrase` for how a typed due phrase becomes one of
 * these.
 */
export type DueRule =
  | { kind: "Never" }
  | { kind: "DefaultCode"; code: string }
  | { kind: "AfterScheduled"; days: number }
  | { kind: "Weekday"; weekday: number; interval_weeks: number };

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

/** Formats a `DueRule` for display, e.g. "Same day as each occurrence", "3 days after each occurrence", "Every Monday", "Every other Friday". */
export function formatDueRule(rule: DueRule): string {
  switch (rule.kind) {
    case "Never":
      return "Never";
    case "DefaultCode":
      return dueRelativeDateLabel(rule.code);
    case "AfterScheduled":
      // "after each occurrence", not "after scheduled" — the latter reads
      // like a one-time calculation against a single date, when this rule
      // actually recomputes fresh against every occurrence's own scheduled
      // date (the original source of user confusion this wording avoids).
      if (rule.days === 0) return "Same day as each occurrence";
      if (rule.days > 0)
        return rule.days === 1 ? "1 day after each occurrence" : `${rule.days} days after each occurrence`;
      return rule.days === -1 ? "1 day before each occurrence" : `${-rule.days} days before each occurrence`;
    case "Weekday": {
      // "Every <day>", not "Next <day>" — this is a per-occurrence rule
      // that resolves afresh each time, not a single upcoming date, and
      // "Next Friday" reads like the latter (the original source of user
      // confusion this wording exists to avoid).
      const day = WEEKDAY_LABELS[rule.weekday];
      return rule.interval_weeks <= 1 ? `Every ${day}` : `Every other ${day}`;
    }
  }
}

function parseISODate(iso: string): Date {
  const [year, month, day] = iso.split("-").map(Number);
  return new Date(year, month - 1, day);
}

/**
 * Resolves `rule` to an absolute `YYYY-MM-DD` due date for an occurrence
 * scheduled on `scheduledIso`, mirroring `resolve_due_rule` in
 * `src-tauri/src/recurrence.rs` exactly. Returns `undefined` for "never
 * due" (`DueRule.Never`, or `DefaultCode`'s `"none"` sentinel/an
 * unrecognized code, both already handled by `resolveDueRelativeDate`).
 * Used both for the add-task preview (any task, recurring or not — see
 * `resolveTaskPreview`) and, for a recurring task, to derive what to send
 * `createRecurringTask` from a normal due phrase that resolved to a single
 * absolute date — see `daysBetween` below.
 */
export function resolveDueRule(rule: DueRule, scheduledIso: string): string | undefined {
  switch (rule.kind) {
    case "Never":
      return undefined;
    case "DefaultCode":
      return resolveDueRelativeDate(rule.code, scheduledIso);
    case "AfterScheduled":
      return formatDateISO(addDays(parseISODate(scheduledIso), rule.days));
    case "Weekday": {
      const scheduled = parseISODate(scheduledIso);
      const daysUntilNext = ((rule.weekday - scheduled.getDay()) % 7 + 7) % 7;
      const next = addDays(scheduled, daysUntilNext);
      const extraWeeks = Math.max(rule.interval_weeks - 1, 0);
      return formatDateISO(addDays(next, extraWeeks * 7));
    }
  }
}

/**
 * The integer number of days from `fromIso` to `toIso` (both `YYYY-MM-DD`),
 * positive if `toIso` is later. Used to convert an absolute due date a
 * normal NL phrase resolved to (e.g. "tomorrow", a weekday, an absolute
 * date) into a `DueRule.AfterScheduled` offset for a recurring task — see
 * `naturalLanguage.ts`'s `tryResolveRecurringDuePhrase`.
 */
export function daysBetween(fromIso: string, toIso: string): number {
  const [fromYear, fromMonth, fromDay] = fromIso.split("-").map(Number);
  const [toYear, toMonth, toDay] = toIso.split("-").map(Number);
  const from = new Date(fromYear, fromMonth - 1, fromDay);
  const to = new Date(toYear, toMonth - 1, toDay);
  const millisecondsPerDay = 24 * 60 * 60 * 1000;
  return Math.round((to.getTime() - from.getTime()) / millisecondsPerDay);
}

/**
 * The `DueRule` to send `createRecurringTask` for the series template,
 * derived from however the typed due phrase resolved — this is what fixes
 * the original bug where the first occurrence used whatever `due` resolved
 * to while every later occurrence silently fell back to an unrelated
 * default. `due`/`dueRule` are `ParsedTaskInput.due`/`.dueRule` (with any
 * manual due-picker override already merged in by the caller), and
 * `scheduledIso` is the task's fully resolved scheduled date
 * (`TaskPreview.scheduledDate`).
 *
 * - `dueRule` already set (`in <n> days`, a weekday-rule phrase, or `due
 *   na`'s `Never`) — used as-is, no inference needed.
 * - `due === "none"` (typed `due na`, or the date picker's "Never" button,
 *   neither of which touch `dueRule`) — `{ kind: "Never" }`.
 * - `due` set to a resolved absolute date (a normal phrase like "today" or
 *   a weekday, or a manual due-picker date) — the *gap* to `scheduledIso`
 *   becomes a generic `AfterScheduled` offset, applied to every occurrence.
 * - Neither set (no due token typed at all) — `undefined`, so the backend
 *   applies the configured project/global default itself, same as it
 *   already does for every other unset field.
 */
export function resolveSeriesDueRule(
  due: string | undefined,
  dueRule: DueRule | undefined,
  scheduledIso: string,
): DueRule | undefined {
  if (dueRule) return dueRule;
  if (due === "none") return { kind: "Never" };
  if (due !== undefined) return { kind: "AfterScheduled", days: daysBetween(scheduledIso, due) };
  return undefined;
}

/**
 * The absolute `due` date (or `"none"`) to submit for a *non-recurring*
 * task. `createTask` only ever reads `due`, never `dueRule` — there's no
 * series to apply a rule to, just one task — so a `due in <n> days`/
 * weekday-rule phrase (which resolves to `dueRule`, not `due`, since
 * neither can resolve to an absolute date during parsing alone — see
 * `ParsedTaskInput.dueRule`'s own doc comment) must be collapsed to an
 * absolute date here, or it would be silently dropped despite the preview
 * showing a real due date.
 */
export function resolveNonRecurringDue(
  due: string | undefined,
  dueRule: DueRule | undefined,
  scheduledIso: string,
): string | undefined {
  return due ?? (dueRule ? resolveDueRule(dueRule, scheduledIso) : undefined);
}
