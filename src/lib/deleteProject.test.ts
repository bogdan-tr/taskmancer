import { describe, expect, test } from "vitest";
import {
  buildTaskStrategy,
  isDefaultProject,
  reassignTargets,
  tasksForProject,
  tasksForProjects,
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
    tracked_minutes: 0,
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
  test("returns true for a matching id", () => {
    expect(isDefaultProject("general-id", "general-id")).toBe(true);
  });

  test("returns false for a different project", () => {
    expect(isDefaultProject("work-id", "general-id")).toBe(false);
  });
});

describe("tasksForProject", () => {
  test("matches tasks filed under the given project id", () => {
    const tasks = [
      makeTask({ id: "a", project_id: "homework-id" }),
      makeTask({ id: "b", project_id: "errands-id" }),
      makeTask({ id: "c" }),
    ];

    const matching = tasksForProject(tasks, "homework-id");

    expect(matching.map((t) => t.id)).toEqual(["a"]);
  });

  test("returns an empty array when no tasks match", () => {
    const tasks = [makeTask({ project_id: "errands-id" })];

    expect(tasksForProject(tasks, "homework-id")).toEqual([]);
  });

  test("does not match tasks with no project", () => {
    const tasks = [makeTask({ project_id: undefined })];

    expect(tasksForProject(tasks, "homework-id")).toEqual([]);
  });
});

describe("tasksForProjects", () => {
  test("matches tasks belonging to any of several project ids", () => {
    const tasks = [
      makeTask({ id: "1", project_id: "a" }),
      makeTask({ id: "2", project_id: "b" }),
      makeTask({ id: "3", project_id: "c" }),
    ];

    const matching = tasksForProjects(tasks, ["a", "b"]);

    expect(matching.map((t) => t.id)).toEqual(["1", "2"]);
  });

  test("returns an empty array when no task matches", () => {
    const tasks = [makeTask({ id: "1", project_id: "a" })];

    expect(tasksForProjects(tasks, ["z"])).toEqual([]);
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

  test("excludes the project being deleted and its descendants", () => {
    const parent = makeProject({ id: "parent", name: "Parent" });
    const child = makeProject({ id: "child", name: "Child", parent_id: "parent" });
    const unrelated = makeProject({ id: "unrelated", name: "Unrelated" });
    const projects = [parent, child, unrelated];

    const targets = reassignTargets(projects, "parent");

    expect(targets.map((p) => p.id)).toEqual(["unrelated"]);
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
