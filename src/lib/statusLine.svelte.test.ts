import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProjectStatusStats } from "./types";

function makeStats(overrides: Partial<ProjectStatusStats> = {}): ProjectStatusStats {
  return {
    status_tier: "great",
    estimated_time_left: 0,
    total_time_tracked: 0,
    avg_time_per_week: 0,
    completion_pct: undefined,
    weighted_completion_pct: undefined,
    effective_layout_id: "layout-1",
    ...overrides,
  };
}

vi.mock("./api", () => ({
  getProjectStatusStats: vi.fn(),
}));

describe("statusLine.svelte", () => {
  afterEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
  });

  describe("refreshProjectStatusStats", () => {
    it("populates state with the loaded stats and the requested project id", async () => {
      const { getProjectStatusStats } = await import("./api");
      const stats = makeStats({ status_tier: "critical" });
      vi.mocked(getProjectStatusStats).mockResolvedValue(stats);
      const { statusLineState, refreshProjectStatusStats } = await import("./statusLine.svelte");

      await refreshProjectStatusStats("project-1", "monday");

      expect(getProjectStatusStats).toHaveBeenCalledWith("project-1", "monday");
      expect(statusLineState.projectId).toBe("project-1");
      expect(statusLineState.stats).toEqual(stats);
    });

    it("preserves the prior stats and project id when the request fails", async () => {
      const { getProjectStatusStats } = await import("./api");
      const stats = makeStats();
      vi.mocked(getProjectStatusStats)
        .mockResolvedValueOnce(stats)
        .mockRejectedValueOnce(new Error("network error"));
      const { statusLineState, refreshProjectStatusStats } = await import("./statusLine.svelte");

      await refreshProjectStatusStats("project-1", "monday");
      expect(statusLineState.stats).toEqual(stats);

      await expect(refreshProjectStatusStats("project-2", "monday")).resolves.toBeUndefined();
      expect(statusLineState.projectId).toBe("project-1");
      expect(statusLineState.stats).toEqual(stats);
    });

    it("overwrites stats for a previously-loaded different project", async () => {
      const { getProjectStatusStats } = await import("./api");
      const firstStats = makeStats({ status_tier: "on_track" });
      const secondStats = makeStats({ status_tier: "severe" });
      vi.mocked(getProjectStatusStats).mockResolvedValueOnce(firstStats).mockResolvedValueOnce(secondStats);
      const { statusLineState, refreshProjectStatusStats } = await import("./statusLine.svelte");

      await refreshProjectStatusStats("project-1", "monday");
      expect(statusLineState.projectId).toBe("project-1");

      await refreshProjectStatusStats("project-2", "sunday");
      expect(statusLineState.projectId).toBe("project-2");
      expect(statusLineState.stats).toEqual(secondStats);
    });
  });

  describe("hasFreshStatsFor", () => {
    it("returns false before any load", async () => {
      const { hasFreshStatsFor } = await import("./statusLine.svelte");

      expect(hasFreshStatsFor("project-1")).toBe(false);
    });

    it("returns true once stats for that exact project are loaded", async () => {
      const { getProjectStatusStats } = await import("./api");
      vi.mocked(getProjectStatusStats).mockResolvedValue(makeStats());
      const { hasFreshStatsFor, refreshProjectStatusStats } = await import("./statusLine.svelte");

      await refreshProjectStatusStats("project-1", "monday");

      expect(hasFreshStatsFor("project-1")).toBe(true);
    });

    it("returns false while stats for a different project are still showing", async () => {
      const { getProjectStatusStats } = await import("./api");
      vi.mocked(getProjectStatusStats).mockResolvedValue(makeStats());
      const { hasFreshStatsFor, refreshProjectStatusStats } = await import("./statusLine.svelte");

      await refreshProjectStatusStats("project-1", "monday");

      expect(hasFreshStatsFor("project-2")).toBe(false);
    });
  });
});
