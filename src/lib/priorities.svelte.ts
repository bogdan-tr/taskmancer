import type { PriorityLevel } from "./types";

/**
 * Seeded priority levels matching `Settings::default()` in the Rust backend.
 * Used as a fallback wherever a component renders priority levels before
 * settings have finished loading.
 */
export const FALLBACK_PRIORITIES: PriorityLevel[] = [
  { id: "high", label: "High", color: "#bc267f", rank: 1 },
  { id: "medium", label: "Medium", color: "#aa6a00", rank: 2 },
  { id: "low", label: "Low", color: "#0e9254", rank: 3 },
];

/**
 * Fallback color for a priority id that has no matching `PriorityLevel`
 * (e.g. a task referencing a since-removed level). Matches
 * `default_priority_color()` in `src-tauri/src/settings.rs`.
 */
export const FALLBACK_PRIORITY_COLOR = "#807973";

/** Returns `priorities` sorted by `rank` ascending (rank 1 sorts first). */
export function sortedPriorities(priorities: PriorityLevel[]): PriorityLevel[] {
  return [...priorities].sort((a, b) => a.rank - b.rank);
}

/**
 * Returns the display label for the priority level with id `id`, or `id`
 * itself if no such level exists.
 */
export function priorityLabel(priorities: PriorityLevel[], id: string): string {
  return priorities.find((level) => level.id === id)?.label ?? id;
}

/**
 * Returns the color for the priority level with id `id`, or
 * `FALLBACK_PRIORITY_COLOR` if no such level exists.
 */
export function priorityColor(priorities: PriorityLevel[], id: string): string {
  return priorities.find((level) => level.id === id)?.color ?? FALLBACK_PRIORITY_COLOR;
}

/**
 * Resolves the priority a new task should get when none was explicitly
 * requested, mirroring `resolve_default_priority` in the Rust command layer:
 * `defaultPriority` if it names a currently-defined priority level,
 * otherwise the level with the lowest `rank`, otherwise `"medium"` if no
 * priority levels are defined at all.
 */
export function defaultPriorityId(priorities: PriorityLevel[], defaultPriority?: string): string {
  if (defaultPriority && priorities.some((level) => level.id === defaultPriority)) {
    return defaultPriority;
  }

  return sortedPriorities(priorities)[0]?.id ?? "medium";
}
