import type { StatusDefinition } from "./types";

/**
 * Seeded statuses matching `Settings::default()` in the Rust backend.
 * Used as a fallback wherever a component renders statuses before settings
 * have finished loading.
 */
export const FALLBACK_STATUSES: StatusDefinition[] = [
  { id: "backlog", label: "Backlog", order: 1, color: "#6f7178" },
  { id: "do", label: "Do", order: 2, color: "#0073b6" },
  { id: "in-progress", label: "In Progress", order: 3, color: "#bd7d00" },
  { id: "blocked", label: "Blocked", order: 4, color: "#bc267f" },
  { id: "done", label: "Done", order: 5, color: "#0e9254" },
];

/**
 * Fallback color for a status id that has no matching `StatusDefinition`
 * (e.g. a task referencing a since-removed status). Matches
 * `default_status_color()` in `src-tauri/src/settings.rs`.
 */
export const FALLBACK_STATUS_COLOR = "#807973";

/** Returns `statuses` sorted by `order` ascending (order 1 sorts first). */
export function sortedStatuses(statuses: StatusDefinition[]): StatusDefinition[] {
  return [...statuses].sort((a, b) => a.order - b.order);
}

/**
 * Returns the display label for the status with id `id`, or `id` itself if
 * no such status exists.
 */
export function statusLabel(statuses: StatusDefinition[], id: string): string {
  return statuses.find((status) => status.id === id)?.label ?? id;
}

/**
 * Returns the color for the status with id `id`, or `FALLBACK_STATUS_COLOR`
 * if no such status exists.
 */
export function statusColor(statuses: StatusDefinition[], id: string): string {
  return statuses.find((status) => status.id === id)?.color ?? FALLBACK_STATUS_COLOR;
}

/**
 * Resolves the status a new task should get when none was explicitly
 * requested, mirroring `resolve_default_status` in the Rust command layer
 * (ignoring its project-board override, which is resolved separately):
 * `defaultStatus` if it names a currently-defined status, otherwise the
 * status with the lowest `order`, otherwise `"backlog"` if no statuses are
 * defined at all.
 */
export function defaultStatusId(statuses: StatusDefinition[], defaultStatus?: string): string {
  if (defaultStatus && statuses.some((status) => status.id === defaultStatus)) {
    return defaultStatus;
  }

  return sortedStatuses(statuses)[0]?.id ?? "backlog";
}
