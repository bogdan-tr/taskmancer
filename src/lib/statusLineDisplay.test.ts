import { describe, expect, it } from "vitest";
import {
  formatAvgTimePerWeek,
  formatCompletionPct,
  formattedStatValue,
  isKnownStatusLineStatId,
  statLabel,
  tierLabel,
  tierTintColor,
} from "./statusLineDisplay";
import type { ProjectStatusStats, StatusTier } from "./types";

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

describe("isKnownStatusLineStatId", () => {
  it("recognizes every catalog stat id, including status_badge", () => {
    expect(isKnownStatusLineStatId("status_badge")).toBe(true);
    expect(isKnownStatusLineStatId("estimated_time_left")).toBe(true);
    expect(isKnownStatusLineStatId("total_time_tracked")).toBe(true);
    expect(isKnownStatusLineStatId("avg_time_per_week")).toBe(true);
    expect(isKnownStatusLineStatId("completion_pct")).toBe(true);
    expect(isKnownStatusLineStatId("weighted_completion_pct")).toBe(true);
  });

  it("rejects an unrecognized id", () => {
    expect(isKnownStatusLineStatId("not_a_real_stat")).toBe(false);
  });
});

describe("statLabel", () => {
  it("returns a short label for each non-badge stat", () => {
    expect(statLabel("estimated_time_left")).toBe("Time left");
    expect(statLabel("total_time_tracked")).toBe("Tracked");
    expect(statLabel("avg_time_per_week")).toBe("Avg/week");
    expect(statLabel("completion_pct")).toBe("Complete");
    expect(statLabel("weighted_completion_pct")).toBe("Complete (weighted)");
  });
});

describe("formatCompletionPct", () => {
  it("renders an em dash for undefined (no meaningful population), not 0%", () => {
    expect(formatCompletionPct(undefined)).toBe("—");
  });

  it("renders 0% distinctly from undefined when the fraction is a real zero", () => {
    expect(formatCompletionPct(0)).toBe("0%");
  });

  it("rounds to the nearest whole percent", () => {
    expect(formatCompletionPct(0.624)).toBe("62%");
    expect(formatCompletionPct(0.626)).toBe("63%");
  });

  it("renders a full population as 100%", () => {
    expect(formatCompletionPct(1)).toBe("100%");
  });
});

describe("formatAvgTimePerWeek", () => {
  it("converts seconds to minutes before formatting", () => {
    expect(formatAvgTimePerWeek(5400)).toBe("1h 30m");
  });

  it("renders zero seconds as 0m", () => {
    expect(formatAvgTimePerWeek(0)).toBe("0m");
  });

  it("floors a fractional minute remainder from the seconds-to-minutes conversion", () => {
    expect(formatAvgTimePerWeek(90)).toBe("1m");
  });
});

describe("formattedStatValue", () => {
  it("formats estimated_time_left and total_time_tracked as minutes", () => {
    const stats = makeStats({ estimated_time_left: 90, total_time_tracked: 125 });
    expect(formattedStatValue("estimated_time_left", stats)).toBe("1h 30m");
    expect(formattedStatValue("total_time_tracked", stats)).toBe("2h 5m");
  });

  it("formats avg_time_per_week by converting seconds to minutes", () => {
    const stats = makeStats({ avg_time_per_week: 3600 });
    expect(formattedStatValue("avg_time_per_week", stats)).toBe("1h");
  });

  it("formats completion_pct and weighted_completion_pct as rounded percentages", () => {
    const stats = makeStats({ completion_pct: 0.5, weighted_completion_pct: undefined });
    expect(formattedStatValue("completion_pct", stats)).toBe("50%");
    expect(formattedStatValue("weighted_completion_pct", stats)).toBe("—");
  });

  it("returns undefined for status_badge, since it has no tile/chip/text value", () => {
    expect(formattedStatValue("status_badge", makeStats())).toBeUndefined();
  });

  it("returns undefined for an unrecognized stat id", () => {
    expect(formattedStatValue("not_a_real_stat", makeStats())).toBeUndefined();
  });
});

describe("tierLabel", () => {
  it("returns a human-readable label for every tier", () => {
    const tiers: StatusTier[] = ["severe", "critical", "needs_attention", "on_track", "great"];
    const labels = tiers.map(tierLabel);
    expect(labels).toEqual(["Severe", "Critical", "Needs Attention", "On Track", "Great"]);
  });
});

describe("tierTintColor", () => {
  it("returns a distinct color for each tier", () => {
    const tiers: StatusTier[] = ["severe", "critical", "needs_attention", "on_track", "great"];
    const colors = new Set(tiers.map(tierTintColor));
    expect(colors.size).toBe(tiers.length);
  });

  it("returns a valid-looking oklch color string for every tier", () => {
    const tiers: StatusTier[] = ["severe", "critical", "needs_attention", "on_track", "great"];
    for (const tier of tiers) {
      expect(tierTintColor(tier)).toMatch(/^oklch\(/);
    }
  });
});
