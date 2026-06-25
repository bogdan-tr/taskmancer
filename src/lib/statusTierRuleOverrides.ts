import type { StatusTierRule } from "./types";

/** The 4 status-tier slots, most-severe-first — matches `Settings.default_status_tier_rules`/`ProjectBoard.status_tier_rule_overrides`' fixed index order throughout this feature. */
export const TIER_COUNT = 4;

/** Display labels for the 4 tier slots, in index order — shared by the global and per-project tier-rule panels so the two never drift apart. */
export const TIER_LABELS: readonly [string, string, string, string] = [
  "Severe",
  "Critical",
  "Needs Attention",
  "On Track",
];

/**
 * Resolves a `<select>` bound to a possibly-empty string into the
 * `string | undefined` shape `StatusTierRule.min_priority`/`ProjectBoard.status_line_layout_id`
 * etc. actually store: an empty string (the "Not set"/"Use global default"
 * option's `value`) becomes `undefined` rather than being saved as a literal
 * empty-string id, which would otherwise either fail priority lookup or
 * silently shadow "inherit" with "explicitly set to nothing".
 */
export function selectValueToOptional(value: string): string | undefined {
  return value === "" ? undefined : value;
}

/**
 * Builds the 4-entry `status_tier_rule_overrides` array `updateProject`
 * expects from this panel's per-tier override state: `overrides[i]` is
 * `null` when that tier's "Override this tier" checkbox is off (inherit the
 * matching global tier), or the tier's own `StatusTierRule` when it's on.
 * Always exactly `TIER_COUNT` entries regardless of how many tiers are
 * actually overridden, satisfying the backend's
 * `status_tier::validate_status_tier_rule_overrides` invariant (a
 * shorter/sparse array is rejected) — see `ProjectBoard.status_tier_rule_overrides`'s
 * own doc comment.
 *
 * `enabled`/`rules` must each have exactly `TIER_COUNT` entries, aligned
 * index-for-index; a mismatched length is a caller bug, not a runtime
 * recovery case, since both arrays are built from the same fixed 4-tier UI
 * state.
 */
export function buildStatusTierRuleOverrides(
  enabled: readonly boolean[],
  rules: readonly StatusTierRule[],
): (StatusTierRule | null)[] {
  return enabled.map((isEnabled, index) => (isEnabled ? rules[index] : null));
}

/**
 * The inverse of `buildStatusTierRuleOverrides`: given a project's saved
 * `status_tier_rule_overrides` (possibly `undefined`, or shorter than
 * `TIER_COUNT` from a legacy/malformed save), returns which of the 4 slots
 * are currently overridden — `undefined`/a missing slot reads as "not
 * overridden", mirroring the backend evaluator's own out-of-bounds-falls-back-
 * to-global tolerance (`status_tier::effective_status_tier_rules`).
 */
export function overriddenTierSlots(overrides: (StatusTierRule | null)[] | undefined): boolean[] {
  return Array.from({ length: TIER_COUNT }, (_, index) => overrides?.[index] != null);
}
