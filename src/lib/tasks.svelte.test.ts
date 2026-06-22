import { afterEach, describe, expect, it, vi } from "vitest";
import type { Task } from "./types";

function makeTask(id: string): Task {
  return {
    id,
    title: "Task",
    status: "backlog",
    tags: [],
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

describe("tasks.svelte", () => {
  afterEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
  });

  it("refreshTasks populates state with the full task list", async () => {
    const { listTasks } = await import("./api");
    const tasks = [makeTask("a"), makeTask("b")];
    vi.mocked(listTasks).mockResolvedValue(tasks);
    const { tasksState, refreshTasks } = await import("./tasks.svelte");

    await refreshTasks();

    expect(tasksState.items).toEqual(tasks);
  });

  it("refreshTasks preserves the prior list when the request fails", async () => {
    const { listTasks } = await import("./api");
    vi.mocked(listTasks)
      .mockResolvedValueOnce([makeTask("a")])
      .mockRejectedValueOnce(new Error("network error"));
    const { tasksState, refreshTasks } = await import("./tasks.svelte");

    await refreshTasks();
    expect(tasksState.items).toEqual([makeTask("a")]);

    await expect(refreshTasks()).resolves.toBeUndefined();
    expect(tasksState.items).toEqual([makeTask("a")]);
  });
});
