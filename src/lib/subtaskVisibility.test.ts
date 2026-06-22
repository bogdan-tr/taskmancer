import { describe, expect, it } from "vitest";
import { isHiddenAsSubtask } from "./subtaskVisibility";
import type { Task } from "./types";

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: crypto.randomUUID(),
    title: "Task",
    status: "backlog",
    tags: [],
    priority: "medium",
    order: 1,
    created: "2026-06-11T00:00:00+00:00",
    depends_on: [],
    tracked_minutes: 0,
    notes: "",
    project_id: "work",
    ...overrides,
  };
}

describe("isHiddenAsSubtask", () => {
  it("is false for a task that isn't a subtask", () => {
    const task = makeTask({ project_id: "work" });
    expect(isHiddenAsSubtask(task, [task], "work")).toBe(false);
  });

  it("is true for a subtask viewed from anywhere other than its own container", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container-1", project_id: "work" });
    const subtask = makeTask({ id: "sub", project_id: "container-1" });
    const allTasks = [parent, subtask];

    expect(isHiddenAsSubtask(subtask, allTasks, undefined)).toBe(true);
    expect(isHiddenAsSubtask(subtask, allTasks, "work")).toBe(true);
  });

  it("is false for a subtask viewed directly on its own container's board", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container-1", project_id: "work" });
    const subtask = makeTask({ id: "sub", project_id: "container-1" });
    const allTasks = [parent, subtask];

    expect(isHiddenAsSubtask(subtask, allTasks, "container-1")).toBe(false);
  });
});
