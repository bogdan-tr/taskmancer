/**
 * The visual variant for a formatted due-date label.
 *
 * - `"today"` / `"tomorrow"`: warm highlights (red / orange) — always active.
 * - `"overdue"`: red highlight for past-due tasks — always active.
 * - `"normal"`: no special highlight.
 */
export type DueDateVariant = "normal" | "today" | "tomorrow" | "overdue";

export interface DueDateDisplay {
  label: string;
  variant: DueDateVariant;
}

const WEEKDAY_NAMES = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

/**
 * Formats a `YYYY-MM-DD` due-date string for display.
 *
 * Always-on rules (D):
 * - Past → `{ label: "Overdue · Xd", variant: "overdue" }`
 * - Today → `{ label: "Due today", variant: "today" }`
 * - Tomorrow → `{ label: "Due tomorrow", variant: "tomorrow" }`
 *
 * Natural-language rules (E, only when `nlEnabled`):
 * - 2–6 days out → `{ label: "Due this {Weekday}", variant: "normal" }`
 * - 7–13 days out → `{ label: "Due next {Weekday}", variant: "normal" }`
 *
 * Fallback: `{ label: "Due YYYY-MM-DD", variant: "normal" }`.
 *
 * Returns `null` when `due` is `undefined`.
 */
export function formatDueDateDisplay(
  due: string | undefined,
  today: Date,
  nlEnabled: boolean,
): DueDateDisplay | null {
  if (!due) return null;

  const todayStr = formatDateISO(today);
  const daysAway = diffDays(todayStr, due);

  if (daysAway < 0) {
    return { label: `Overdue · ${-daysAway}d`, variant: "overdue" };
  }
  if (daysAway === 0) return { label: "Due today", variant: "today" };
  if (daysAway === 1) return { label: "Due tomorrow", variant: "tomorrow" };

  if (nlEnabled) {
    const dueDate = parseDate(due);
    if (dueDate) {
      const weekday = WEEKDAY_NAMES[dueDate.getDay()];
      if (daysAway >= 2 && daysAway <= 6) {
        return { label: `Due this ${weekday}`, variant: "normal" };
      }
      if (daysAway >= 7 && daysAway <= 13) {
        return { label: `Due next ${weekday}`, variant: "normal" };
      }
    }
  }

  return { label: `Due ${due}`, variant: "normal" };
}

function formatDateISO(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

/** Parses a `YYYY-MM-DD` string into a local-timezone Date, or returns `null` on invalid input. */
function parseDate(iso: string): Date | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso);
  if (!match) return null;
  const [, y, m, d] = match.map(Number);
  const date = new Date(y, m - 1, d);
  if (date.getFullYear() !== y || date.getMonth() !== m - 1 || date.getDate() !== d) return null;
  return date;
}

/**
 * Returns the number of calendar days from `fromISO` to `toISO`.
 * Negative when `toISO` is in the past relative to `fromISO`.
 */
function diffDays(fromISO: string, toISO: string): number {
  const from = parseDate(fromISO);
  const to = parseDate(toISO);
  if (!from || !to) return 0;
  return Math.round((to.getTime() - from.getTime()) / (1000 * 60 * 60 * 24));
}
