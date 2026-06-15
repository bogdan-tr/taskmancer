import { describe, expect, it } from "vitest";
import { groupTasksByWeek } from "./weekGrouping";
import type { Task } from "./types";

const WEEK_DATES = ["2024-01-01", "2024-01-02", "2024-01-03", "2024-01-04", "2024-01-05", "2024-01-06", "2024-01-07"];

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

describe("groupTasksByWeek", () => {
  it("returns an empty bar list for each day when there are no tasks", () => {
    const result = groupTasksByWeek([], WEEK_DATES);

    expect(result).toHaveLength(7);
    expect(result.every((dayBars) => dayBars.length === 0)).toBe(true);
  });

  it("places a scheduled task on its scheduled day", () => {
    const task = makeTask({ id: "1", scheduled: "2024-01-03" });

    const result = groupTasksByWeek([task], WEEK_DATES);

    expect(result[2]).toEqual([{ task, type: "scheduled", date: "2024-01-03" }]);
    expect(result[0]).toEqual([]);
    expect(result[1]).toEqual([]);
  });

  it("places a due task on its due day", () => {
    const task = makeTask({ id: "1", due: "2024-01-05" });

    const result = groupTasksByWeek([task], WEEK_DATES);

    expect(result[4]).toEqual([{ task, type: "due", date: "2024-01-05" }]);
  });

  it("creates two separate bars for a task with both scheduled and due dates in the week", () => {
    const task = makeTask({ id: "1", scheduled: "2024-01-02", due: "2024-01-06" });

    const result = groupTasksByWeek([task], WEEK_DATES);

    expect(result[1]).toEqual([{ task, type: "scheduled", date: "2024-01-02" }]);
    expect(result[5]).toEqual([{ task, type: "due", date: "2024-01-06" }]);
  });

  it("lists the scheduled bar before the due bar when both dates fall on the same day", () => {
    const task = makeTask({ id: "1", scheduled: "2024-01-04", due: "2024-01-04" });

    const result = groupTasksByWeek([task], WEEK_DATES);

    expect(result[3]).toEqual([
      { task, type: "scheduled", date: "2024-01-04" },
      { task, type: "due", date: "2024-01-04" },
    ]);
  });

  it("ignores dates outside the given week", () => {
    const task = makeTask({ id: "1", scheduled: "2024-02-01", due: "2023-12-31" });

    const result = groupTasksByWeek([task], WEEK_DATES);

    expect(result.every((dayBars) => dayBars.length === 0)).toBe(true);
  });

  it("ignores tasks with neither a scheduled nor due date", () => {
    const task = makeTask({ id: "1" });

    const result = groupTasksByWeek([task], WEEK_DATES);

    expect(result.every((dayBars) => dayBars.length === 0)).toBe(true);
  });

  it("preserves task order within a day, grouping all scheduled bars before all due bars", () => {
    const taskA = makeTask({ id: "a", due: "2024-01-01" });
    const taskB = makeTask({ id: "b", scheduled: "2024-01-01" });
    const taskC = makeTask({ id: "c", scheduled: "2024-01-01" });
    const taskD = makeTask({ id: "d", due: "2024-01-01" });

    const result = groupTasksByWeek([taskA, taskB, taskC, taskD], WEEK_DATES);

    expect(result[0]).toEqual([
      { task: taskB, type: "scheduled", date: "2024-01-01" },
      { task: taskC, type: "scheduled", date: "2024-01-01" },
      { task: taskA, type: "due", date: "2024-01-01" },
      { task: taskD, type: "due", date: "2024-01-01" },
    ]);
  });
});
