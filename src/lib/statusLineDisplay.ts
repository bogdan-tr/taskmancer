import { formatMinutes } from "./estimatedTime";
import type { ProjectStatusStats, StatusTier } from "./types";

/**
 * Every stat id a `"status_line"` `StatLayout.stat_ids` entry may reference,
 * mirroring `layout::KNOWN_STATUS_LINE_STAT_IDS` exactly — `"status_badge"`
 * is the health tier itself, rendered as part of the bar's header rather
 * than as a tile/chip/text value like the other 5 (see
 * `ProjectStatusLine.svelte`), but it's still a valid, orderable entry in
 * `stat_ids` that this union must recognize.
 */
export type StatusLineStatId =
  | "status_badge"
  | "estimated_time_left"
  | "total_time_tracked"
  | "avg_time_per_week"
  | "completion_pct"
  | "weighted_completion_pct";

/** Short display labels for every stat *other than* `"status_badge"` (which has no tile/chip/text of its own — see `StatusLineStatId`'s doc comment). */
const STAT_LABELS: Record<Exclude<StatusLineStatId, "status_badge">, string> = {
  estimated_time_left: "Time left",
  total_time_tracked: "Tracked",
  avg_time_per_week: "Avg/week",
  completion_pct: "Complete",
  weighted_completion_pct: "Complete (weighted)",
};

/** `true` if `id` is a recognized status-line stat id — guards against a layout referencing a since-removed/unknown id (e.g. a future Phase 3 dashboard-only stat) so rendering can skip it instead of crashing. */
export function isKnownStatusLineStatId(id: string): id is StatusLineStatId {
  return id in STAT_LABELS || id === "status_badge";
}

/** The short display label for `statId`, or `undefined` for `"status_badge"` (no tile/chip/text label — see `STAT_LABELS`). */
export function statLabel(statId: Exclude<StatusLineStatId, "status_badge">): string {
  return STAT_LABELS[statId];
}

/**
 * Formats `completion_pct`/`weighted_completion_pct` (a fraction in
 * `0.0..=1.0`, or `undefined` when there's no meaningful population to
 * divide by) as a rounded whole-number percentage, e.g. `0.624` → `"62%"`.
 * `undefined` renders as an em dash rather than `"0%"` — collapsing "no
 * population" into "0% complete" would lose a real, backend-preserved
 * distinction (see `ProjectStatusStats`'s own doc comment).
 */
export function formatCompletionPct(fraction: number | undefined): string {
  if (fraction === undefined) return "—";
  return `${Math.round(fraction * 100)}%`;
}

/** Formats `avg_time_per_week` (seconds, the backend's native unit for this one stat — see `ProjectStatusStats`'s doc comment) by converting to minutes and reusing `formatMinutes`, rather than introducing a third time formatter alongside it and `formatHms`. */
export function formatAvgTimePerWeek(totalSeconds: number): string {
  return formatMinutes(totalSeconds / 60);
}

/**
 * Resolves the formatted display value for every status-line stat *except*
 * `"status_badge"` (which the caller renders separately as the health badge,
 * not a tile/chip/text value — see `StatusLineStatId`'s doc comment).
 * `undefined` for `"status_badge"` itself or any unrecognized id, so a
 * caller iterating a layout's `stat_ids` can filter those out with a single
 * `undefined` check rather than needing its own id-recognition logic
 * duplicated from `isKnownStatusLineStatId`.
 */
export function formattedStatValue(statId: string, stats: ProjectStatusStats): string | undefined {
  switch (statId) {
    case "estimated_time_left":
      return formatMinutes(stats.estimated_time_left);
    case "total_time_tracked":
      return formatMinutes(stats.total_time_tracked);
    case "avg_time_per_week":
      return formatAvgTimePerWeek(stats.avg_time_per_week);
    case "completion_pct":
      return formatCompletionPct(stats.completion_pct);
    case "weighted_completion_pct":
      return formatCompletionPct(stats.weighted_completion_pct);
    default:
      return undefined;
  }
}

/** Display label for the health badge itself, e.g. `"needs_attention"` → `"Needs Attention"`. */
const TIER_LABELS: Record<StatusTier, string> = {
  severe: "Severe",
  critical: "Critical",
  needs_attention: "Needs Attention",
  on_track: "On Track",
  great: "Great",
};

/** The human-readable label for `tier`, e.g. `"needs_attention"` → `"Needs Attention"`. */
export function tierLabel(tier: StatusTier): string {
  return TIER_LABELS[tier];
}

/**
 * A soft background color wash per health tier, for the `"tint"` bar style —
 * loosely consistent with how this app already uses red/amber/green for
 * status urgency (e.g. the due-date glow, priority colors) without a direct
 * existing tier-to-color precedent to copy exactly. Severe is the most
 * alarming (red), trailing off through amber for the two "watch this"
 * middle tiers, to green for the two healthy tiers — `on_track` and `great`
 * intentionally share the same hue family so a fully healthy project doesn't
 * look like a visually distinct "extra credit" state, just a deeper shade of
 * the same "you're fine" green.
 */
const TIER_TINT_COLORS: Record<StatusTier, string> = {
  severe: "oklch(58% 0.19 25)",
  critical: "oklch(62% 0.17 45)",
  needs_attention: "oklch(72% 0.15 70)",
  on_track: "oklch(75% 0.13 145)",
  great: "oklch(68% 0.15 145)",
};

/** The `"tint"` bar style's background-wash color for `tier`. */
export function tierTintColor(tier: StatusTier): string {
  return TIER_TINT_COLORS[tier];
}
