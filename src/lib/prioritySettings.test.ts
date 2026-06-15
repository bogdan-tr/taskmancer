import { describe, expect, test } from "vitest";
import { deleteBlockReason, levelsEqual, renumber, toggleDefault, uniqueId } from "./prioritySettings";
import type { PriorityLevel } from "./types";

function makeLevel(overrides: Partial<PriorityLevel> = {}): PriorityLevel {
  return { id: "medium", label: "Medium", color: "oklch(58% 0.13 70)", rank: 2, ...overrides };
}

describe("levelsEqual", () => {
  test("returns true for two lists with identical levels in the same order", () => {
    const a = [makeLevel({ id: "high", rank: 1 }), makeLevel({ id: "low", rank: 2 })];
    const b = [makeLevel({ id: "high", rank: 1 }), makeLevel({ id: "low", rank: 2 })];

    expect(levelsEqual(a, b)).toBe(true);
  });

  test("returns false when lengths differ", () => {
    const a = [makeLevel()];
    const b = [makeLevel(), makeLevel({ id: "low" })];

    expect(levelsEqual(a, b)).toBe(false);
  });

  test("returns false when a label differs", () => {
    const a = [makeLevel({ label: "Medium" })];
    const b = [makeLevel({ label: "Med" })];

    expect(levelsEqual(a, b)).toBe(false);
  });

  test("returns false when a color differs", () => {
    const a = [makeLevel({ color: "oklch(58% 0.13 70)" })];
    const b = [makeLevel({ color: "oklch(50% 0.13 70)" })];

    expect(levelsEqual(a, b)).toBe(false);
  });

  test("returns false when a rank differs", () => {
    const a = [makeLevel({ rank: 1 })];
    const b = [makeLevel({ rank: 2 })];

    expect(levelsEqual(a, b)).toBe(false);
  });

  test("returns false when order differs", () => {
    const a = [makeLevel({ id: "high" }), makeLevel({ id: "low" })];
    const b = [makeLevel({ id: "low" }), makeLevel({ id: "high" })];

    expect(levelsEqual(a, b)).toBe(false);
  });
});

describe("renumber", () => {
  test("assigns rank as each level's 1-based position", () => {
    const levels = [makeLevel({ id: "a", rank: 9 }), makeLevel({ id: "b", rank: 1 })];

    const result = renumber(levels);

    expect(result.map((level) => level.rank)).toEqual([1, 2]);
  });

  test("does not mutate the input levels", () => {
    const levels = [makeLevel({ id: "a", rank: 9 })];
    const before = makeLevel({ id: "a", rank: 9 });

    renumber(levels);

    expect(levels[0]).toEqual(before);
  });

  test("returns an empty array for an empty input", () => {
    expect(renumber([])).toEqual([]);
  });
});

describe("uniqueId", () => {
  test("returns the base id when it isn't already used", () => {
    expect(uniqueId(["high", "low"], "medium")).toBe("medium");
  });

  test("appends -2 when the base id is already used", () => {
    expect(uniqueId(["new-priority"], "new-priority")).toBe("new-priority-2");
  });

  test("skips suffixes until an unused id is found", () => {
    expect(uniqueId(["new-priority", "new-priority-2", "new-priority-3"], "new-priority")).toBe(
      "new-priority-4",
    );
  });
});

describe("deleteBlockReason", () => {
  test("blocks deletion of the last remaining priority level", () => {
    const level = makeLevel({ id: "medium" });

    expect(deleteBlockReason(level, 1, "high", {})).toBe(
      "At least one priority level is required",
    );
  });

  test("blocks deletion of the last remaining level even when it is the default", () => {
    const level = makeLevel({ id: "medium" });

    expect(deleteBlockReason(level, 1, "medium", {})).toBe(
      "At least one priority level is required",
    );
  });

  test("blocks deletion of the default priority", () => {
    const level = makeLevel({ id: "medium" });

    expect(deleteBlockReason(level, 2, "medium", {})).toBe(
      "This is the default priority and can't be deleted",
    );
  });

  test("blocks deletion when tasks use this priority, with singular wording for one task", () => {
    const level = makeLevel({ id: "medium" });

    expect(deleteBlockReason(level, 2, "high", { medium: 1 })).toBe(
      "1 task use this priority — reassign them first",
    );
  });

  test("blocks deletion when tasks use this priority, with plural wording for multiple tasks", () => {
    const level = makeLevel({ id: "medium" });

    expect(deleteBlockReason(level, 2, "high", { medium: 3 })).toBe(
      "3 tasks use this priority — reassign them first",
    );
  });

  test("allows deletion when not the default and no tasks use this priority", () => {
    const level = makeLevel({ id: "medium" });

    expect(deleteBlockReason(level, 2, "high", { medium: 0 })).toBeUndefined();
  });

  test("allows deletion when taskCounts has no entry for this priority", () => {
    const level = makeLevel({ id: "medium" });

    expect(deleteBlockReason(level, 2, "high", {})).toBeUndefined();
  });
});

describe("toggleDefault", () => {
  test("sets the clicked level as the default when none was set", () => {
    expect(toggleDefault(undefined, "medium")).toBe("medium");
  });

  test("sets the clicked level as the default, replacing the previous default", () => {
    expect(toggleDefault("high", "medium")).toBe("medium");
  });

  test("clears the default when the current default is clicked again", () => {
    expect(toggleDefault("medium", "medium")).toBeUndefined();
  });
});
