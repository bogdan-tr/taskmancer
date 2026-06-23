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

  it("upsertCachedTask adds a task not already in the cache", async () => {
    const { tasksState, upsertCachedTask } = await import("./tasks.svelte");

    upsertCachedTask(makeTask("a"));

    expect(tasksState.items.map((t) => t.id)).toEqual(["a"]);
  });

  it("upsertCachedTask replaces an existing task in place, preserving order", async () => {
    const { tasksState, upsertCachedTask } = await import("./tasks.svelte");
    upsertCachedTask(makeTask("a"));
    upsertCachedTask(makeTask("b"));

    upsertCachedTask({ ...makeTask("a"), status: "done" });

    expect(tasksState.items.map((t) => t.id)).toEqual(["a", "b"]);
    expect(tasksState.items[0].status).toBe("done");
  });

  it("removeCachedTask removes a task from the cache by id", async () => {
    const { tasksState, upsertCachedTask, removeCachedTask } = await import("./tasks.svelte");
    upsertCachedTask(makeTask("a"));
    upsertCachedTask(makeTask("b"));

    removeCachedTask("a");

    expect(tasksState.items.map((t) => t.id)).toEqual(["b"]);
  });

  it("removeCachedTask is a no-op for an id not present", async () => {
    const { tasksState, upsertCachedTask, removeCachedTask } = await import("./tasks.svelte");
    upsertCachedTask(makeTask("a"));

    removeCachedTask("does-not-exist");

    expect(tasksState.items.map((t) => t.id)).toEqual(["a"]);
  });
});
