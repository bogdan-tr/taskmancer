import { describe, expect, test } from "vitest";
import type { ParsedTaskInput } from "./naturalLanguage";
import {
  effectiveDefaultCode,
  effectiveDefaultStatus,
  effectiveDefaultTags,
  mergeTags,
  resolveTaskPreview,
} from "./taskPreview";
import type { PriorityLevel, StatusDefinition, TaskDefaults } from "./types";

const PRIORITIES: PriorityLevel[] = [
  { id: "high", label: "High", color: "#bc267f", rank: 1 },
  { id: "medium", label: "Medium", color: "#aa6a00", rank: 2 },
  { id: "low", label: "Low", color: "#0e9254", rank: 3 },
];

const STATUSES: StatusDefinition[] = [
  { id: "backlog", label: "Backlog", order: 1, color: "#6f7178" },
  { id: "do", label: "Do", order: 2, color: "#0073b6" },
  { id: "done", label: "Done", order: 3, color: "#0e9254" },
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

describe("effectiveDefaultStatus", () => {
  test("uses the project board default when it names a defined status", () => {
    expect(effectiveDefaultStatus(STATUSES, "done", "do")).toBe("done");
  });

  test("falls back to the global default when the board default is invalid", () => {
    expect(effectiveDefaultStatus(STATUSES, "nonexistent", "do")).toBe("do");
  });

  test("falls back to the global default when the board default is undefined", () => {
    expect(effectiveDefaultStatus(STATUSES, undefined, "do")).toBe("do");
  });

  test("falls back to the lowest-order status when neither default is valid", () => {
    expect(effectiveDefaultStatus(STATUSES, undefined, undefined)).toBe("backlog");
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

const NOW = new Date(2026, 5, 14); // 2026-06-14

describe("resolveTaskPreview", () => {
  test("uses the default project, configured default priority/status, and global defaults when nothing is overridden", () => {
    const preview = resolveTaskPreview({
      parsed: parsed(),
      defaultProjectName: "General",
      globalDefaults: { tags: ["chore"], priority: "low", status: "do", due: "next_day", scheduled: "today" },
      priorities: PRIORITIES,
      statuses: STATUSES,
      now: NOW,
    });

    expect(preview).toEqual({
      project: "General",
      priorityId: "low",
      statusId: "do",
      tags: ["chore"],
      due: "2026-06-15",
      scheduled: "Today",
    });
  });

  test("falls back to the lowest-rank priority/order status and leaves due/scheduled unset when no defaults are configured", () => {
    const preview = resolveTaskPreview({
      parsed: parsed(),
      defaultProjectName: "General",
      globalDefaults: EMPTY_DEFAULTS,
      priorities: PRIORITIES,
      statuses: STATUSES,
    });

    expect(preview).toEqual({
      project: "General",
      priorityId: "high",
      statusId: "backlog",
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
      statuses: STATUSES,
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
      statuses: STATUSES,
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
      statuses: STATUSES,
    });

    expect(preview.project).toBe("Errands");
  });

  test("a !priority quick-add token overrides the resolved default priority", () => {
    const preview = resolveTaskPreview({
      parsed: parsed({ priority: "low" }),
      defaultProjectName: "General",
      globalDefaults: { tags: [], priority: "high" },
      priorities: PRIORITIES,
      statuses: STATUSES,
    });

    expect(preview.priorityId).toBe("low");
  });

  test("an @status quick-add token overrides the resolved default status", () => {
    const preview = resolveTaskPreview({
      parsed: parsed({ status: "done" }),
      defaultProjectName: "General",
      globalDefaults: { tags: [], status: "do" },
      priorities: PRIORITIES,
      statuses: STATUSES,
    });

    expect(preview.statusId).toBe("done");
  });

  test("the matched project's board default status is used when no @status token is given", () => {
    const preview = resolveTaskPreview({
      parsed: parsed(),
      defaultProjectName: "General",
      globalDefaults: { tags: [], status: "do" },
      priorities: PRIORITIES,
      statuses: STATUSES,
      projectBoardDefaultStatus: "done",
    });

    expect(preview.statusId).toBe("done");
  });

  test("an @status quick-add token overrides the project board default status", () => {
    const preview = resolveTaskPreview({
      parsed: parsed({ status: "backlog" }),
      defaultProjectName: "General",
      globalDefaults: { tags: [], status: "do" },
      priorities: PRIORITIES,
      statuses: STATUSES,
      projectBoardDefaultStatus: "done",
    });

    expect(preview.statusId).toBe("backlog");
  });

  test("merges quick-add tags with the effective default tags", () => {
    const preview = resolveTaskPreview({
      parsed: parsed({ tags: ["urgent"] }),
      defaultProjectName: "General",
      globalDefaults: { tags: ["chore"] },
      priorities: PRIORITIES,
      statuses: STATUSES,
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
      statuses: STATUSES,
    });

    expect(preview.tags).toEqual(["school"]);
  });

  test("project default due/scheduled codes override global codes", () => {
    const preview = resolveTaskPreview({
      parsed: parsed(),
      defaultProjectName: "General",
      globalDefaults: { tags: [], due: "same_day", scheduled: "today" },
      projectDefaults: { tags: [], due: "in_1_week", scheduled: "in_1_month" },
      priorities: PRIORITIES,
      statuses: STATUSES,
      now: NOW,
    });

    // scheduled = in_1_month relative to now (2026-06-14) = 2026-07-14;
    // due = in_1_week relative to *that* scheduled date, not to now.
    expect(preview.due).toBe("2026-07-21");
    expect(preview.scheduled).toBe("In 1 month");
  });

  test("a default due code resolves relative to an explicitly-typed scheduled date, not 'today' (regression test: AddTaskModal preview previously showed 'Due today' for any default-resolved due date)", () => {
    const preview = resolveTaskPreview({
      // e.g. typing "sch tomorrow" on 2026-06-14, with no explicit due: token.
      parsed: parsed({ scheduled: "2026-06-15" }),
      defaultProjectName: "General",
      globalDefaults: { tags: [], due: "next_day" },
      priorities: PRIORITIES,
      statuses: STATUSES,
      now: NOW,
    });

    expect(preview.scheduled).toBe("2026-06-15");
    expect(preview.due).toBe("2026-06-16");
  });

  test("an explicit due:/sch: quick-add token overrides the relative-date default", () => {
    const preview = resolveTaskPreview({
      parsed: parsed({ due: "2026-07-01", scheduled: "2026-07-02" }),
      defaultProjectName: "General",
      globalDefaults: { tags: [], due: "same_day", scheduled: "today" },
      priorities: PRIORITIES,
      statuses: STATUSES,
    });

    expect(preview.due).toBe("2026-07-01");
    expect(preview.scheduled).toBe("2026-07-02");
  });

  test("a due:na/due na quick-add token shows 'Never' regardless of the default due code", () => {
    const preview = resolveTaskPreview({
      parsed: parsed({ due: "none" }),
      defaultProjectName: "General",
      globalDefaults: { tags: [], due: "same_day", scheduled: "today" },
      priorities: PRIORITIES,
      statuses: STATUSES,
    });

    expect(preview.due).toBe("Never");
  });

  test("falls back to 'Never' when the effective default due code is the 'none' sentinel", () => {
    const preview = resolveTaskPreview({
      parsed: parsed(),
      defaultProjectName: "General",
      globalDefaults: { tags: [], due: "none", scheduled: "today" },
      priorities: PRIORITIES,
      statuses: STATUSES,
    });

    expect(preview.due).toBe("Never");
  });
});
