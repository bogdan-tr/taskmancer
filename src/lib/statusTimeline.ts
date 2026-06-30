import type { StatusHistoryEntry } from "./types";

/**
 * One node in the detail panel's status timeline: a status the task entered,
 * and how long it stayed there. Built from the raw `StatusHistoryEntry[]`
 * returned by `getTaskHistory` (see `status_history.rs`).
 */
export interface TimelineRow {
  /** The status entered at this node (`to_status`). */
  status: string;
  /** ISO timestamp the task entered this status. */
  changedAt: string;
  /** Milliseconds spent in this status — until the next transition, or until
   *  `now` for the most recent (current) status. Never negative. */
  durationMs: number;
  /** `true` for the most recent status (the one the task is in now). */
  isCurrent: boolean;
  /** `true` when this row came from a back-filled `source: "seed"` event,
   *  shown with a dashed border + `~` prefix to mark it as inferred. */
  isSeed: boolean;
}

/**
 * Turns a task's status history into timeline rows. Entries are sorted
 * ascending by `changed_at` defensively (the backend already returns them
 * sorted, but a caller may pass an arbitrary slice). Each row's duration runs
 * until the next transition; the final row runs until `now`, clamped so clock
 * skew never yields a negative span.
 */
export function buildStatusTimeline(
  entries: StatusHistoryEntry[],
  now: Date = new Date(),
): TimelineRow[] {
  if (entries.length === 0) return [];

  const sorted = [...entries].sort(
    (a, b) => Date.parse(a.changed_at) - Date.parse(b.changed_at),
  );
  const nowMs = now.getTime();

  return sorted.map((entry, index) => {
    const startMs = Date.parse(entry.changed_at);
    const isCurrent = index === sorted.length - 1;
    const endMs = isCurrent ? nowMs : Date.parse(sorted[index + 1].changed_at);
    const durationMs = Math.max(0, endMs - startMs);

    return {
      status: entry.to_status,
      changedAt: entry.changed_at,
      durationMs,
      isCurrent,
      isSeed: entry.source === "seed",
    };
  });
}

const MS_PER_MINUTE = 60_000;
const MS_PER_HOUR = 60 * MS_PER_MINUTE;
const MS_PER_DAY = 24 * MS_PER_HOUR;

/**
 * Formats a status duration for the timeline. Distinct from
 * `estimatedTime.formatMinutes` (hours/minutes only) because a task can sit in
 * a status for days — this shows the two largest meaningful units (`3d 4h`,
 * `2h 15m`, `5m`) and collapses anything under a minute to `< 1m`.
 */
export function formatStatusDuration(ms: number): string {
  if (ms < MS_PER_MINUTE) return "< 1m";

  const days = Math.floor(ms / MS_PER_DAY);
  const hours = Math.floor((ms % MS_PER_DAY) / MS_PER_HOUR);
  const minutes = Math.floor((ms % MS_PER_HOUR) / MS_PER_MINUTE);

  if (days > 0) {
    return hours > 0 ? `${days}d ${hours}h` : `${days}d`;
  }
  if (hours > 0) {
    return minutes > 0 ? `${hours}h ${minutes}m` : `${hours}h`;
  }
  return `${minutes}m`;
}
