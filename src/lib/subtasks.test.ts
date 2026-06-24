import { describe, expect, test } from "vitest";
import {
  allSubtasksDone,
  containerOwner,
  effectiveEstimatedMinutes,
  isSubtask,
  relevantSubtasksOf,
  subtaskNameSuggestions,
  subtaskProgress,
  subtasksOf,
} from "./subtasks";
import type { Task } from "./types";

function makeTask(overrides: Partial<Task> & { id: string }): Task {
  return {
    title: "Untitled",
    status: "backlog",
    tags: [],
    priority: "medium",
    order: 1,
    created: "2026-01-01T00:00:00+00:00",
    depends_on: [],
    tracked_minutes: 0,
    hidden: false,
    notes: "",
    ...overrides,
  };
}

describe("isSubtask", () => {
  test("is true when some other task's subtask_project_id matches this task's project_id", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const child = makeTask({ id: "child", project_id: "container" });

    expect(isSubtask(child, [parent, child])).toBe(true);
  });

  test("is false for a task whose project isn't anyone's container", () => {
    const task = makeTask({ id: "task", project_id: "work" });

    expect(isSubtask(task, [task])).toBe(false);
  });

  test("is false for a task with no project_id, even if another task has no container either", () => {
    const orphan = makeTask({ id: "orphan" });
    const other = makeTask({ id: "other" });

    expect(isSubtask(orphan, [orphan, other])).toBe(false);
  });
});

describe("containerOwner", () => {
  test("finds the task that owns the given container project", () => {
    const owner = makeTask({ id: "owner", subtask_project_id: "container" });

    expect(containerOwner("container", [owner])?.id).toBe("owner");
  });

  test("returns undefined when no task owns that container", () => {
    const task = makeTask({ id: "task" });

    expect(containerOwner("container", [task])).toBeUndefined();
  });
});

describe("subtasksOf", () => {
  test("returns every task filed under the parent's container", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const sub1 = makeTask({ id: "sub1", project_id: "container" });
    const sub2 = makeTask({ id: "sub2", project_id: "container" });
    const unrelated = makeTask({ id: "unrelated", project_id: "elsewhere" });

    const result = subtasksOf(parent, [parent, sub1, sub2, unrelated]);

    expect(result.map((t) => t.id).sort()).toEqual(["sub1", "sub2"]);
  });

  test("is empty when the parent has no container yet", () => {
    const parent = makeTask({ id: "parent" });

    expect(subtasksOf(parent, [parent])).toEqual([]);
  });
});

const TODAY = "2026-06-22";

describe("relevantSubtasksOf", () => {
  test("passes through every one-off (non-recurring) subtask unchanged", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const sub1 = makeTask({ id: "sub1", project_id: "container" });
    const sub2 = makeTask({ id: "sub2", project_id: "container", scheduled: "2026-07-01" });

    const result = relevantSubtasksOf(parent, [parent, sub1, sub2], TODAY);

    expect(result.map((t) => t.id).sort()).toEqual(["sub1", "sub2"]);
  });

  test("collapses a recurring subtask's many generated occurrences down to today's one", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const today = makeTask({
      id: "today",
      project_id: "container",
      series_id: "series-1",
      scheduled: TODAY,
    });
    const future1 = makeTask({
      id: "future1",
      project_id: "container",
      series_id: "series-1",
      scheduled: "2026-06-23",
    });
    const future2 = makeTask({
      id: "future2",
      project_id: "container",
      series_id: "series-1",
      scheduled: "2026-08-20",
    });

    const result = relevantSubtasksOf(parent, [parent, today, future1, future2], TODAY);

    expect(result.map((t) => t.id)).toEqual(["today"]);
  });

  test("picks the latest still-due occurrence when more than one qualifies (an overdue catch-up)", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const yesterday = makeTask({
      id: "yesterday",
      project_id: "container",
      series_id: "series-1",
      scheduled: "2026-06-21",
    });
    const today = makeTask({
      id: "today",
      project_id: "container",
      series_id: "series-1",
      scheduled: TODAY,
    });

    const result = relevantSubtasksOf(parent, [parent, yesterday, today], TODAY);

    expect(result.map((t) => t.id)).toEqual(["today"]);
  });

  test("picks the earliest upcoming occurrence when a series hasn't started yet", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const soon = makeTask({
      id: "soon",
      project_id: "container",
      series_id: "series-1",
      scheduled: "2026-06-25",
    });
    const later = makeTask({
      id: "later",
      project_id: "container",
      series_id: "series-1",
      scheduled: "2026-06-30",
    });

    const result = relevantSubtasksOf(parent, [parent, soon, later], TODAY);

    expect(result.map((t) => t.id)).toEqual(["soon"]);
  });

  test("keeps a one-off subtask and a recurring subtask's collapsed occurrence side by side", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const oneOff = makeTask({ id: "one-off", project_id: "container" });
    const recurringToday = makeTask({
      id: "recurring-today",
      project_id: "container",
      series_id: "series-1",
      scheduled: TODAY,
    });
    const recurringFuture = makeTask({
      id: "recurring-future",
      project_id: "container",
      series_id: "series-1",
      scheduled: "2026-07-15",
    });

    const result = relevantSubtasksOf(parent, [parent, oneOff, recurringToday, recurringFuture], TODAY);

    expect(result.map((t) => t.id).sort()).toEqual(["one-off", "recurring-today"]);
  });
});

describe("subtaskProgress", () => {
  test("counts done subtasks out of all active (non-cancelled) ones", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const done1 = makeTask({ id: "done1", project_id: "container", status: "done" });
    const done2 = makeTask({ id: "done2", project_id: "container", status: "done" });
    const pending = makeTask({ id: "pending", project_id: "container", status: "backlog" });

    const result = subtaskProgress(parent, [parent, done1, done2, pending], "done", "cancelled", TODAY);

    expect(result).toEqual({ done: 2, total: 3 });
  });

  test("excludes cancelled subtasks from both done and total", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const done = makeTask({ id: "done", project_id: "container", status: "done" });
    const cancelled = makeTask({ id: "cancelled", project_id: "container", status: "cancelled" });

    const result = subtaskProgress(parent, [parent, done, cancelled], "done", "cancelled", TODAY);

    expect(result).toEqual({ done: 1, total: 1 });
  });

  test("is zero/zero for a parent with no subtasks", () => {
    const parent = makeTask({ id: "parent" });

    expect(subtaskProgress(parent, [parent], "done", "cancelled", TODAY)).toEqual({ done: 0, total: 0 });
  });

  test("counts a recurring subtask once, against today's occurrence, not once per generated occurrence", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const today = makeTask({
      id: "today",
      project_id: "container",
      series_id: "series-1",
      status: "done",
      scheduled: TODAY,
    });
    const future = makeTask({
      id: "future",
      project_id: "container",
      series_id: "series-1",
      status: "backlog",
      scheduled: "2026-08-01",
    });

    const result = subtaskProgress(parent, [parent, today, future], "done", "cancelled", TODAY);

    expect(result).toEqual({ done: 1, total: 1 });
  });
});

describe("allSubtasksDone", () => {
  test("is true when every non-cancelled subtask is done", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const done1 = makeTask({ id: "done1", project_id: "container", status: "done" });
    const cancelled = makeTask({ id: "cancelled", project_id: "container", status: "cancelled" });

    expect(allSubtasksDone(parent, [parent, done1, cancelled], "done", "cancelled", TODAY)).toBe(true);
  });

  test("is false when a non-cancelled subtask is still pending", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const done = makeTask({ id: "done", project_id: "container", status: "done" });
    const pending = makeTask({ id: "pending", project_id: "container", status: "backlog" });

    expect(allSubtasksDone(parent, [parent, done, pending], "done", "cancelled", TODAY)).toBe(false);
  });

  test("is false (not vacuously true) when there are zero non-cancelled subtasks", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const cancelled = makeTask({ id: "cancelled", project_id: "container", status: "cancelled" });

    expect(allSubtasksDone(parent, [parent, cancelled], "done", "cancelled", TODAY)).toBe(false);
  });

  test("is false for a parent with no subtasks at all", () => {
    const parent = makeTask({ id: "parent" });

    expect(allSubtasksDone(parent, [parent], "done", "cancelled", TODAY)).toBe(false);
  });

  test("is true once today's occurrence of a recurring subtask is done, even with un-done future occurrences pre-generated", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const today = makeTask({
      id: "today",
      project_id: "container",
      series_id: "series-1",
      status: "done",
      scheduled: TODAY,
    });
    const future = makeTask({
      id: "future",
      project_id: "container",
      series_id: "series-1",
      status: "backlog",
      scheduled: "2026-08-01",
    });

    expect(allSubtasksDone(parent, [parent, today, future], "done", "cancelled", TODAY)).toBe(true);
  });
});

describe("effectiveEstimatedMinutes", () => {
  test("falls back to the parent's own estimate unchanged when it has no subtasks", () => {
    const parent = makeTask({ id: "parent", estimated_minutes: 90 });

    expect(effectiveEstimatedMinutes(parent, [parent], TODAY, "cancelled", false)).toBe(90);
  });

  test("is undefined when there are no subtasks and the parent has no estimate either", () => {
    const parent = makeTask({ id: "parent" });

    expect(effectiveEstimatedMinutes(parent, [parent], TODAY, "cancelled", false)).toBeUndefined();
  });

  test("sums non-cancelled subtasks' estimates, replacing the parent's own when includeOwnEstimate is false", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container", estimated_minutes: 999 });
    const sub1 = makeTask({ id: "sub1", project_id: "container", estimated_minutes: 30 });
    const sub2 = makeTask({ id: "sub2", project_id: "container", estimated_minutes: 45 });

    expect(effectiveEstimatedMinutes(parent, [parent, sub1, sub2], TODAY, "cancelled", false)).toBe(75);
  });

  test("adds the parent's own estimate on top of the subtasks' sum when includeOwnEstimate is true", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container", estimated_minutes: 60 });
    const sub1 = makeTask({ id: "sub1", project_id: "container", estimated_minutes: 30 });

    expect(effectiveEstimatedMinutes(parent, [parent, sub1], TODAY, "cancelled", true)).toBe(90);
  });

  test("excludes a cancelled subtask's estimate from the sum", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const sub1 = makeTask({ id: "sub1", project_id: "container", estimated_minutes: 30 });
    const cancelled = makeTask({
      id: "cancelled",
      project_id: "container",
      status: "cancelled",
      estimated_minutes: 1000,
    });

    expect(effectiveEstimatedMinutes(parent, [parent, sub1, cancelled], TODAY, "cancelled", false)).toBe(30);
  });

  test("treats a subtask with no estimate of its own as zero in the sum", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const estimated = makeTask({ id: "estimated", project_id: "container", estimated_minutes: 30 });
    const unestimated = makeTask({ id: "unestimated", project_id: "container" });

    expect(effectiveEstimatedMinutes(parent, [parent, estimated, unestimated], TODAY, "cancelled", false)).toBe(30);
  });

  test("is undefined (no misleading 0m badge) when subtasks exist but nothing anywhere has an estimate", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const sub1 = makeTask({ id: "sub1", project_id: "container" });

    expect(effectiveEstimatedMinutes(parent, [parent, sub1], TODAY, "cancelled", false)).toBeUndefined();
  });

  test("sums only the currently-relevant occurrence of a recurring subtask, not every pre-generated one", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const today = makeTask({
      id: "today",
      project_id: "container",
      series_id: "series-1",
      estimated_minutes: 20,
      scheduled: TODAY,
    });
    const future = makeTask({
      id: "future",
      project_id: "container",
      series_id: "series-1",
      estimated_minutes: 20,
      scheduled: "2026-08-01",
    });

    expect(effectiveEstimatedMinutes(parent, [parent, today, future], TODAY, "cancelled", false)).toBe(20);
  });
});

describe("subtaskNameSuggestions", () => {
  test("suggests an active, matching task bare when single-word", () => {
    const task = makeTask({ id: "task", title: "Refactor", status: "backlog" });

    expect(subtaskNameSuggestions([task], "Ref", "done", "cancelled")).toEqual(["Refactor"]);
  });

  test("quotes a matching multi-word title", () => {
    const task = makeTask({ id: "task", title: "Fix the bug", status: "backlog" });

    expect(subtaskNameSuggestions([task], "Fix", "done", "cancelled")).toEqual(['"Fix the bug"']);
  });

  test("excludes done tasks", () => {
    const task = makeTask({ id: "task", title: "Refactor", status: "done" });

    expect(subtaskNameSuggestions([task], "Ref", "done", "cancelled")).toEqual([]);
  });

  test("excludes cancelled tasks", () => {
    const task = makeTask({ id: "task", title: "Refactor", status: "cancelled" });

    expect(subtaskNameSuggestions([task], "Ref", "done", "cancelled")).toEqual([]);
  });

  test("excludes tasks that are themselves subtasks", () => {
    const owner = makeTask({ id: "owner", title: "Parent", subtask_project_id: "container" });
    const subtask = makeTask({ id: "subtask", title: "Refactor", project_id: "container" });

    expect(subtaskNameSuggestions([owner, subtask], "Ref", "done", "cancelled")).toEqual([]);
  });

  test("matches case-insensitively", () => {
    const task = makeTask({ id: "task", title: "Refactor" });

    expect(subtaskNameSuggestions([task], "ref", "done", "cancelled")).toEqual(["Refactor"]);
  });

  test("returns nothing for an empty typed prefix", () => {
    const task = makeTask({ id: "task", title: "Refactor" });

    expect(subtaskNameSuggestions([task], "", "done", "cancelled")).toEqual([]);
  });

  test("deduplicates two tasks sharing the exact same title", () => {
    const task1 = makeTask({ id: "task1", title: "Refactor" });
    const task2 = makeTask({ id: "task2", title: "Refactor" });

    expect(subtaskNameSuggestions([task1, task2], "Ref", "done", "cancelled")).toEqual(["Refactor"]);
  });
});
