import { describe, expect, test } from "vitest";
import { resolveBarLightness, resolveCardLightness, resolveInkMode, resolveProjectColor } from "./projectColor";
import { DEFAULT_PROJECT_COLOR, type Project, type ProjectBoard } from "./types";

const project = (
  name: string,
  color: string,
  board: Partial<ProjectBoard> = {},
  parentId?: string,
): Project => ({
  id: name.toLowerCase(),
  name,
  color,
  parent_id: parentId,
  order: 0,
  created: "2026-06-18T00:00:00Z",
  board: { statuses: [], default_status: undefined, ...board },
  defaults: { tags: [] },
});

describe("resolveProjectColor", () => {
  test("returns the matching project's color", () => {
    const work = project("Work", "#ef4444");
    const projects = [work, project("Home", "#22c55e")];
    expect(resolveProjectColor(work.id, projects)).toBe("#ef4444");
  });

  test("returns the default color when projectId is undefined", () => {
    expect(resolveProjectColor(undefined, [project("Work", "#ef4444")])).toBe(DEFAULT_PROJECT_COLOR);
  });

  test("returns the default color when no project matches the id", () => {
    expect(resolveProjectColor("unknown-id", [project("Work", "#ef4444")])).toBe(DEFAULT_PROJECT_COLOR);
  });
});

describe("resolveCardLightness", () => {
  test("returns the matching project's override when set", () => {
    const work = project("Work", "#ef4444", { card_lightness: 0.7 });
    expect(resolveCardLightness(work.id, [work], 0.5)).toBe(0.7);
  });

  test("falls back to the global value when the project has no override", () => {
    const work = project("Work", "#ef4444");
    expect(resolveCardLightness(work.id, [work], 0.5)).toBe(0.5);
  });

  test("falls back to the global value when projectId is undefined", () => {
    expect(resolveCardLightness(undefined, [project("Work", "#ef4444", { card_lightness: 0.7 })], 0.5)).toBe(
      0.5,
    );
  });

  test("falls back to the global value when no project matches the id", () => {
    expect(
      resolveCardLightness("unknown-id", [project("Work", "#ef4444", { card_lightness: 0.7 })], 0.5),
    ).toBe(0.5);
  });

  test("an override of exactly 0 is respected, not treated as unset", () => {
    const work = project("Work", "#ef4444", { card_lightness: 0 });
    expect(resolveCardLightness(work.id, [work], 0.5)).toBe(0);
  });
});

describe("resolveBarLightness", () => {
  test("returns the matching project's override when set", () => {
    const work = project("Work", "#ef4444", { bar_lightness: 0.1 });
    expect(resolveBarLightness(work.id, [work], 0.38)).toBe(0.1);
  });

  test("falls back to the global value when the project has no override", () => {
    const work = project("Work", "#ef4444");
    expect(resolveBarLightness(work.id, [work], 0.38)).toBe(0.38);
  });

  test("resolves card and bar lightness independently for the same project", () => {
    const work = project("Work", "#ef4444", { card_lightness: 0.7 });
    expect(resolveCardLightness(work.id, [work], 0.5)).toBe(0.7);
    expect(resolveBarLightness(work.id, [work], 0.38)).toBe(0.38);
  });
});

describe("resolveInkMode", () => {
  test("returns the matching project's override when set", () => {
    const work = project("Work", "#ef4444", { ink_mode: "white" });
    expect(resolveInkMode(work.id, [work], "auto")).toBe("white");
  });

  test("falls back to the global value when the project has no override", () => {
    const work = project("Work", "#ef4444");
    expect(resolveInkMode(work.id, [work], "auto")).toBe("auto");
  });

  test("falls back to the global value when projectId is undefined", () => {
    expect(resolveInkMode(undefined, [project("Work", "#ef4444", { ink_mode: "black" })], "auto")).toBe("auto");
  });

  test("falls back to the global value when no project matches the id", () => {
    expect(resolveInkMode("unknown-id", [project("Work", "#ef4444", { ink_mode: "black" })], "auto")).toBe(
      "auto",
    );
  });

  test("a project override of 'auto' is respected even when the global default is forced", () => {
    const work = project("Work", "#ef4444", { ink_mode: "auto" });
    expect(resolveInkMode(work.id, [work], "white")).toBe("auto");
  });
});

describe("ancestor-chain resolution", () => {
  test("resolveCardLightness falls through to a grandparent's override", () => {
    const grandparent = project("Grandparent", "#111111", { card_lightness: 0.7 });
    const parent = project("Parent", "#222222", {}, grandparent.id);
    const child = project("Child", "#333333", {}, parent.id);
    const projects = [grandparent, parent, child];

    expect(resolveCardLightness(child.id, projects, 0.5)).toBe(0.7);
  });

  test("resolveCardLightness prefers the nearest override over a further one", () => {
    const grandparent = project("Grandparent", "#111111", { card_lightness: 0.7 });
    const parent = project("Parent", "#222222", { card_lightness: 0.9 }, grandparent.id);
    const child = project("Child", "#333333", {}, parent.id);
    const projects = [grandparent, parent, child];

    expect(resolveCardLightness(child.id, projects, 0.5)).toBe(0.9);
  });

  test("resolveBarLightness falls through to a grandparent's override", () => {
    const grandparent = project("Grandparent", "#111111", { bar_lightness: 0.6 });
    const parent = project("Parent", "#222222", {}, grandparent.id);
    const child = project("Child", "#333333", {}, parent.id);
    const projects = [grandparent, parent, child];

    expect(resolveBarLightness(child.id, projects, 0.38)).toBe(0.6);
  });

  test("resolveInkMode falls through to a grandparent's override", () => {
    const grandparent = project("Grandparent", "#111111", { ink_mode: "white" });
    const parent = project("Parent", "#222222", {}, grandparent.id);
    const child = project("Child", "#333333", {}, parent.id);
    const projects = [grandparent, parent, child];

    expect(resolveInkMode(child.id, projects, "auto")).toBe("white");
  });

  test("falls back to global when no project in the chain has an override", () => {
    const grandparent = project("Grandparent", "#111111");
    const parent = project("Parent", "#222222", {}, grandparent.id);
    const child = project("Child", "#333333", {}, parent.id);
    const projects = [grandparent, parent, child];

    expect(resolveCardLightness(child.id, projects, 0.5)).toBe(0.5);
  });
});
