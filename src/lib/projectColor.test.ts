import { describe, expect, test } from "vitest";
import { resolveBarLightness, resolveCardLightness, resolveInkMode, resolveProjectColor } from "./projectColor";
import { DEFAULT_PROJECT_COLOR, type Project, type ProjectBoard } from "./types";

const project = (name: string, color: string, board: Partial<ProjectBoard> = {}): Project => ({
  id: name.toLowerCase(),
  name,
  color,
  order: 0,
  created: "2026-06-18T00:00:00Z",
  board: { statuses: [], default_status: undefined, ...board },
  defaults: { tags: [] },
});

describe("resolveProjectColor", () => {
  test("returns the matching project's color", () => {
    const projects = [project("Work", "#ef4444"), project("Home", "#22c55e")];
    expect(resolveProjectColor("Work", projects)).toBe("#ef4444");
  });

  test("returns the default color when projectName is undefined", () => {
    expect(resolveProjectColor(undefined, [project("Work", "#ef4444")])).toBe(DEFAULT_PROJECT_COLOR);
  });

  test("returns the default color when no project matches the name", () => {
    expect(resolveProjectColor("Unknown", [project("Work", "#ef4444")])).toBe(DEFAULT_PROJECT_COLOR);
  });

  test("project name matching is case-sensitive (mirrors stored project name exactly)", () => {
    expect(resolveProjectColor("work", [project("Work", "#ef4444")])).toBe(DEFAULT_PROJECT_COLOR);
  });
});

describe("resolveCardLightness", () => {
  test("returns the matching project's override when set", () => {
    const projects = [project("Work", "#ef4444", { card_lightness: 0.7 })];
    expect(resolveCardLightness("Work", projects, 0.5)).toBe(0.7);
  });

  test("falls back to the global value when the project has no override", () => {
    const projects = [project("Work", "#ef4444")];
    expect(resolveCardLightness("Work", projects, 0.5)).toBe(0.5);
  });

  test("falls back to the global value when projectName is undefined", () => {
    expect(resolveCardLightness(undefined, [project("Work", "#ef4444", { card_lightness: 0.7 })], 0.5)).toBe(
      0.5,
    );
  });

  test("falls back to the global value when no project matches the name", () => {
    expect(resolveCardLightness("Unknown", [project("Work", "#ef4444", { card_lightness: 0.7 })], 0.5)).toBe(
      0.5,
    );
  });

  test("an override of exactly 0 is respected, not treated as unset", () => {
    const projects = [project("Work", "#ef4444", { card_lightness: 0 })];
    expect(resolveCardLightness("Work", projects, 0.5)).toBe(0);
  });
});

describe("resolveBarLightness", () => {
  test("returns the matching project's override when set", () => {
    const projects = [project("Work", "#ef4444", { bar_lightness: 0.1 })];
    expect(resolveBarLightness("Work", projects, 0.38)).toBe(0.1);
  });

  test("falls back to the global value when the project has no override", () => {
    const projects = [project("Work", "#ef4444")];
    expect(resolveBarLightness("Work", projects, 0.38)).toBe(0.38);
  });

  test("resolves card and bar lightness independently for the same project", () => {
    const projects = [project("Work", "#ef4444", { card_lightness: 0.7 })];
    expect(resolveCardLightness("Work", projects, 0.5)).toBe(0.7);
    expect(resolveBarLightness("Work", projects, 0.38)).toBe(0.38);
  });
});

describe("resolveInkMode", () => {
  test("returns the matching project's override when set", () => {
    const projects = [project("Work", "#ef4444", { ink_mode: "white" })];
    expect(resolveInkMode("Work", projects, "auto")).toBe("white");
  });

  test("falls back to the global value when the project has no override", () => {
    const projects = [project("Work", "#ef4444")];
    expect(resolveInkMode("Work", projects, "auto")).toBe("auto");
  });

  test("falls back to the global value when projectName is undefined", () => {
    expect(resolveInkMode(undefined, [project("Work", "#ef4444", { ink_mode: "black" })], "auto")).toBe("auto");
  });

  test("falls back to the global value when no project matches the name", () => {
    expect(resolveInkMode("Unknown", [project("Work", "#ef4444", { ink_mode: "black" })], "auto")).toBe("auto");
  });

  test("a project override of 'auto' is respected even when the global default is forced", () => {
    const projects = [project("Work", "#ef4444", { ink_mode: "auto" })];
    expect(resolveInkMode("Work", projects, "white")).toBe("auto");
  });
});
