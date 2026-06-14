import { describe, expect, test } from "vitest";
import {
  defaultPriorityId,
  FALLBACK_PRIORITY_COLOR,
  priorityColor,
  priorityLabel,
  sortedPriorities,
} from "./priorities.svelte";
import type { PriorityLevel } from "./types";

const PRIORITIES: PriorityLevel[] = [
  { id: "medium", label: "Medium", color: "oklch(58% 0.13 70)", rank: 2 },
  { id: "high", label: "High", color: "oklch(54% 0.2 350)", rank: 1 },
  { id: "low", label: "Low", color: "oklch(58% 0.14 155)", rank: 3 },
];

describe("sortedPriorities", () => {
  test("sorts levels by rank ascending", () => {
    const sorted = sortedPriorities(PRIORITIES);

    expect(sorted.map((level) => level.id)).toEqual(["high", "medium", "low"]);
  });

  test("does not mutate the input array", () => {
    const original = [...PRIORITIES];

    sortedPriorities(PRIORITIES);

    expect(PRIORITIES).toEqual(original);
  });

  test("returns an empty array for an empty input", () => {
    expect(sortedPriorities([])).toEqual([]);
  });
});

describe("priorityLabel", () => {
  test("returns the label for a known priority id", () => {
    expect(priorityLabel(PRIORITIES, "high")).toBe("High");
  });

  test("falls back to the id itself for an unknown priority id", () => {
    expect(priorityLabel(PRIORITIES, "urgent")).toBe("urgent");
  });
});

describe("priorityColor", () => {
  test("returns the color for a known priority id", () => {
    expect(priorityColor(PRIORITIES, "high")).toBe("oklch(54% 0.2 350)");
  });

  test("falls back to FALLBACK_PRIORITY_COLOR for an unknown priority id", () => {
    expect(priorityColor(PRIORITIES, "urgent")).toBe(FALLBACK_PRIORITY_COLOR);
  });
});

describe("defaultPriorityId", () => {
  test("returns the configured default when it names a defined level", () => {
    expect(defaultPriorityId(PRIORITIES, "low")).toBe("low");
  });

  test("falls back to the lowest-rank level when the default is undefined", () => {
    expect(defaultPriorityId(PRIORITIES, undefined)).toBe("high");
  });

  test("falls back to the lowest-rank level when the default names an unknown id", () => {
    expect(defaultPriorityId(PRIORITIES, "urgent")).toBe("high");
  });

  test("falls back to medium when no priority levels are defined", () => {
    expect(defaultPriorityId([], "urgent")).toBe("medium");
  });
});
