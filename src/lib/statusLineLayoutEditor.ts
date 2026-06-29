import type { StatusLineStatId } from "./statusLineDisplay";

/** Every stat id offered in the layout editor's toggle list, in a fixed catalog order — independent of any one layout's current `stat_ids` ordering. Mirrors the backend's `layout::KNOWN_STATUS_LINE_STAT_IDS` set (see `statusLineDisplay.ts`'s `StatusLineStatId`). Mini widget ids (Phase C) appear at the end. */
export const ALL_STATUS_LINE_STAT_IDS: StatusLineStatId[] = [
  "status_badge",
  "estimated_time_left",
  "total_time_tracked",
  "avg_time_per_week",
  "completion_pct",
  "weighted_completion_pct",
  "active_completion_pct",
  "mini_health",
  "mini_completion",
  "mini_fuel",
  "mini_sparkline",
];

/**
 * Returns the next `stat_ids` array after toggling `statId` on or off, per
 * `StatLayout`'s own invariant: toggling off removes the id from the list
 * entirely rather than marking it disabled in place; toggling on appends it
 * to the end (a freshly re-enabled stat doesn't try to guess its old
 * position). A no-op if `enabled` already matches `statId`'s current
 * presence in `statIds` (e.g. a stale double-toggle).
 */
export function toggleStatId(statIds: string[], statId: string, enabled: boolean): string[] {
  const isPresent = statIds.includes(statId);
  if (enabled === isPresent) return statIds;
  return enabled ? [...statIds, statId] : statIds.filter((id) => id !== statId);
}

/**
 * Returns `statIds` reordered to match `reordered`'s element sequence — the
 * shape `svelte-dnd-action`'s `onconsider`/`onfinalize` events hand back
 * (`event.detail.items`, the full post-drag array) for a flat, ungrouped
 * dndzone. Exists as a named function (rather than inlining the assignment)
 * purely so the "what does a drag event's payload become" logic has a
 * single, independently testable home rather than living only inside a
 * Svelte event handler.
 */
export function reorderStatIds(reordered: { id: string }[]): string[] {
  return reordered.map((item) => item.id);
}
