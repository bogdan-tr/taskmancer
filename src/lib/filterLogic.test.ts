import { describe, expect, it } from "vitest";
import { applyFilter, applySort, isFilterActive } from "./filterLogic";
import { DEFAULT_FILTER_CONFIG } from "./filterViewState.svelte";
import type { FilterConfig, PriorityLevel, SortConfig, StatusDefinition, Task } from "./types";

// ── Helpers ────────────────────────────────────────────────────────────────────

function makeTask(overrides: Partial<Task> & { id: string }): Task {
  return {
    title: "Untitled",
    status: "todo",
    project_id: undefined,
    tags: [],
    priority: "medium",
    due: undefined,
    scheduled: undefined,
    order: 0,
    created: "2024-01-01T00:00:00Z",
    depends_on: [],
    tracked_minutes: 0,
    hidden: false,
    notes: "",
    ...overrides,
  };
}

function makeConfig(overrides: Partial<FilterConfig> = {}): FilterConfig {
  return { ...DEFAULT_FILTER_CONFIG, ...overrides };
}

const PRIORITIES: PriorityLevel[] = [
  { id: "high", label: "High", color: "#ef4444", rank: 1 },
  { id: "medium", label: "Medium", color: "#f97316", rank: 2 },
  { id: "low", label: "Low", color: "#3b82f6", rank: 3 },
];

const STATUSES: StatusDefinition[] = [
  { id: "todo", label: "To Do", color: "#6b7280", order: 0 },
  { id: "in_progress", label: "In Progress", color: "#3b82f6", order: 1 },
  { id: "done", label: "Done", color: "#22c55e", order: 2 },
];

// ── applyFilter ────────────────────────────────────────────────────────────────

describe("applyFilter", () => {
  it("returns all tasks when config is default (no active groups)", () => {
    const tasks = [makeTask({ id: "1" }), makeTask({ id: "2" })];
    expect(applyFilter(tasks, makeConfig())).toHaveLength(2);
  });

  it("always excludes hidden tasks", () => {
    const tasks = [makeTask({ id: "1", hidden: true }), makeTask({ id: "2" })];
    expect(applyFilter(tasks, makeConfig())).toHaveLength(1);
  });

  // Text search
  it("filters by title text (case-insensitive)", () => {
    const tasks = [makeTask({ id: "1", title: "Buy Milk" }), makeTask({ id: "2", title: "Read book" })];
    expect(applyFilter(tasks, makeConfig({ text: "milk" }))).toHaveLength(1);
  });

  it("filters by notes when titleOnly is false", () => {
    const tasks = [
      makeTask({ id: "1", title: "Task A", notes: "contains keyword" }),
      makeTask({ id: "2", title: "Task B" }),
    ];
    const result = applyFilter(tasks, makeConfig({ text: "keyword", titleOnly: false }));
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("1");
  });

  it("does not search notes when titleOnly is true", () => {
    const tasks = [
      makeTask({ id: "1", title: "Task A", notes: "contains keyword" }),
      makeTask({ id: "2", title: "Task B" }),
    ];
    expect(applyFilter(tasks, makeConfig({ text: "keyword", titleOnly: true }))).toHaveLength(0);
  });

  // Status
  it("filters by status", () => {
    const tasks = [
      makeTask({ id: "1", status: "todo" }),
      makeTask({ id: "2", status: "done" }),
    ];
    expect(applyFilter(tasks, makeConfig({ statuses: ["todo"] }))).toHaveLength(1);
  });

  it("matches any of the selected statuses", () => {
    const tasks = [
      makeTask({ id: "1", status: "todo" }),
      makeTask({ id: "2", status: "in_progress" }),
      makeTask({ id: "3", status: "done" }),
    ];
    const result = applyFilter(tasks, makeConfig({ statuses: ["todo", "done"] }));
    expect(result).toHaveLength(2);
  });

  // Project
  it("filters by project id", () => {
    const tasks = [
      makeTask({ id: "1", project_id: "proj-a" }),
      makeTask({ id: "2", project_id: "proj-b" }),
      makeTask({ id: "3", project_id: undefined }),
    ];
    const result = applyFilter(tasks, makeConfig({ projectIds: ["proj-a"] }));
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("1");
  });

  it("excludes tasks with no project when project filter is active", () => {
    const tasks = [makeTask({ id: "1" })]; // no project_id
    expect(applyFilter(tasks, makeConfig({ projectIds: ["proj-x"] }))).toHaveLength(0);
  });

  // Tags — any mode
  it("matches tasks that have any of the selected tags (tagMode: any)", () => {
    const tasks = [
      makeTask({ id: "1", tags: ["work", "urgent"] }),
      makeTask({ id: "2", tags: ["personal"] }),
      makeTask({ id: "3", tags: [] }),
    ];
    const result = applyFilter(tasks, makeConfig({ tags: ["work", "personal"], tagMode: "any" }));
    expect(result).toHaveLength(2);
  });

  // Tags — all mode
  it("matches only tasks that have all selected tags (tagMode: all)", () => {
    const tasks = [
      makeTask({ id: "1", tags: ["work", "urgent"] }),
      makeTask({ id: "2", tags: ["work"] }),
    ];
    const result = applyFilter(tasks, makeConfig({ tags: ["work", "urgent"], tagMode: "all" }));
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("1");
  });

  // Priority
  it("filters by priority", () => {
    const tasks = [
      makeTask({ id: "1", priority: "high" }),
      makeTask({ id: "2", priority: "low" }),
    ];
    expect(applyFilter(tasks, makeConfig({ priorities: ["high"] }))).toHaveLength(1);
  });

  // Estimate range
  it("filters by minimum estimate hours", () => {
    const tasks = [
      makeTask({ id: "1", estimated_minutes: 30 }),  // 0.5h
      makeTask({ id: "2", estimated_minutes: 120 }), // 2h
    ];
    const result = applyFilter(tasks, makeConfig({ estimateMinHours: 1 }));
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("2");
  });

  it("filters by maximum estimate hours", () => {
    const tasks = [
      makeTask({ id: "1", estimated_minutes: 30 }),  // 0.5h
      makeTask({ id: "2", estimated_minutes: 120 }), // 2h
    ];
    const result = applyFilter(tasks, makeConfig({ estimateMaxHours: 1 }));
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("1");
  });

  it("excludes tasks with no estimate when estimate filter is active", () => {
    const tasks = [makeTask({ id: "1" })]; // no estimated_minutes
    expect(applyFilter(tasks, makeConfig({ estimateMinHours: 1 }))).toHaveLength(0);
  });

  // Tracked time
  it("filters by minimum tracked time", () => {
    const tasks = [
      makeTask({ id: "1", tracked_minutes: 10 }),
      makeTask({ id: "2", tracked_minutes: 90 }),
    ];
    expect(applyFilter(tasks, makeConfig({ trackedMinHours: 1 }))).toHaveLength(1);
  });

  // Has subtasks
  it("filters tasks with subtasks (hasSubtasks: yes)", () => {
    const tasks = [
      makeTask({ id: "1", subtask_project_id: "sub-proj" }),
      makeTask({ id: "2" }),
    ];
    expect(applyFilter(tasks, makeConfig({ hasSubtasks: "yes" }))).toHaveLength(1);
  });

  it("filters tasks without subtasks (hasSubtasks: no)", () => {
    const tasks = [
      makeTask({ id: "1", subtask_project_id: "sub-proj" }),
      makeTask({ id: "2" }),
    ];
    expect(applyFilter(tasks, makeConfig({ hasSubtasks: "no" }))).toHaveLength(1);
  });

  // Is recurring
  it("filters recurring tasks (isRecurring: yes)", () => {
    const tasks = [
      makeTask({ id: "1", series_id: "s1" }),
      makeTask({ id: "2" }),
    ];
    expect(applyFilter(tasks, makeConfig({ isRecurring: "yes" }))).toHaveLength(1);
  });

  it("filters non-recurring tasks (isRecurring: no)", () => {
    const tasks = [
      makeTask({ id: "1", series_id: "s1" }),
      makeTask({ id: "2" }),
    ];
    expect(applyFilter(tasks, makeConfig({ isRecurring: "no" }))).toHaveLength(1);
  });

  // Due date filter
  it("filters overdue tasks", () => {
    const past = "2000-01-01";
    const future = "2099-12-31";
    const tasks = [
      makeTask({ id: "1", due: past }),
      makeTask({ id: "2", due: future }),
      makeTask({ id: "3" }), // no due date
    ];
    const result = applyFilter(tasks, makeConfig({ dueFilter: { type: "overdue" } }));
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("1");
  });

  it("excludes tasks with no due date when due filter is active", () => {
    const tasks = [makeTask({ id: "1" })];
    expect(applyFilter(tasks, makeConfig({ dueFilter: { type: "overdue" } }))).toHaveLength(0);
  });

  it("filters by custom due date range", () => {
    const tasks = [
      makeTask({ id: "1", due: "2024-03-01" }),
      makeTask({ id: "2", due: "2024-06-15" }),
      makeTask({ id: "3", due: "2024-09-01" }),
    ];
    const result = applyFilter(tasks, makeConfig({
      dueFilter: { type: "custom", from: "2024-04-01", to: "2024-07-31" },
    }));
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("2");
  });

  // groupMode: OR
  it("returns task matching any active group when groupMode is 'any'", () => {
    const tasks = [
      makeTask({ id: "1", status: "todo", priority: "low" }),
      makeTask({ id: "2", status: "done", priority: "high" }),
      makeTask({ id: "3", status: "done", priority: "low" }),
    ];
    // status=todo OR priority=high
    const result = applyFilter(tasks, makeConfig({
      statuses: ["todo"],
      priorities: ["high"],
      groupMode: "any",
    }));
    expect(result).toHaveLength(2); // id 1 (status match) and id 2 (priority match)
    expect(result.map((t) => t.id)).toEqual(expect.arrayContaining(["1", "2"]));
  });

  // groupMode: AND
  it("returns only tasks matching all active groups when groupMode is 'all'", () => {
    const tasks = [
      makeTask({ id: "1", status: "todo", priority: "low" }),
      makeTask({ id: "2", status: "done", priority: "high" }),
      makeTask({ id: "3", status: "todo", priority: "high" }),
    ];
    // status=todo AND priority=high
    const result = applyFilter(tasks, makeConfig({
      statuses: ["todo"],
      priorities: ["high"],
      groupMode: "all",
    }));
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("3");
  });

  // Edge cases
  it("returns empty array for empty task list", () => {
    expect(applyFilter([], makeConfig({ text: "anything" }))).toHaveLength(0);
  });

  it("handles task with empty tags array when tag filter is set", () => {
    const tasks = [makeTask({ id: "1", tags: [] })];
    expect(applyFilter(tasks, makeConfig({ tags: ["work"] }))).toHaveLength(0);
  });
});

// ── applySort ──────────────────────────────────────────────────────────────────

describe("applySort", () => {
  it("returns a copy without modifying original when no sort levels", () => {
    const tasks = [makeTask({ id: "1" }), makeTask({ id: "2" })];
    const sort: SortConfig = { levels: [] };
    const result = applySort(tasks, sort, PRIORITIES, STATUSES);
    expect(result).not.toBe(tasks); // new array
    expect(result).toHaveLength(2);
  });

  it("sorts by title ascending", () => {
    const tasks = [
      makeTask({ id: "1", title: "Zebra" }),
      makeTask({ id: "2", title: "Apple" }),
      makeTask({ id: "3", title: "Mango" }),
    ];
    const sort: SortConfig = { levels: [{ key: "title", direction: "asc" }] };
    const result = applySort(tasks, sort, PRIORITIES, STATUSES);
    expect(result.map((t) => t.title)).toEqual(["Apple", "Mango", "Zebra"]);
  });

  it("sorts by title descending", () => {
    const tasks = [
      makeTask({ id: "1", title: "Apple" }),
      makeTask({ id: "2", title: "Zebra" }),
    ];
    const sort: SortConfig = { levels: [{ key: "title", direction: "desc" }] };
    const result = applySort(tasks, sort, PRIORITIES, STATUSES);
    expect(result[0].title).toBe("Zebra");
  });

  it("sorts by priority rank ascending", () => {
    const tasks = [
      makeTask({ id: "1", priority: "low" }),
      makeTask({ id: "2", priority: "high" }),
      makeTask({ id: "3", priority: "medium" }),
    ];
    const sort: SortConfig = { levels: [{ key: "priority", direction: "asc" }] };
    const result = applySort(tasks, sort, PRIORITIES, STATUSES);
    expect(result.map((t) => t.priority)).toEqual(["high", "medium", "low"]);
  });

  it("sorts by due date with nulls last", () => {
    const tasks = [
      makeTask({ id: "1", due: "2024-06-01" }),
      makeTask({ id: "2" }), // no due
      makeTask({ id: "3", due: "2024-01-01" }),
    ];
    const sort: SortConfig = { levels: [{ key: "due", direction: "asc" }] };
    const result = applySort(tasks, sort, PRIORITIES, STATUSES);
    expect(result.map((t) => t.id)).toEqual(["3", "1", "2"]);
  });

  it("applies secondary sort level as tie-breaker", () => {
    const tasks = [
      makeTask({ id: "1", priority: "high", title: "Zebra" }),
      makeTask({ id: "2", priority: "high", title: "Apple" }),
      makeTask({ id: "3", priority: "low", title: "Mango" }),
    ];
    const sort: SortConfig = {
      levels: [
        { key: "priority", direction: "asc" },
        { key: "title", direction: "asc" },
      ],
    };
    const result = applySort(tasks, sort, PRIORITIES, STATUSES);
    expect(result.map((t) => t.id)).toEqual(["2", "1", "3"]);
  });

  it("uses order field as final tie-breaker", () => {
    const tasks = [
      makeTask({ id: "1", title: "Same", order: 200 }),
      makeTask({ id: "2", title: "Same", order: 100 }),
    ];
    const sort: SortConfig = { levels: [{ key: "title", direction: "asc" }] };
    const result = applySort(tasks, sort, PRIORITIES, STATUSES);
    expect(result.map((t) => t.id)).toEqual(["2", "1"]);
  });

  it("sorts by tracked time", () => {
    const tasks = [
      makeTask({ id: "1", tracked_minutes: 90 }),
      makeTask({ id: "2", tracked_minutes: 10 }),
    ];
    const sort: SortConfig = { levels: [{ key: "tracked_time", direction: "asc" }] };
    const result = applySort(tasks, sort, PRIORITIES, STATUSES);
    expect(result[0].id).toBe("2");
  });

  it("sorts by estimated time", () => {
    const tasks = [
      makeTask({ id: "1", estimated_minutes: 120 }),
      makeTask({ id: "2", estimated_minutes: 30 }),
    ];
    const sort: SortConfig = { levels: [{ key: "estimated_time", direction: "desc" }] };
    const result = applySort(tasks, sort, PRIORITIES, STATUSES);
    expect(result[0].id).toBe("1");
  });

  it("does not mutate the original array", () => {
    const tasks = [makeTask({ id: "1", title: "B" }), makeTask({ id: "2", title: "A" })];
    const original = [...tasks];
    const sort: SortConfig = { levels: [{ key: "title", direction: "asc" }] };
    applySort(tasks, sort, PRIORITIES, STATUSES);
    expect(tasks[0].title).toBe(original[0].title); // unchanged
  });
});

// ── isFilterActive ─────────────────────────────────────────────────────────────

describe("isFilterActive", () => {
  it("returns false for default config", () => {
    expect(isFilterActive(makeConfig())).toBe(false);
  });

  it("returns true when text is set", () => {
    expect(isFilterActive(makeConfig({ text: "hello" }))).toBe(true);
  });

  it("returns false when text is only whitespace", () => {
    expect(isFilterActive(makeConfig({ text: "   " }))).toBe(false);
  });

  it("returns true when statuses are selected", () => {
    expect(isFilterActive(makeConfig({ statuses: ["todo"] }))).toBe(true);
  });

  it("returns true when projects are selected", () => {
    expect(isFilterActive(makeConfig({ projectIds: ["p1"] }))).toBe(true);
  });

  it("returns true when tags are set", () => {
    expect(isFilterActive(makeConfig({ tags: ["work"] }))).toBe(true);
  });

  it("returns true when dueFilter is not 'any'", () => {
    expect(isFilterActive(makeConfig({ dueFilter: { type: "today" } }))).toBe(true);
  });

  it("returns true when scheduledFilter is not 'any'", () => {
    expect(isFilterActive(makeConfig({ scheduledFilter: { type: "this_week" } }))).toBe(true);
  });

  it("returns true when priorities are selected", () => {
    expect(isFilterActive(makeConfig({ priorities: ["high"] }))).toBe(true);
  });

  it("returns true when estimateMinHours is set", () => {
    expect(isFilterActive(makeConfig({ estimateMinHours: 2 }))).toBe(true);
  });

  it("returns true when estimateMaxHours is set", () => {
    expect(isFilterActive(makeConfig({ estimateMaxHours: 4 }))).toBe(true);
  });

  it("returns true when trackedMinHours is set", () => {
    expect(isFilterActive(makeConfig({ trackedMinHours: 1 }))).toBe(true);
  });

  it("returns true when trackedMaxHours is set", () => {
    expect(isFilterActive(makeConfig({ trackedMaxHours: 8 }))).toBe(true);
  });

  it("returns true when hasSubtasks is not 'any'", () => {
    expect(isFilterActive(makeConfig({ hasSubtasks: "yes" }))).toBe(true);
  });

  it("returns true when isRecurring is not 'any'", () => {
    expect(isFilterActive(makeConfig({ isRecurring: "no" }))).toBe(true);
  });

  it("returns true when isArchived is not 'active_only'", () => {
    expect(isFilterActive(makeConfig({ isArchived: "both" }))).toBe(true);
  });
});
