import { describe, expect, it } from "vitest";
import { isVisibleOnBoard } from "./boardVisibility";
import type { Task } from "./types";

function makeTask(overrides: Partial<Task> & { id: string }): Task {
  return {
    title: "Untitled",
    status: "todo",
    project: undefined,
    tags: [],
    priority: "medium",
    due: undefined,
    scheduled: undefined,
    order: 0,
    created: "2024-01-01T00:00:00Z",
    depends_on: [],
    notes: "",
    ...overrides,
  };
}

describe("isVisibleOnBoard", () => {
  it("is visible when the task has no scheduled date", () => {
    const task = makeTask({ id: "1" });

    expect(isVisibleOnBoard(task, "2024-01-15")).toBe(true);
  });

  it("is visible on the day the task is scheduled", () => {
    const task = makeTask({ id: "1", scheduled: "2024-01-15" });

    expect(isVisibleOnBoard(task, "2024-01-15")).toBe(true);
  });

  it("is visible after the scheduled date has passed", () => {
    const task = makeTask({ id: "1", scheduled: "2024-01-10" });

    expect(isVisibleOnBoard(task, "2024-01-15")).toBe(true);
  });

  it("is hidden before the scheduled date arrives", () => {
    const task = makeTask({ id: "1", scheduled: "2024-01-20" });

    expect(isVisibleOnBoard(task, "2024-01-15")).toBe(false);
  });
});
