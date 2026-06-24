import { describe, expect, test } from "vitest";
import { emptyToUndefined, formatTags, isValidOptionalDate, parseTags, seriesSharedFieldsChanged } from "./taskFields";
import type { Task } from "./types";

function baseTask(overrides: Partial<Task> = {}): Task {
  return {
    id: "task-1",
    title: "Water the plants",
    status: "backlog",
    project_id: "home-id",
    tags: ["chore"],
    priority: "medium",
    due: "2026-06-21",
    scheduled: "2026-06-20",
    order: 1,
    created: "2026-06-15T10:00:00+00:00",
    depends_on: [],
    estimated_minutes: 15,
    tracked_minutes: 0,
    hidden: false,
    notes: "Use the green watering can",
    ...overrides,
  };
}

describe("parseTags", () => {
  test("splits comma-separated tags and trims whitespace", () => {
    expect(parseTags(" urgent, reading ,  home")).toEqual(["urgent", "reading", "home"]);
  });

  test("drops empty entries from repeated or trailing commas", () => {
    expect(parseTags("urgent,, reading, ")).toEqual(["urgent", "reading"]);
  });

  test("returns an empty array for an empty string", () => {
    expect(parseTags("")).toEqual([]);
  });

  test("returns an empty array for whitespace-only input", () => {
    expect(parseTags("   ")).toEqual([]);
  });
});

describe("formatTags", () => {
  test("joins tags with a comma and space", () => {
    expect(formatTags(["urgent", "reading"])).toBe("urgent, reading");
  });

  test("returns an empty string for an empty list", () => {
    expect(formatTags([])).toBe("");
  });
});

describe("emptyToUndefined", () => {
  test("returns undefined for an empty string", () => {
    expect(emptyToUndefined("")).toBeUndefined();
  });

  test("returns undefined for whitespace-only input", () => {
    expect(emptyToUndefined("   ")).toBeUndefined();
  });

  test("returns the trimmed string when non-empty", () => {
    expect(emptyToUndefined("  2026-07-01 ")).toBe("2026-07-01");
  });
});

describe("isValidOptionalDate", () => {
  test("accepts an empty string", () => {
    expect(isValidOptionalDate("")).toBe(true);
  });

  test("accepts a whitespace-only string", () => {
    expect(isValidOptionalDate("   ")).toBe(true);
  });

  test("accepts a YYYY-MM-DD date", () => {
    expect(isValidOptionalDate("2026-07-01")).toBe(true);
  });

  test("rejects a non-ISO date format", () => {
    expect(isValidOptionalDate("07/01/2026")).toBe(false);
  });

  test("rejects a string that isn't a date at all", () => {
    expect(isValidOptionalDate("tomorrow")).toBe(false);
  });
});

describe("seriesSharedFieldsChanged", () => {
  test("returns false when nothing shared changed", () => {
    const original = baseTask();
    const edited = baseTask();

    expect(seriesSharedFieldsChanged(original, edited)).toBe(false);
  });

  test("returns false when only per-occurrence fields changed (status, due, scheduled)", () => {
    const original = baseTask();
    const edited = baseTask({ status: "done", due: "2026-07-01", scheduled: "2026-06-25" });

    expect(seriesSharedFieldsChanged(original, edited)).toBe(false);
  });

  test("returns true when the title changed", () => {
    const original = baseTask();
    const edited = baseTask({ title: "Water the ferns" });

    expect(seriesSharedFieldsChanged(original, edited)).toBe(true);
  });

  test("returns true when the project changed", () => {
    const original = baseTask();
    const edited = baseTask({ project_id: "garden-id" });

    expect(seriesSharedFieldsChanged(original, edited)).toBe(true);
  });

  test("returns true when the priority changed", () => {
    const original = baseTask();
    const edited = baseTask({ priority: "high" });

    expect(seriesSharedFieldsChanged(original, edited)).toBe(true);
  });

  test("returns true when estimated_minutes changed", () => {
    const original = baseTask();
    const edited = baseTask({ estimated_minutes: 30 });

    expect(seriesSharedFieldsChanged(original, edited)).toBe(true);
  });

  test("returns true when notes changed", () => {
    const original = baseTask();
    const edited = baseTask({ notes: "Use the blue watering can" });

    expect(seriesSharedFieldsChanged(original, edited)).toBe(true);
  });

  test("returns true when a tag was added", () => {
    const original = baseTask();
    const edited = baseTask({ tags: ["chore", "urgent"] });

    expect(seriesSharedFieldsChanged(original, edited)).toBe(true);
  });

  test("returns false when tags are the same but in a different order", () => {
    const original = baseTask({ tags: ["chore", "urgent"] });
    const edited = baseTask({ tags: ["urgent", "chore"] });

    expect(seriesSharedFieldsChanged(original, edited)).toBe(false);
  });
});
