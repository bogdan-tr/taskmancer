import { describe, expect, it } from "vitest";
import {
  compareByPriorityThenTitle,
  countTasksBeforeWeek,
  groupPreviousWeeksBars,
  groupTasksByWeek,
  isTaskFinished,
} from "./weekGrouping";
import { FALLBACK_PRIORITIES } from "./priorities.svelte";
import type { Task } from "./types";

const WEEK_DATES = ["2024-01-01", "2024-01-02", "2024-01-03", "2024-01-04", "2024-01-05", "2024-01-06", "2024-01-07"];

const PRIORITIES = FALLBACK_PRIORITIES;

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
    const result = groupTasksByWeek([], WEEK_DATES, PRIORITIES);

    expect(result).toHaveLength(7);
    expect(result.every((dayBars) => dayBars.length === 0)).toBe(true);
  });

  it("places a scheduled task on its scheduled day", () => {
    const task = makeTask({ id: "1", scheduled: "2024-01-03" });

    const result = groupTasksByWeek([task], WEEK_DATES, PRIORITIES);

    expect(result[2]).toEqual([{ task, type: "scheduled", date: "2024-01-03" }]);
    expect(result[0]).toEqual([]);
    expect(result[1]).toEqual([]);
  });

  it("places a due task on its due day", () => {
    const task = makeTask({ id: "1", due: "2024-01-05" });

    const result = groupTasksByWeek([task], WEEK_DATES, PRIORITIES);

    expect(result[4]).toEqual([{ task, type: "due", date: "2024-01-05" }]);
  });

  it("creates two separate bars for a task with both scheduled and due dates in the week", () => {
    const task = makeTask({ id: "1", scheduled: "2024-01-02", due: "2024-01-06" });

    const result = groupTasksByWeek([task], WEEK_DATES, PRIORITIES);

    expect(result[1]).toEqual([{ task, type: "scheduled", date: "2024-01-02" }]);
    expect(result[5]).toEqual([{ task, type: "due", date: "2024-01-06" }]);
  });

  it("lists the scheduled bar before the due bar when both dates fall on the same day", () => {
    const task = makeTask({ id: "1", scheduled: "2024-01-04", due: "2024-01-04" });

    const result = groupTasksByWeek([task], WEEK_DATES, PRIORITIES);

    expect(result[3]).toEqual([
      { task, type: "scheduled", date: "2024-01-04" },
      { task, type: "due", date: "2024-01-04" },
    ]);
  });

  it("ignores dates outside the given week", () => {
    const task = makeTask({ id: "1", scheduled: "2024-02-01", due: "2023-12-31" });

    const result = groupTasksByWeek([task], WEEK_DATES, PRIORITIES);

    expect(result.every((dayBars) => dayBars.length === 0)).toBe(true);
  });

  it("ignores tasks with neither a scheduled nor due date", () => {
    const task = makeTask({ id: "1" });

    const result = groupTasksByWeek([task], WEEK_DATES, PRIORITIES);

    expect(result.every((dayBars) => dayBars.length === 0)).toBe(true);
  });

  it("preserves relative order of equal-priority, equal-title bars within a day, grouping all scheduled bars before all due bars", () => {
    const taskA = makeTask({ id: "a", due: "2024-01-01" });
    const taskB = makeTask({ id: "b", scheduled: "2024-01-01" });
    const taskC = makeTask({ id: "c", scheduled: "2024-01-01" });
    const taskD = makeTask({ id: "d", due: "2024-01-01" });

    const result = groupTasksByWeek([taskA, taskB, taskC, taskD], WEEK_DATES, PRIORITIES);

    expect(result[0]).toEqual([
      { task: taskB, type: "scheduled", date: "2024-01-01" },
      { task: taskC, type: "scheduled", date: "2024-01-01" },
      { task: taskA, type: "due", date: "2024-01-01" },
      { task: taskD, type: "due", date: "2024-01-01" },
    ]);
  });

  it("sorts same-day bars by priority rank, highest priority first", () => {
    const low = makeTask({ id: "low", title: "Zebra", priority: "low", scheduled: "2024-01-01" });
    const high = makeTask({ id: "high", title: "Apple", priority: "high", scheduled: "2024-01-01" });
    const medium = makeTask({ id: "medium", title: "Mango", priority: "medium", scheduled: "2024-01-01" });

    const result = groupTasksByWeek([low, high, medium], WEEK_DATES, PRIORITIES);

    expect(result[0]).toEqual([
      { task: high, type: "scheduled", date: "2024-01-01" },
      { task: medium, type: "scheduled", date: "2024-01-01" },
      { task: low, type: "scheduled", date: "2024-01-01" },
    ]);
  });

  it("tiebreaks same-priority same-day bars alphabetically by title, case-insensitively", () => {
    const taskB = makeTask({ id: "b", title: "banana", priority: "medium", scheduled: "2024-01-01" });
    const taskA = makeTask({ id: "a", title: "Apple", priority: "medium", scheduled: "2024-01-01" });
    const taskC = makeTask({ id: "c", title: "Cherry", priority: "medium", scheduled: "2024-01-01" });

    const result = groupTasksByWeek([taskB, taskA, taskC], WEEK_DATES, PRIORITIES);

    expect(result[0]).toEqual([
      { task: taskA, type: "scheduled", date: "2024-01-01" },
      { task: taskB, type: "scheduled", date: "2024-01-01" },
      { task: taskC, type: "scheduled", date: "2024-01-01" },
    ]);
  });

  it("sorts scheduled and due bars independently within the same day", () => {
    const scheduledLow = makeTask({ id: "s-low", title: "Zebra", priority: "low", scheduled: "2024-01-01" });
    const scheduledHigh = makeTask({ id: "s-high", title: "Apple", priority: "high", scheduled: "2024-01-01" });
    const dueLow = makeTask({ id: "d-low", title: "Zebra", priority: "low", due: "2024-01-01" });
    const dueHigh = makeTask({ id: "d-high", title: "Apple", priority: "high", due: "2024-01-01" });

    const result = groupTasksByWeek([scheduledLow, scheduledHigh, dueLow, dueHigh], WEEK_DATES, PRIORITIES);

    expect(result[0]).toEqual([
      { task: scheduledHigh, type: "scheduled", date: "2024-01-01" },
      { task: scheduledLow, type: "scheduled", date: "2024-01-01" },
      { task: dueHigh, type: "due", date: "2024-01-01" },
      { task: dueLow, type: "due", date: "2024-01-01" },
    ]);
  });

  it("treats an unrecognized priority as sorting after all recognized priorities", () => {
    const unknown = makeTask({ id: "u", title: "Apple", priority: "urgent", scheduled: "2024-01-01" });
    const low = makeTask({ id: "low", title: "Zebra", priority: "low", scheduled: "2024-01-01" });

    const result = groupTasksByWeek([unknown, low], WEEK_DATES, PRIORITIES);

    expect(result[0]).toEqual([
      { task: low, type: "scheduled", date: "2024-01-01" },
      { task: unknown, type: "scheduled", date: "2024-01-01" },
    ]);
  });
});

describe("compareByPriorityThenTitle", () => {
  it("orders by priority rank ascending when priorities differ", () => {
    const high = makeTask({ id: "high", title: "Zzz", priority: "high" });
    const low = makeTask({ id: "low", title: "Aaa", priority: "low" });

    expect(compareByPriorityThenTitle(high, low, PRIORITIES)).toBeLessThan(0);
    expect(compareByPriorityThenTitle(low, high, PRIORITIES)).toBeGreaterThan(0);
  });

  it("orders alphabetically by title, case-insensitively, when priorities are equal", () => {
    const apple = makeTask({ id: "a", title: "apple", priority: "medium" });
    const banana = makeTask({ id: "b", title: "Banana", priority: "medium" });

    expect(compareByPriorityThenTitle(apple, banana, PRIORITIES)).toBeLessThan(0);
    expect(compareByPriorityThenTitle(banana, apple, PRIORITIES)).toBeGreaterThan(0);
  });

  it("returns 0 for tasks with equal priority and title", () => {
    const a = makeTask({ id: "a", title: "Same", priority: "medium" });
    const b = makeTask({ id: "b", title: "Same", priority: "medium" });

    expect(compareByPriorityThenTitle(a, b, PRIORITIES)).toBe(0);
  });
});

describe("isTaskFinished", () => {
  it("returns true for the done status", () => {
    const task = makeTask({ id: "1", status: "done" });
    expect(isTaskFinished(task, "done", undefined)).toBe(true);
  });

  it("returns true for the cancelled status when set", () => {
    const task = makeTask({ id: "1", status: "cancelled" });
    expect(isTaskFinished(task, "done", "cancelled")).toBe(true);
  });

  it("returns false for any other status", () => {
    const task = makeTask({ id: "1", status: "do" });
    expect(isTaskFinished(task, "done", "cancelled")).toBe(false);
  });

  it("returns false when cancelled status is unset and task status differs from done", () => {
    const task = makeTask({ id: "1", status: "cancelled" });
    expect(isTaskFinished(task, "done", undefined)).toBe(false);
  });
});

describe("groupPreviousWeeksBars", () => {
  const WEEK_START = "2024-01-08";

  it("includes a bar for a scheduled date before the week start", () => {
    const task = makeTask({ id: "1", status: "do", scheduled: "2024-01-05" });
    const result = groupPreviousWeeksBars([task], WEEK_START, PRIORITIES, "done", "cancelled");
    expect(result).toEqual([{ task, type: "scheduled", date: "2024-01-05" }]);
  });

  it("includes a bar for a due date before the week start", () => {
    const task = makeTask({ id: "1", status: "do", due: "2024-01-06" });
    const result = groupPreviousWeeksBars([task], WEEK_START, PRIORITIES, "done", "cancelled");
    expect(result).toEqual([{ task, type: "due", date: "2024-01-06" }]);
  });

  it("includes both bars when scheduled and due both fall before the week start", () => {
    const task = makeTask({ id: "1", status: "do", scheduled: "2024-01-03", due: "2024-01-06" });
    const result = groupPreviousWeeksBars([task], WEEK_START, PRIORITIES, "done", "cancelled");
    expect(result).toEqual([
      { task, type: "scheduled", date: "2024-01-03" },
      { task, type: "due", date: "2024-01-06" },
    ]);
  });

  it("excludes dates on or after the week start", () => {
    const task = makeTask({ id: "1", status: "do", scheduled: "2024-01-08", due: "2024-01-10" });
    const result = groupPreviousWeeksBars([task], WEEK_START, PRIORITIES, "done", "cancelled");
    expect(result).toEqual([]);
  });

  it("excludes done tasks", () => {
    const task = makeTask({ id: "1", status: "done", scheduled: "2024-01-01" });
    const result = groupPreviousWeeksBars([task], WEEK_START, PRIORITIES, "done", "cancelled");
    expect(result).toEqual([]);
  });

  it("excludes cancelled tasks", () => {
    const task = makeTask({ id: "1", status: "cancelled", scheduled: "2024-01-01" });
    const result = groupPreviousWeeksBars([task], WEEK_START, PRIORITIES, "done", "cancelled");
    expect(result).toEqual([]);
  });

  it("sorts bars by date ascending (oldest first), then priority/title", () => {
    const older = makeTask({ id: "older", status: "do", priority: "low", scheduled: "2024-01-01" });
    const newer = makeTask({ id: "newer", status: "do", priority: "high", scheduled: "2024-01-05" });

    const result = groupPreviousWeeksBars([newer, older], WEEK_START, PRIORITIES, "done", "cancelled");

    expect(result).toEqual([
      { task: older, type: "scheduled", date: "2024-01-01" },
      { task: newer, type: "scheduled", date: "2024-01-05" },
    ]);
  });
});

describe("countTasksBeforeWeek", () => {
  const WEEK_START = "2024-01-08";

  it("counts a task with a scheduled date before the week start", () => {
    const task = makeTask({ id: "1", status: "do", scheduled: "2024-01-01" });
    expect(countTasksBeforeWeek([task], WEEK_START, "done", "cancelled")).toBe(1);
  });

  it("counts a task with a due date before the week start", () => {
    const task = makeTask({ id: "1", status: "do", due: "2024-01-01" });
    expect(countTasksBeforeWeek([task], WEEK_START, "done", "cancelled")).toBe(1);
  });

  it("counts a task only once even if both scheduled and due are before the week start", () => {
    const task = makeTask({ id: "1", status: "do", scheduled: "2024-01-01", due: "2024-01-02" });
    expect(countTasksBeforeWeek([task], WEEK_START, "done", "cancelled")).toBe(1);
  });

  it("does not count a task with dates on or after the week start", () => {
    const task = makeTask({ id: "1", status: "do", scheduled: "2024-01-08" });
    expect(countTasksBeforeWeek([task], WEEK_START, "done", "cancelled")).toBe(0);
  });

  it("excludes done and cancelled tasks", () => {
    const tasks = [
      makeTask({ id: "1", status: "done", scheduled: "2024-01-01" }),
      makeTask({ id: "2", status: "cancelled", scheduled: "2024-01-01" }),
    ];
    expect(countTasksBeforeWeek(tasks, WEEK_START, "done", "cancelled")).toBe(0);
  });

  it("returns 0 when no tasks are behind", () => {
    expect(countTasksBeforeWeek([], WEEK_START, "done", "cancelled")).toBe(0);
  });
});
