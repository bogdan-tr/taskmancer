import { describe, expect, it } from "vitest";
import {
  ancestorsOf,
  childrenOf,
  computeZoneOrderUpdates,
  descendantsOf,
  findProjectByPath,
  projectPath,
  selfAndAncestors,
  wouldCreateCycle,
} from "./projectTree";
import type { Project } from "./types";

/**
 * Builds a small fixture tree:
 * ```text
 * root_a (top-level)
 * ├── child_a1
 * │   └── grandchild_a1a
 * └── child_a2
 * root_b (top-level, no children)
 * ```
 */
function fixtureTree(): Project[] {
  const base = (id: string, name: string, parentId: string | undefined, order: number): Project => ({
    id,
    name,
    color: "#111111",
    parent_id: parentId,
    order,
    created: "2026-06-11T10:00:00+00:00",
    board: { statuses: [] },
    defaults: { tags: [] },
  });

  return [
    base("root_a", "Root A", undefined, 1),
    base("root_b", "Root B", undefined, 2),
    base("child_a1", "Child A1", "root_a", 1),
    base("child_a2", "Child A2", "root_a", 2),
    base("grandchild_a1a", "Grandchild A1a", "child_a1", 1),
  ];
}

describe("childrenOf", () => {
  it("returns top-level projects when parentId is undefined", () => {
    const projects = fixtureTree();

    const children = childrenOf(projects, undefined);

    expect(children.map((p) => p.id)).toEqual(["root_a", "root_b"]);
  });

  it("returns direct children only", () => {
    const projects = fixtureTree();

    const children = childrenOf(projects, "root_a");

    expect(children.map((p) => p.id)).toEqual(["child_a1", "child_a2"]);
  });

  it("returns empty for a leaf", () => {
    const projects = fixtureTree();

    expect(childrenOf(projects, "child_a2")).toEqual([]);
  });
});

describe("ancestorsOf", () => {
  it("is empty for a top-level project", () => {
    const projects = fixtureTree();

    expect(ancestorsOf(projects, "root_a")).toEqual([]);
  });

  it("returns ancestors nearest-first", () => {
    const projects = fixtureTree();

    const ancestors = ancestorsOf(projects, "grandchild_a1a");

    expect(ancestors.map((p) => p.id)).toEqual(["child_a1", "root_a"]);
  });

  it("is empty for a missing id", () => {
    const projects = fixtureTree();

    expect(ancestorsOf(projects, "does-not-exist")).toEqual([]);
  });

  it("stops at a dangling parent_id", () => {
    const projects = fixtureTree();
    projects[2] = { ...projects[2], parent_id: "deleted-project" };

    const ancestors = ancestorsOf(projects, "grandchild_a1a");

    expect(ancestors.map((p) => p.id)).toEqual(["child_a1"]);
  });
});

describe("selfAndAncestors", () => {
  it("includes self first", () => {
    const projects = fixtureTree();

    const chain = selfAndAncestors(projects, "grandchild_a1a");

    expect(chain.map((p) => p.id)).toEqual(["grandchild_a1a", "child_a1", "root_a"]);
  });

  it("is empty for a missing id", () => {
    const projects = fixtureTree();

    expect(selfAndAncestors(projects, "does-not-exist")).toEqual([]);
  });
});

describe("descendantsOf", () => {
  it("returns all levels", () => {
    const projects = fixtureTree();

    const descendants = descendantsOf(projects, "root_a");

    expect(descendants.map((p) => p.id).sort()).toEqual(["child_a1", "child_a2", "grandchild_a1a"]);
  });

  it("is empty for a leaf", () => {
    const projects = fixtureTree();

    expect(descendantsOf(projects, "grandchild_a1a")).toEqual([]);
  });

  it("is empty for an unrelated root", () => {
    const projects = fixtureTree();

    expect(descendantsOf(projects, "root_b")).toEqual([]);
  });
});

describe("wouldCreateCycle", () => {
  it("is true when moving under self", () => {
    const projects = fixtureTree();

    expect(wouldCreateCycle(projects, "root_a", "root_a")).toBe(true);
  });

  it("is true when moving under own descendant", () => {
    const projects = fixtureTree();

    expect(wouldCreateCycle(projects, "root_a", "grandchild_a1a")).toBe(true);
  });

  it("is false when moving to an unrelated project", () => {
    const projects = fixtureTree();

    expect(wouldCreateCycle(projects, "child_a1", "root_b")).toBe(false);
  });
});

describe("computeZoneOrderUpdates", () => {
  function project(id: string, parentId: string | undefined, order: number): Project {
    return {
      id,
      name: id,
      color: "#111111",
      parent_id: parentId,
      order,
      created: "2026-06-11T10:00:00+00:00",
      board: { statuses: [] },
      defaults: { tags: [] },
    };
  }

  it("returns no updates when a zone is already correctly ordered", () => {
    const a = project("a", "parent", 1000);
    const b = project("b", "parent", 2000);
    const allProjects = [a, b];

    const { updates, rejected } = computeZoneOrderUpdates(allProjects, "parent", [a, b]);

    expect(rejected).toBe(false);
    expect(updates).toEqual([]);
  });

  it("returns an update for a reordered item within the same zone", () => {
    const a = project("a", "parent", 1000);
    const b = project("b", "parent", 2000);
    const allProjects = [a, b];

    const { updates, rejected } = computeZoneOrderUpdates(allProjects, "parent", [b, a]);

    expect(rejected).toBe(false);
    expect(updates).toEqual([
      { id: "b", parent_id: "parent", order: 1000 },
      { id: "a", parent_id: "parent", order: 2000 },
    ]);
  });

  it("returns a reparent update for an item moved in from a different zone", () => {
    const moved = project("moved", "old-parent", 1000);
    const sibling = project("sibling", "new-parent", 1000);
    const allProjects = [moved, sibling];

    const { updates, rejected } = computeZoneOrderUpdates(allProjects, "new-parent", [sibling, moved]);

    expect(rejected).toBe(false);
    expect(updates).toContainEqual({ id: "moved", parent_id: "new-parent", order: 2000 });
  });

  it("rejects moving a project into one of its own descendants", () => {
    const parent = project("parent", undefined, 1000);
    const child = project("child", "parent", 1000);
    const allProjects = [parent, child];

    const { rejected } = computeZoneOrderUpdates(allProjects, "child", [parent]);

    expect(rejected).toBe(true);
  });

  it("treats undefined zoneParentId (top-level) as never a cycle risk", () => {
    const a = project("a", undefined, 1000);
    const allProjects = [a];

    const { rejected } = computeZoneOrderUpdates(allProjects, undefined, [a]);

    expect(rejected).toBe(false);
  });
});

describe("projectPath", () => {
  it("returns just the name for a top-level project", () => {
    const projects = fixtureTree();

    expect(projectPath(projects, "root_a")).toBe("Root A");
  });

  it("returns the root-first ancestor path for a nested project", () => {
    const projects = fixtureTree();

    expect(projectPath(projects, "grandchild_a1a")).toBe("Root A/Child A1/Grandchild A1a");
  });

  it("returns an empty string for a missing id", () => {
    const projects = fixtureTree();

    expect(projectPath(projects, "does-not-exist")).toBe("");
  });
});

describe("findProjectByPath", () => {
  it("resolves a single-segment path to a top-level project", () => {
    const projects = fixtureTree();

    expect(findProjectByPath(projects, ["Root A"])?.id).toBe("root_a");
  });

  it("resolves a multi-level path", () => {
    const projects = fixtureTree();

    expect(findProjectByPath(projects, ["Root A", "Child A1", "Grandchild A1a"])?.id).toBe(
      "grandchild_a1a",
    );
  });

  it("matches case-insensitively at every level", () => {
    const projects = fixtureTree();

    expect(findProjectByPath(projects, ["root a", "CHILD A1"])?.id).toBe("child_a1");
  });

  it("returns undefined when an intermediate segment has no match", () => {
    const projects = fixtureTree();

    expect(findProjectByPath(projects, ["Root A", "Bogus", "Grandchild A1a"])).toBeUndefined();
  });

  it("returns undefined when the final segment has no match", () => {
    const projects = fixtureTree();

    expect(findProjectByPath(projects, ["Root A", "Child A1", "Bogus"])).toBeUndefined();
  });

  it("returns undefined for an empty segments array", () => {
    const projects = fixtureTree();

    expect(findProjectByPath(projects, [])).toBeUndefined();
  });

  it("disambiguates same-named projects under different parents", () => {
    const base = (id: string, name: string, parentId: string | undefined, order: number): Project => ({
      id,
      name,
      color: "#111111",
      parent_id: parentId,
      order,
      created: "2026-06-11T10:00:00+00:00",
      board: { statuses: [] },
      defaults: { tags: [] },
    });
    const projects = [
      base("work", "Work", undefined, 1),
      base("personal", "Personal", undefined, 2),
      base("homework_work", "Homework", "work", 1),
      base("homework_personal", "Homework", "personal", 1),
    ];

    expect(findProjectByPath(projects, ["Work", "Homework"])?.id).toBe("homework_work");
    expect(findProjectByPath(projects, ["Personal", "Homework"])?.id).toBe("homework_personal");
  });
});
