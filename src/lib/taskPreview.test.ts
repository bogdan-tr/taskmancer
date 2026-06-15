import { describe, expect, test } from "vitest";
import type { ParsedTaskInput } from "./naturalLanguage";
import { effectiveDefaultCode, effectiveDefaultTags, mergeTags, resolveTaskPreview } from "./taskPreview";
import type { PriorityLevel, TaskDefaults } from "./types";

const PRIORITIES: PriorityLevel[] = [
  { id: "high", label: "High", color: "#bc267f", rank: 1 },
  { id: "medium", label: "Medium", color: "#aa6a00", rank: 2 },
  { id: "low", label: "Low", color: "#0e9254", rank: 3 },
];

const EMPTY_DEFAULTS: TaskDefaults = { tags: [] };

function parsed(overrides: Partial<ParsedTaskInput> = {}): ParsedTaskInput {
  return { title: "Buy milk", tags: [], ...overrides };
}

describe("effectiveDefaultTags", () => {
  test("returns the project tags when they're non-empty", () => {
    expect(effectiveDefaultTags(["global"], ["project"])).toEqual(["project"]);
  });

  test("falls back to the global tags when the project tags are empty", () => {
    expect(effectiveDefaultTags(["global"], [])).toEqual(["global"]);
  });

  test("falls back to the global tags when the project tags are undefined", () => {
    expect(effectiveDefaultTags(["global"], undefined)).toEqual(["global"]);
  });
});

describe("effectiveDefaultCode", () => {
  test("returns the project code when it's set", () => {
    expect(effectiveDefaultCode("today", "tomorrow")).toBe("tomorrow");
  });

  test("falls back to the global code when the project code is unset", () => {
    expect(effectiveDefaultCode("today", undefined)).toBe("today");
  });

  test("returns undefined when neither code is set", () => {
    expect(effectiveDefaultCode(undefined, undefined)).toBeUndefined();
  });
});

describe("mergeTags", () => {
  test("appends default tags after explicit tags", () => {
    expect(mergeTags(["urgent"], ["work"])).toEqual(["urgent", "work"]);
  });

  test("returns the explicit tags unchanged when there are no defaults", () => {
    expect(mergeTags(["urgent"], [])).toEqual(["urgent"]);
  });

  test("returns the default tags when there are no explicit tags", () => {
    expect(mergeTags([], ["work"])).toEqual(["work"]);
  });

  test("does not duplicate a tag present in both lists", () => {
    expect(mergeTags(["urgent", "work"], ["work", "home"])).toEqual(["urgent", "work", "home"]);
  });
});

describe("resolveTaskPreview", () => {
  test("uses the default project, configured default priority, and global defaults when nothing is overridden", () => {
    const preview = resolveTaskPreview({
      parsed: parsed(),
      defaultProjectName: "General",
      globalDefaults: { tags: ["chore"], priority: "low", due: "tomorrow", scheduled: "today" },
      priorities: PRIORITIES,
    });

    expect(preview).toEqual({
      project: "General",
      priorityId: "low",
      tags: ["chore"],
      due: "Tomorrow",
      scheduled: "Today",
    });
  });

  test("falls back to the lowest-rank priority and leaves due/scheduled unset when no defaults are configured", () => {
    const preview = resolveTaskPreview({
      parsed: parsed(),
      defaultProjectName: "General",
      globalDefaults: EMPTY_DEFAULTS,
      priorities: PRIORITIES,
    });

    expect(preview).toEqual({
      project: "General",
      priorityId: "high",
      tags: [],
      due: undefined,
      scheduled: undefined,
    });
  });

  test("uses the project filter as the project when no +Project token is given", () => {
    const preview = resolveTaskPreview({
      parsed: parsed(),
      projectFilter: "Homework",
      defaultProjectName: "General",
      globalDefaults: EMPTY_DEFAULTS,
      priorities: PRIORITIES,
    });

    expect(preview.project).toBe("Homework");
  });

  test("a +Project quick-add token overrides the project filter", () => {
    const preview = resolveTaskPreview({
      parsed: parsed({ project: "Errands" }),
      projectFilter: "Homework",
      defaultProjectName: "General",
      globalDefaults: EMPTY_DEFAULTS,
      priorities: PRIORITIES,
    });

    expect(preview.project).toBe("Errands");
  });

  test("uses the matched project's canonical casing over a differently-cased +Project token", () => {
    const preview = resolveTaskPreview({
      parsed: parsed({ project: "errands" }),
      defaultProjectName: "General",
      globalDefaults: EMPTY_DEFAULTS,
      matchedProjectName: "Errands",
      priorities: PRIORITIES,
    });

    expect(preview.project).toBe("Errands");
  });

  test("a !priority quick-add token overrides the resolved default priority", () => {
    const preview = resolveTaskPreview({
      parsed: parsed({ priority: "low" }),
      defaultProjectName: "General",
      globalDefaults: { tags: [], priority: "high" },
      priorities: PRIORITIES,
    });

    expect(preview.priorityId).toBe("low");
  });

  test("merges quick-add tags with the effective default tags", () => {
    const preview = resolveTaskPreview({
      parsed: parsed({ tags: ["urgent"] }),
      defaultProjectName: "General",
      globalDefaults: { tags: ["chore"] },
      priorities: PRIORITIES,
    });

    expect(preview.tags).toEqual(["urgent", "chore"]);
  });

  test("non-empty project default tags override global default tags", () => {
    const preview = resolveTaskPreview({
      parsed: parsed(),
      defaultProjectName: "General",
      globalDefaults: { tags: ["chore"] },
      projectDefaults: { tags: ["school"] },
      priorities: PRIORITIES,
    });

    expect(preview.tags).toEqual(["school"]);
  });

  test("project default due/scheduled codes override global codes", () => {
    const preview = resolveTaskPreview({
      parsed: parsed(),
      defaultProjectName: "General",
      globalDefaults: { tags: [], due: "today", scheduled: "today" },
      projectDefaults: { tags: [], due: "in_1_week", scheduled: "in_1_month" },
      priorities: PRIORITIES,
    });

    expect(preview.due).toBe("In 1 week");
    expect(preview.scheduled).toBe("In 1 month");
  });

  test("an explicit due:/sch: quick-add token overrides the relative-date default", () => {
    const preview = resolveTaskPreview({
      parsed: parsed({ due: "2026-07-01", scheduled: "2026-07-02" }),
      defaultProjectName: "General",
      globalDefaults: { tags: [], due: "today", scheduled: "today" },
      priorities: PRIORITIES,
    });

    expect(preview.due).toBe("2026-07-01");
    expect(preview.scheduled).toBe("2026-07-02");
  });
});
