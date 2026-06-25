import { afterEach, describe, expect, it, vi } from "vitest";
import type { Settings } from "./types";

const sampleSettings: Settings = {
  priorities: [
    { id: "high", label: "High", color: "oklch(54% 0.2 350)", rank: 1 },
    { id: "medium", label: "Medium", color: "oklch(58% 0.13 70)", rank: 2 },
    { id: "low", label: "Low", color: "oklch(58% 0.14 155)", rank: 3 },
  ],
  statuses: [
    { id: "backlog", label: "Backlog", order: 1, color: "oklch(55% 0.01 270)" },
    { id: "do", label: "Do", order: 2, color: "oklch(52% 0.16 235)" },
    { id: "in-progress", label: "In Progress", order: 3, color: "oklch(64% 0.14 75)" },
    { id: "blocked", label: "Blocked", order: 4, color: "oklch(54% 0.2 350)" },
    { id: "done", label: "Done", order: 5, color: "oklch(58% 0.14 155)" },
  ],
  defaults: { tags: [], priority: "medium", status: "backlog" },
  done_status: "done",
  cancelled_status: undefined,
  default_project_id: "general-id",
  show_previous_weeks_column: false,
  card_lightness: 0.5,
  bar_lightness: 0.38,
  ink_mode: "auto",
  show_subproject_tasks_default: false,
  parent_estimate_includes_own_value: false,
  max_visible_subtasks: 5,
  tracking_auto_transition_enabled: false,
  card_tracked_time_display: "total",
  default_status_tier_rules: [],
  avg_time_per_week_window: 4,
  default_status_line_layout_id: "layout-1",
  status_bar_style: "tiles",
  status_bar_enabled: true,
  status_bar_tile_tint: false,
};

vi.mock("./api", () => ({
  getSettings: vi.fn(),
  saveSettings: vi.fn(),
}));

describe("settings.svelte", () => {
  afterEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
  });

  it("refreshSettings populates state from getSettings", async () => {
    const { getSettings } = await import("./api");
    vi.mocked(getSettings).mockResolvedValue(sampleSettings);
    const { settingsState, refreshSettings } = await import("./settings.svelte");

    await refreshSettings();

    expect(settingsState.current).toEqual(sampleSettings);
  });

  it("refreshSettings preserves the prior settings when the request fails", async () => {
    const { getSettings } = await import("./api");
    vi.mocked(getSettings)
      .mockResolvedValueOnce(sampleSettings)
      .mockRejectedValueOnce(new Error("network error"));
    const { settingsState, refreshSettings } = await import("./settings.svelte");

    await refreshSettings();
    expect(settingsState.current).toEqual(sampleSettings);

    await expect(refreshSettings()).resolves.toBeUndefined();
    expect(settingsState.current).toEqual(sampleSettings);
  });

  it("refreshSettings leaves state undefined when the first request fails", async () => {
    const { getSettings } = await import("./api");
    vi.mocked(getSettings).mockRejectedValue(new Error("network error"));
    const { settingsState, refreshSettings } = await import("./settings.svelte");

    await refreshSettings();

    expect(settingsState.current).toBeUndefined();
  });

  it("persistSettings updates state from the save result", async () => {
    const { saveSettings } = await import("./api");
    const updated: Settings = {
      ...sampleSettings,
      defaults: { ...sampleSettings.defaults, priority: "high" },
    };
    vi.mocked(saveSettings).mockResolvedValue(updated);
    const { settingsState, persistSettings } = await import("./settings.svelte");

    await persistSettings(updated);

    expect(settingsState.current).toEqual(updated);
  });
});
