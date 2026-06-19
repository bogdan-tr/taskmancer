import { addDays, formatDateISO } from "./weekRange";

/**
 * A relative-date option for `TaskDefaults.due`/`.scheduled`: an `id` stored
 * in settings and a human-readable `label` for dropdowns.
 */
export interface RelativeDateOption {
  id: string;
  label: string;
}

/**
 * Options for `TaskDefaults.scheduled`. Mirrors
 * `SCHEDULED_RELATIVE_DATE_CODES` in `src-tauri/src/settings.rs` — keep both
 * lists in sync. Resolved relative to "today" at task-creation time.
 */
export const SCHEDULED_RELATIVE_DATE_OPTIONS: RelativeDateOption[] = [
  { id: "today", label: "Today" },
  { id: "tomorrow", label: "Tomorrow" },
  { id: "in_2_days", label: "In 2 days" },
  { id: "in_3_days", label: "In 3 days" },
  { id: "in_1_week", label: "In 1 week" },
  { id: "in_1_month", label: "In 1 month" },
];

/**
 * Options for `TaskDefaults.due`. Mirrors `DUE_RELATIVE_DATE_CODES` in
 * `src-tauri/src/settings.rs` — keep both lists in sync. Resolved relative to
 * the task's *scheduled* date (not "today") at task-creation time. `"none"`
 * means "never due".
 */
export const DUE_RELATIVE_DATE_OPTIONS: RelativeDateOption[] = [
  { id: "none", label: "Never" },
  { id: "same_day", label: "Same day" },
  { id: "next_day", label: "Next day" },
  { id: "in_2_days", label: "2 days later" },
  { id: "in_3_days", label: "3 days later" },
  { id: "in_1_week", label: "1 week later" },
  { id: "in_1_month", label: "1 month later" },
];

/** Returns the display label for a scheduled-date option id, or the id itself if unrecognized. */
export function scheduledRelativeDateLabel(id: string): string {
  return SCHEDULED_RELATIVE_DATE_OPTIONS.find((option) => option.id === id)?.label ?? id;
}

/** Returns the display label for a due-date option id, or the id itself if unrecognized. */
export function dueRelativeDateLabel(id: string): string {
  return DUE_RELATIVE_DATE_OPTIONS.find((option) => option.id === id)?.label ?? id;
}

/** Parses a `YYYY-MM-DD` string into a local-timezone `Date`. Assumes well-formed input. */
function parseISODate(iso: string): Date {
  const [year, month, day] = iso.split("-").map(Number);
  return new Date(year, month - 1, day);
}

/**
 * Returns a new Date offset by `months` (may be negative), via JS's normal
 * calendar-rollover semantics (e.g. Jan 31 + 1 month becomes Mar 3, since
 * February doesn't have a 31st). This differs from the Rust backend's
 * `checked_add_months`, which returns `None` (no date applied) for such an
 * overflowing day-of-month instead of rolling over. That's an acceptable
 * divergence here since this module is preview-only — the actual saved date
 * is always computed authoritatively by the backend at creation time.
 */
function addMonths(date: Date, months: number): Date {
  const result = new Date(date);
  result.setMonth(result.getMonth() + months);
  return result;
}

/**
 * Resolves a [`SCHEDULED_RELATIVE_DATE_OPTIONS`] `code` to a `YYYY-MM-DD`
 * date relative to `today`, mirroring `resolve_scheduled_relative_date` in
 * `src-tauri/src/settings.rs`. Returns `undefined` for an unrecognized code.
 */
export function resolveScheduledRelativeDate(code: string, today: Date): string | undefined {
  switch (code) {
    case "today":
      return formatDateISO(today);
    case "tomorrow":
      return formatDateISO(addDays(today, 1));
    case "in_2_days":
      return formatDateISO(addDays(today, 2));
    case "in_3_days":
      return formatDateISO(addDays(today, 3));
    case "in_1_week":
      return formatDateISO(addDays(today, 7));
    case "in_1_month":
      return formatDateISO(addMonths(today, 1));
    default:
      return undefined;
  }
}

/**
 * Resolves a [`DUE_RELATIVE_DATE_OPTIONS`] `code` to a `YYYY-MM-DD` date
 * relative to `scheduledISO` (not "today"), mirroring
 * `resolve_due_relative_date` in `src-tauri/src/settings.rs`. `"none"`
 * always resolves to `undefined` (never due). Returns `undefined` for an
 * unrecognized code.
 */
export function resolveDueRelativeDate(code: string, scheduledISO: string): string | undefined {
  if (code === "none") return undefined;

  const scheduled = parseISODate(scheduledISO);
  switch (code) {
    case "same_day":
      return formatDateISO(scheduled);
    case "next_day":
      return formatDateISO(addDays(scheduled, 1));
    case "in_2_days":
      return formatDateISO(addDays(scheduled, 2));
    case "in_3_days":
      return formatDateISO(addDays(scheduled, 3));
    case "in_1_week":
      return formatDateISO(addDays(scheduled, 7));
    case "in_1_month":
      return formatDateISO(addMonths(scheduled, 1));
    default:
      return undefined;
  }
}
