import { describe, expect, it } from "vitest";
import {
  buildStatusTierRuleOverrides,
  overriddenTierSlots,
  selectValueToOptional,
  TIER_COUNT,
  TIER_LABELS,
} from "./statusTierRuleOverrides";
import type { StatusTierRule } from "./types";

function rule(overrides: Partial<StatusTierRule> = {}): StatusTierRule {
  return { due_within_days: undefined, min_priority: undefined, estimated_time_left_exceeds_minutes: undefined, ...overrides };
}

describe("statusTierRuleOverrides", () => {
  describe("TIER_LABELS", () => {
    it("has exactly TIER_COUNT entries in severe-first order", () => {
      expect(TIER_LABELS).toHaveLength(TIER_COUNT);
      expect(TIER_LABELS).toEqual(["Severe", "Critical", "Needs Attention", "On Track"]);
    });
  });

  describe("selectValueToOptional", () => {
    it("resolves an empty string to undefined", () => {
      expect(selectValueToOptional("")).toBeUndefined();
    });

    it("passes through a non-empty string unchanged", () => {
      expect(selectValueToOptional("high")).toBe("high");
    });

    it("treats whitespace-only input as a real value, not empty", () => {
      expect(selectValueToOptional(" ")).toBe(" ");
    });
  });

  describe("buildStatusTierRuleOverrides", () => {
    it("returns 4 nulls when no tier is enabled", () => {
      const result = buildStatusTierRuleOverrides(
        [false, false, false, false],
        [rule(), rule(), rule(), rule()],
      );

      expect(result).toEqual([null, null, null, null]);
    });

    it("returns the tier's own rule for each enabled slot and null for disabled slots", () => {
      const severeRule = rule({ due_within_days: 0 });
      const onTrackRule = rule({ due_within_days: 14 });

      const result = buildStatusTierRuleOverrides(
        [true, false, false, true],
        [severeRule, rule({ due_within_days: 1 }), rule({ due_within_days: 3 }), onTrackRule],
      );

      expect(result).toEqual([severeRule, null, null, onTrackRule]);
    });

    it("always returns exactly TIER_COUNT entries", () => {
      const result = buildStatusTierRuleOverrides(
        [true, true, true, true],
        [rule(), rule(), rule(), rule()],
      );

      expect(result).toHaveLength(TIER_COUNT);
    });
  });

  describe("overriddenTierSlots", () => {
    it("returns all false when overrides is undefined", () => {
      expect(overriddenTierSlots(undefined)).toEqual([false, false, false, false]);
    });

    it("returns true only for slots with a non-null rule", () => {
      const overrides = [rule({ due_within_days: 0 }), null, rule({ due_within_days: 3 }), null];

      expect(overriddenTierSlots(overrides)).toEqual([true, false, true, false]);
    });

    it("treats a too-short overrides array's missing slots as not overridden", () => {
      const overrides = [rule({ due_within_days: 0 })];

      expect(overriddenTierSlots(overrides)).toEqual([true, false, false, false]);
    });

    it("returns all false for an array of all nulls", () => {
      expect(overriddenTierSlots([null, null, null, null])).toEqual([false, false, false, false]);
    });
  });
});
