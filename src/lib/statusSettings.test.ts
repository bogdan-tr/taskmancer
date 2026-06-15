import { describe, expect, test } from "vitest";
import {
  deleteBlockReason,
  projectsReferencingStatus,
  renumber,
  statusesEqual,
  toggleCancelled,
  toggleDefault,
  toggleDone,
  uniqueId,
} from "./statusSettings";
import { DEFAULT_PROJECT_COLOR, type Project, type StatusDefinition } from "./types";

function makeStatus(overrides: Partial<StatusDefinition> = {}): StatusDefinition {
  return { id: "backlog", label: "Backlog", order: 1, color: "oklch(55% 0.01 270)", ...overrides };
}

function makeProject(overrides: Partial<Project> = {}): Project {
  return {
    id: "p1",
    name: "Homework",
    color: DEFAULT_PROJECT_COLOR,
    order: 1,
    created: "2026-06-10T00:00:00.000Z",
    board: { statuses: [] },
    defaults: { tags: [] },
    ...overrides,
  };
}

describe("statusesEqual", () => {
  test("returns true for two lists with identical statuses in the same order", () => {
    const a = [makeStatus({ id: "backlog", order: 1 }), makeStatus({ id: "done", order: 2 })];
    const b = [makeStatus({ id: "backlog", order: 1 }), makeStatus({ id: "done", order: 2 })];

    expect(statusesEqual(a, b)).toBe(true);
  });

  test("returns false when lengths differ", () => {
    const a = [makeStatus()];
    const b = [makeStatus(), makeStatus({ id: "done" })];

    expect(statusesEqual(a, b)).toBe(false);
  });

  test("returns false when a label differs", () => {
    const a = [makeStatus({ label: "Backlog" })];
    const b = [makeStatus({ label: "Later" })];

    expect(statusesEqual(a, b)).toBe(false);
  });

  test("returns false when a color differs", () => {
    const a = [makeStatus({ color: "oklch(55% 0.01 270)" })];
    const b = [makeStatus({ color: "oklch(50% 0.01 270)" })];

    expect(statusesEqual(a, b)).toBe(false);
  });

  test("returns false when an order differs", () => {
    const a = [makeStatus({ order: 1 })];
    const b = [makeStatus({ order: 2 })];

    expect(statusesEqual(a, b)).toBe(false);
  });

  test("returns false when order of entries differs", () => {
    const a = [makeStatus({ id: "backlog" }), makeStatus({ id: "done" })];
    const b = [makeStatus({ id: "done" }), makeStatus({ id: "backlog" })];

    expect(statusesEqual(a, b)).toBe(false);
  });
});

describe("renumber", () => {
  test("assigns order as each status's 1-based position", () => {
    const statuses = [makeStatus({ id: "a", order: 9 }), makeStatus({ id: "b", order: 1 })];

    const result = renumber(statuses);

    expect(result.map((status) => status.order)).toEqual([1, 2]);
  });

  test("does not mutate the input statuses", () => {
    const statuses = [makeStatus({ id: "a", order: 9 })];
    const before = makeStatus({ id: "a", order: 9 });

    renumber(statuses);

    expect(statuses[0]).toEqual(before);
  });

  test("returns an empty array for an empty input", () => {
    expect(renumber([])).toEqual([]);
  });
});

describe("uniqueId", () => {
  test("returns the base id when it isn't already used", () => {
    expect(uniqueId(["backlog", "done"], "on-hold")).toBe("on-hold");
  });

  test("appends -2 when the base id is already used", () => {
    expect(uniqueId(["new-status"], "new-status")).toBe("new-status-2");
  });

  test("skips suffixes until an unused id is found", () => {
    expect(uniqueId(["new-status", "new-status-2", "new-status-3"], "new-status")).toBe("new-status-4");
  });
});

describe("projectsReferencingStatus", () => {
  test("returns names of projects whose board.statuses includes the status id", () => {
    const projects = [
      makeProject({ name: "Homework", board: { statuses: ["backlog", "done"] } }),
      makeProject({ name: "Garden", board: { statuses: ["done"] } }),
    ];

    expect(projectsReferencingStatus(projects, "backlog")).toEqual(["Homework"]);
  });

  test("returns names of projects whose board.default_status is the status id", () => {
    const projects = [
      makeProject({ name: "Homework", board: { statuses: [], default_status: "on-hold" } }),
    ];

    expect(projectsReferencingStatus(projects, "on-hold")).toEqual(["Homework"]);
  });

  test("returns an empty array when no project references the status id", () => {
    const projects = [makeProject({ name: "Homework", board: { statuses: ["done"] } })];

    expect(projectsReferencingStatus(projects, "backlog")).toEqual([]);
  });

  test("includes a project only once even if it matches both statuses and default_status", () => {
    const projects = [
      makeProject({ name: "Homework", board: { statuses: ["backlog"], default_status: "backlog" } }),
    ];

    expect(projectsReferencingStatus(projects, "backlog")).toEqual(["Homework"]);
  });
});

describe("deleteBlockReason", () => {
  test("blocks deletion of the last remaining status", () => {
    const status = makeStatus({ id: "backlog" });

    expect(deleteBlockReason(status, 1, "do", "done", undefined, {}, [])).toBe(
      "At least one status is required",
    );
  });

  test("blocks deletion of the last remaining status even when it is the default", () => {
    const status = makeStatus({ id: "backlog" });

    expect(deleteBlockReason(status, 1, "backlog", "done", undefined, {}, [])).toBe(
      "At least one status is required",
    );
  });

  test("blocks deletion of the default status", () => {
    const status = makeStatus({ id: "backlog" });

    expect(deleteBlockReason(status, 2, "backlog", "done", undefined, {}, [])).toBe(
      "This is the default status and can't be deleted",
    );
  });

  test("blocks deletion of the done status", () => {
    const status = makeStatus({ id: "backlog" });

    expect(deleteBlockReason(status, 2, "do", "backlog", undefined, {}, [])).toBe(
      "This is the Done status and can't be deleted",
    );
  });

  test("blocks deletion of the cancelled status", () => {
    const status = makeStatus({ id: "backlog" });

    expect(deleteBlockReason(status, 2, "do", "done", "backlog", {}, [])).toBe(
      "This is the Cancelled status and can't be deleted",
    );
  });

  test("blocks deletion when tasks use this status, with singular wording for one task", () => {
    const status = makeStatus({ id: "backlog" });

    expect(deleteBlockReason(status, 2, "do", "done", undefined, { backlog: 1 }, [])).toBe(
      "1 task uses this status — reassign them first",
    );
  });

  test("blocks deletion when tasks use this status, with plural wording for multiple tasks", () => {
    const status = makeStatus({ id: "backlog" });

    expect(deleteBlockReason(status, 2, "do", "done", undefined, { backlog: 3 }, [])).toBe(
      "3 tasks use this status — reassign them first",
    );
  });

  test("blocks deletion when a single project's board references this status", () => {
    const status = makeStatus({ id: "backlog" });

    expect(deleteBlockReason(status, 2, "do", "done", undefined, {}, ["Homework"])).toBe(
      'Used by project "Homework" — update its board first',
    );
  });

  test("blocks deletion when multiple projects' boards reference this status", () => {
    const status = makeStatus({ id: "backlog" });

    expect(deleteBlockReason(status, 2, "do", "done", undefined, {}, ["Homework", "Garden"])).toBe(
      'Used by projects "Homework", "Garden" — update their boards first',
    );
  });

  test("prioritizes the task-count reason over the project-reference reason", () => {
    const status = makeStatus({ id: "backlog" });

    expect(deleteBlockReason(status, 2, "do", "done", undefined, { backlog: 1 }, ["Homework"])).toBe(
      "1 task uses this status — reassign them first",
    );
  });

  test("allows deletion when not the default, no tasks use it, and no project references it", () => {
    const status = makeStatus({ id: "backlog" });

    expect(deleteBlockReason(status, 2, "do", "done", undefined, {}, [])).toBeUndefined();
  });

  test("allows deletion when taskCounts has no entry for this status", () => {
    const status = makeStatus({ id: "backlog" });

    expect(deleteBlockReason(status, 2, "do", "done", undefined, {}, [])).toBeUndefined();
  });
});

describe("toggleDefault", () => {
  test("sets the clicked status as the default when none was set", () => {
    expect(toggleDefault(undefined, "backlog")).toBe("backlog");
  });

  test("sets the clicked status as the default, replacing the previous default", () => {
    expect(toggleDefault("backlog", "do")).toBe("do");
  });

  test("clears the default when the current default is clicked again", () => {
    expect(toggleDefault("backlog", "backlog")).toBeUndefined();
  });
});

describe("toggleDone", () => {
  test("sets the clicked status as done, replacing the previous done status", () => {
    expect(toggleDone({ doneId: "backlog", cancelledId: undefined }, "do")).toEqual({
      doneId: "do",
      cancelledId: undefined,
    });
  });

  test("is a no-op when the clicked status is already the done status", () => {
    expect(toggleDone({ doneId: "backlog", cancelledId: undefined }, "backlog")).toEqual({
      doneId: "backlog",
      cancelledId: undefined,
    });
  });

  test("clears the cancelled status if the newly-done status was previously cancelled", () => {
    expect(toggleDone({ doneId: "backlog", cancelledId: "do" }, "do")).toEqual({
      doneId: "do",
      cancelledId: undefined,
    });
  });

  test("leaves an unrelated cancelled status untouched", () => {
    expect(toggleDone({ doneId: "backlog", cancelledId: "on-hold" }, "do")).toEqual({
      doneId: "do",
      cancelledId: "on-hold",
    });
  });
});

describe("toggleCancelled", () => {
  test("sets the clicked status as cancelled when none was set", () => {
    expect(toggleCancelled(undefined, "done", "blocked")).toBe("blocked");
  });

  test("clears cancelled when the current cancelled status is clicked again", () => {
    expect(toggleCancelled("blocked", "done", "blocked")).toBeUndefined();
  });

  test("replaces the previous cancelled status with the newly clicked one", () => {
    expect(toggleCancelled("blocked", "done", "on-hold")).toBe("on-hold");
  });

  test("is a no-op when the clicked status is the done status", () => {
    expect(toggleCancelled("blocked", "done", "done")).toBe("blocked");
    expect(toggleCancelled(undefined, "done", "done")).toBeUndefined();
  });
});
