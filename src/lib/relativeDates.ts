/**
 * A relative-date option for `TaskDefaults.due`/`.scheduled`: an `id` stored
 * in settings and a human-readable `label` for dropdowns. Mirrors
 * `RELATIVE_DATE_CODES` in `src-tauri/src/settings.rs` — keep both lists in
 * sync.
 */
export interface RelativeDateOption {
  id: string;
  label: string;
}

export const RELATIVE_DATE_OPTIONS: RelativeDateOption[] = [
  { id: "today", label: "Today" },
  { id: "tomorrow", label: "Tomorrow" },
  { id: "in_2_days", label: "In 2 days" },
  { id: "in_3_days", label: "In 3 days" },
  { id: "in_1_week", label: "In 1 week" },
  { id: "in_1_month", label: "In 1 month" },
];

/** Returns the display label for a relative-date option id, or the id itself if unrecognized. */
export function relativeDateLabel(id: string): string {
  return RELATIVE_DATE_OPTIONS.find((option) => option.id === id)?.label ?? id;
}
