import { describe, expect, it } from "vitest";
import { ALL_STATUS_LINE_STAT_IDS, reorderStatIds, toggleStatId } from "./statusLineLayoutEditor";

describe("statusLineLayoutEditor", () => {
  describe("ALL_STATUS_LINE_STAT_IDS", () => {
    it("contains every known status-line stat id exactly once", () => {
      expect(ALL_STATUS_LINE_STAT_IDS).toEqual([
        "status_badge",
        "estimated_time_left",
        "total_time_tracked",
        "avg_time_per_week",
        "completion_pct",
        "weighted_completion_pct",
      ]);
      expect(new Set(ALL_STATUS_LINE_STAT_IDS).size).toBe(ALL_STATUS_LINE_STAT_IDS.length);
    });
  });

  describe("toggleStatId", () => {
    it("appends a stat id to the end when enabling one not currently present", () => {
      const result = toggleStatId(["status_badge", "total_time_tracked"], "estimated_time_left", true);

      expect(result).toEqual(["status_badge", "total_time_tracked", "estimated_time_left"]);
    });

    it("removes a stat id entirely when disabling one currently present", () => {
      const result = toggleStatId(["status_badge", "total_time_tracked", "completion_pct"], "total_time_tracked", false);

      expect(result).toEqual(["status_badge", "completion_pct"]);
    });

    it("is a no-op when enabling a stat id already present", () => {
      const original = ["status_badge", "total_time_tracked"];

      const result = toggleStatId(original, "total_time_tracked", true);

      expect(result).toEqual(original);
    });

    it("is a no-op when disabling a stat id already absent", () => {
      const original = ["status_badge", "total_time_tracked"];

      const result = toggleStatId(original, "completion_pct", false);

      expect(result).toEqual(original);
    });

    it("does not mutate the input array", () => {
      const original = ["status_badge"];

      toggleStatId(original, "completion_pct", true);

      expect(original).toEqual(["status_badge"]);
    });

    it("removes only the matching id when duplicates would otherwise be ambiguous", () => {
      const result = toggleStatId(["status_badge", "total_time_tracked"], "status_badge", false);

      expect(result).toEqual(["total_time_tracked"]);
    });

    it("handles toggling off the only remaining stat id, leaving an empty list", () => {
      const result = toggleStatId(["status_badge"], "status_badge", false);

      expect(result).toEqual([]);
    });
  });

  describe("reorderStatIds", () => {
    it("maps a drag event's items to their ids in order", () => {
      const result = reorderStatIds([{ id: "total_time_tracked" }, { id: "status_badge" }, { id: "completion_pct" }]);

      expect(result).toEqual(["total_time_tracked", "status_badge", "completion_pct"]);
    });

    it("returns an empty array for an empty drag payload", () => {
      expect(reorderStatIds([])).toEqual([]);
    });

    it("preserves a single-item list unchanged", () => {
      expect(reorderStatIds([{ id: "status_badge" }])).toEqual(["status_badge"]);
    });
  });
});
