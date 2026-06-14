import { describe, expect, test } from "vitest";
import {
  defaultStatusId,
  FALLBACK_STATUS_COLOR,
  sortedStatuses,
  statusColor,
  statusLabel,
} from "./statuses.svelte";
import type { StatusDefinition } from "./types";

const STATUSES: StatusDefinition[] = [
  { id: "do", label: "Do", order: 2, color: "oklch(52% 0.16 235)" },
  { id: "backlog", label: "Backlog", order: 1, color: "oklch(55% 0.01 270)" },
  { id: "done", label: "Done", order: 3, color: "oklch(58% 0.14 155)" },
];

describe("sortedStatuses", () => {
  test("sorts statuses by order ascending", () => {
    const sorted = sortedStatuses(STATUSES);

    expect(sorted.map((status) => status.id)).toEqual(["backlog", "do", "done"]);
  });

  test("does not mutate the input array", () => {
    const original = [...STATUSES];

    sortedStatuses(STATUSES);

    expect(STATUSES).toEqual(original);
  });

  test("returns an empty array for an empty input", () => {
    expect(sortedStatuses([])).toEqual([]);
  });
});

describe("statusLabel", () => {
  test("returns the label for a known status id", () => {
    expect(statusLabel(STATUSES, "backlog")).toBe("Backlog");
  });

  test("falls back to the id itself for an unknown status id", () => {
    expect(statusLabel(STATUSES, "on-hold")).toBe("on-hold");
  });
});

describe("statusColor", () => {
  test("returns the color for a known status id", () => {
    expect(statusColor(STATUSES, "backlog")).toBe("oklch(55% 0.01 270)");
  });

  test("falls back to FALLBACK_STATUS_COLOR for an unknown status id", () => {
    expect(statusColor(STATUSES, "on-hold")).toBe(FALLBACK_STATUS_COLOR);
  });
});

describe("defaultStatusId", () => {
  test("returns the configured default when it names a defined status", () => {
    expect(defaultStatusId(STATUSES, "done")).toBe("done");
  });

  test("falls back to the lowest-order status when the default is undefined", () => {
    expect(defaultStatusId(STATUSES, undefined)).toBe("backlog");
  });

  test("falls back to the lowest-order status when the default names an unknown id", () => {
    expect(defaultStatusId(STATUSES, "on-hold")).toBe("backlog");
  });

  test("falls back to backlog when no statuses are defined", () => {
    expect(defaultStatusId([], "on-hold")).toBe("backlog");
  });
});
