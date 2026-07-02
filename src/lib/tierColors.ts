/**
 * The ONE canonical health-tier palette (approved 2026-07-01, see
 * docs/features/widget-fixes-2026-07.md). Every surface that renders a
 * health tier — status bar badge, global Project Health widget, W2 Health
 * Pulse, mini widgets, W16/W17 badges — must use these values. Three
 * conflicting palettes used to coexist (Severe was red in one widget and
 * purple in another); do not reintroduce a local palette.
 *
 * The Rust side mirrors these hex values in `commands.rs`
 * (`tier_color_hex`) — keep both in sync.
 */

/** Backend tier ids (snake_case, as returned by `get_global_status_stats`
 *  and `get_dashboard_project_health`). */
export type TierId = "great" | "on_track" | "needs_attention" | "critical" | "severe";

/** Human labels, as returned by W2 / W16 / W17 backends. */
export type TierLabel = "Great" | "On Track" | "Needs Attention" | "Critical" | "Severe";

export const TIER_COLORS: Record<TierId, string> = {
  great: "#22c55e",
  on_track: "#3b82f6",
  needs_attention: "#f59e0b",
  critical: "#f97316",
  severe: "#ef4444",
};

export const TIER_LABELS: Record<TierId, TierLabel> = {
  great: "Great",
  on_track: "On Track",
  needs_attention: "Needs Attention",
  critical: "Critical",
  severe: "Severe",
};

const LABEL_TO_ID: Record<string, TierId> = {
  Great: "great",
  "On Track": "on_track",
  "Needs Attention": "needs_attention",
  Critical: "critical",
  Severe: "severe",
};

/** Color for a tier given either its id ("on_track") or label ("On Track").
 *  Unknown values fall back to the muted Great green so nothing renders
 *  unstyled. */
export function tierColor(tier: string): string {
  const id = (tier in TIER_COLORS ? tier : LABEL_TO_ID[tier]) as TierId | undefined;
  return TIER_COLORS[id ?? "great"];
}

/** Sort weight: higher = more urgent (Severe first when sorting desc). */
export function tierUrgency(tier: string): number {
  const id = (tier in TIER_COLORS ? tier : LABEL_TO_ID[tier]) as TierId | undefined;
  switch (id) {
    case "severe": return 5;
    case "critical": return 4;
    case "needs_attention": return 3;
    case "on_track": return 2;
    default: return 1;
  }
}
