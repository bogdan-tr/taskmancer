import { describe, expect, test } from "vitest";
import { buildStatusTimeline, formatStatusDuration } from "./statusTimeline";
import type { StatusHistoryEntry } from "./types";

function entry(overrides: Partial<StatusHistoryEntry>): StatusHistoryEntry {
  return {
    id: overrides.id ?? crypto.randomUUID(),
    task_id: overrides.task_id ?? "t1",
    from_status: overrides.from_status ?? null,
    to_status: overrides.to_status ?? "backlog",
    changed_at: overrides.changed_at ?? "2026-01-01T00:00:00Z",
    source: overrides.source ?? "user",
  };
}

const NOW = new Date("2026-01-10T00:00:00Z");

describe("buildStatusTimeline", () => {
  test("returns an empty array for no entries", () => {
    expect(buildStatusTimeline([], NOW)).toEqual([]);
  });

  test("a single entry spans from its timestamp until now and is current", () => {
    const e = entry({ to_status: "backlog", changed_at: "2026-01-09T00:00:00Z", source: "seed" });
    const rows = buildStatusTimeline([e], NOW);

    expect(rows).toHaveLength(1);
    expect(rows[0].status).toBe("backlog");
    expect(rows[0].changedAt).toBe("2026-01-09T00:00:00Z");
    expect(rows[0].durationMs).toBe(24 * 60 * 60 * 1000); // exactly one day
    expect(rows[0].isCurrent).toBe(true);
    expect(rows[0].isSeed).toBe(true);
  });

  test("each non-final row's duration runs until the next transition", () => {
    const entries = [
      entry({ to_status: "backlog", changed_at: "2026-01-01T00:00:00Z" }),
      entry({ to_status: "in-progress", changed_at: "2026-01-03T00:00:00Z" }),
    ];
    const rows = buildStatusTimeline(entries, NOW);

    expect(rows[0].durationMs).toBe(2 * 24 * 60 * 60 * 1000); // 2 days backlog
    expect(rows[0].isCurrent).toBe(false);
    expect(rows[1].durationMs).toBe(7 * 24 * 60 * 60 * 1000); // until NOW
    expect(rows[1].isCurrent).toBe(true);
  });

  test("marks only seed-sourced rows as seed", () => {
    const entries = [
      entry({ to_status: "backlog", changed_at: "2026-01-01T00:00:00Z", source: "seed" }),
      entry({ to_status: "done", changed_at: "2026-01-02T00:00:00Z", source: "user" }),
    ];
    const rows = buildStatusTimeline(entries, NOW);
    expect(rows[0].isSeed).toBe(true);
    expect(rows[1].isSeed).toBe(false);
  });

  test("sorts unsorted input ascending by changed_at", () => {
    const entries = [
      entry({ to_status: "in-progress", changed_at: "2026-01-05T00:00:00Z" }),
      entry({ to_status: "backlog", changed_at: "2026-01-01T00:00:00Z" }),
    ];
    const rows = buildStatusTimeline(entries, NOW);
    expect(rows.map((r) => r.status)).toEqual(["backlog", "in-progress"]);
  });

  test("clamps a negative duration (now earlier than last change) to zero", () => {
    const past = new Date("2026-01-01T00:00:00Z");
    const entries = [entry({ to_status: "backlog", changed_at: "2026-01-05T00:00:00Z" })];
    const rows = buildStatusTimeline(entries, past);
    expect(rows[0].durationMs).toBe(0);
  });

  test("chains durations across three transitions", () => {
    const entries = [
      entry({ to_status: "backlog", changed_at: "2026-01-01T00:00:00Z" }),
      entry({ to_status: "in-progress", changed_at: "2026-01-02T00:00:00Z" }),
      entry({ to_status: "done", changed_at: "2026-01-04T00:00:00Z" }),
    ];
    const rows = buildStatusTimeline(entries, NOW);
    expect(rows[0].durationMs).toBe(1 * 24 * 60 * 60 * 1000);
    expect(rows[1].durationMs).toBe(2 * 24 * 60 * 60 * 1000);
    expect(rows[2].durationMs).toBe(6 * 24 * 60 * 60 * 1000);
    expect(rows[2].isCurrent).toBe(true);
  });
});

describe("formatStatusDuration", () => {
  test.each([
    [0, "< 1m"],
    [30_000, "< 1m"],
    [5 * 60_000, "5m"],
    [(2 * 3600 + 15 * 60) * 1000, "2h 15m"],
    [2 * 3600_000, "2h"],
    [(3 * 86400 + 4 * 3600) * 1000, "3d 4h"],
    [3 * 86400_000, "3d"],
    [(1 * 86400 + 30 * 60) * 1000, "1d"], // minutes dropped once days are shown
  ])("formats %ims as %s", (ms, expected) => {
    expect(formatStatusDuration(ms)).toBe(expected);
  });
});
