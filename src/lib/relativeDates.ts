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
