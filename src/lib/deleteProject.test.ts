import { describe, expect, test } from "vitest";
import {
  buildTaskStrategy,
  isDefaultProject,
  reassignTargets,
  tasksForProject,
} from "./deleteProject";
import { DEFAULT_PROJECT_COLOR } from "./types";
import type { Project, Task } from "./types";

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: "task-1",
    title: "Untitled",
    status: "backlog",
    tags: [],
    priority: "medium",
    order: 1,
    created: "2026-01-01T00:00:00+00:00",
    depends_on: [],
    notes: "",
    ...overrides,
  };
}

function makeProject(overrides: Partial<Project> = {}): Project {
  return {
    id: "project-1",
    name: "Inbox",
    color: DEFAULT_PROJECT_COLOR,
    order: 1,
    created: "2026-01-01T00:00:00+00:00",
    board: { statuses: [] },
    defaults: { tags: [] },
    ...overrides,
  };
}

describe("isDefaultProject", () => {
  test("returns true for an exact name match", () => {
    expect(isDefaultProject("General", "General")).toBe(true);
  });

  test("matches case-insensitively", () => {
    expect(isDefaultProject("general", "General")).toBe(true);
  });

  test("ignores surrounding whitespace", () => {
    expect(isDefaultProject("  General  ", "General")).toBe(true);
  });

  test("returns false for a different project", () => {
    expect(isDefaultProject("Work", "General")).toBe(false);
  });
});

describe("tasksForProject", () => {
  test("matches tasks case-insensitively", () => {
    const tasks = [
      makeTask({ id: "a", project: "Homework" }),
      makeTask({ id: "b", project: "HOMEWORK" }),
      makeTask({ id: "c", project: "Errands" }),
      makeTask({ id: "d" }),
    ];

    const matching = tasksForProject(tasks, "homework");

    expect(matching.map((t) => t.id)).toEqual(["a", "b"]);
  });

  test("returns an empty array when no tasks match", () => {
    const tasks = [makeTask({ project: "Errands" })];

    expect(tasksForProject(tasks, "Homework")).toEqual([]);
  });

  test("does not match tasks with no project", () => {
    const tasks = [makeTask({ project: undefined })];

    expect(tasksForProject(tasks, "Homework")).toEqual([]);
  });
});

describe("reassignTargets", () => {
  test("excludes the project being deleted", () => {
    const projects = [
      makeProject({ id: "a", name: "Homework" }),
      makeProject({ id: "b", name: "Work" }),
    ];

    const targets = reassignTargets(projects, "a");

    expect(targets.map((p) => p.id)).toEqual(["b"]);
  });

  test("returns an empty array when it is the only project", () => {
    const projects = [makeProject({ id: "a", name: "Homework" })];

    expect(reassignTargets(projects, "a")).toEqual([]);
  });
});

describe("buildTaskStrategy", () => {
  test("builds a reassign strategy with the target project id", () => {
    expect(buildTaskStrategy("reassign", "project-2")).toEqual({
      type: "reassign",
      target_project_id: "project-2",
    });
  });

  test("builds an archive strategy", () => {
    expect(buildTaskStrategy("archive", "")).toEqual({ type: "archive" });
  });

  test("builds a delete strategy", () => {
    expect(buildTaskStrategy("delete", "")).toEqual({ type: "delete" });
  });
});
