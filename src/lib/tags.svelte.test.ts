import { afterEach, describe, expect, it, vi } from "vitest";
import type { Task } from "./types";

function makeTask(tags: string[]): Task {
  return {
    id: crypto.randomUUID(),
    title: "Task",
    status: "backlog",
    tags,
    priority: "medium",
    order: 1,
    created: "2026-06-11T00:00:00+00:00",
    depends_on: [],
    tracked_minutes: 0,
    notes: "",
  };
}

vi.mock("./api", () => ({
  listTasks: vi.fn(),
}));

describe("tags.svelte", () => {
  afterEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
  });

  it("refreshTags populates state with the distinct tags across all tasks, sorted", async () => {
    const { listTasks } = await import("./api");
    vi.mocked(listTasks).mockResolvedValue([
      makeTask(["urgent", "home"]),
      makeTask(["home", "errand"]),
    ]);
    const { tagsState, refreshTags } = await import("./tags.svelte");

    await refreshTags();

    expect(tagsState.items).toEqual(["errand", "home", "urgent"]);
  });

  it("refreshTags preserves the prior list when the request fails", async () => {
    const { listTasks } = await import("./api");
    vi.mocked(listTasks)
      .mockResolvedValueOnce([makeTask(["urgent"])])
      .mockRejectedValueOnce(new Error("network error"));
    const { tagsState, refreshTags } = await import("./tags.svelte");

    await refreshTags();
    expect(tagsState.items).toEqual(["urgent"]);

    await expect(refreshTags()).resolves.toBeUndefined();
    expect(tagsState.items).toEqual(["urgent"]);
  });
});
