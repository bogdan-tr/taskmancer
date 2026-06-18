import { describe, expect, test } from "vitest";
import { resolveProjectColor } from "./projectColor";
import { DEFAULT_PROJECT_COLOR, type Project } from "./types";

const project = (name: string, color: string): Project => ({
  id: name.toLowerCase(),
  name,
  color,
  order: 0,
  created: "2026-06-18T00:00:00Z",
  board: { statuses: [], default_status: undefined },
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
