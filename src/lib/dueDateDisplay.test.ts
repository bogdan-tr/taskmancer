import { describe, expect, test } from "vitest";
import { formatDueDateDisplay } from "./dueDateDisplay";

/** Wednesday 2026-06-18 */
const TODAY = new Date(2026, 5, 18);

describe("formatDueDateDisplay", () => {
  test("returns null when due is undefined", () => {
    expect(formatDueDateDisplay(undefined, TODAY, false)).toBeNull();
  });

  test("past due: variant=overdue, label says 'Overdue · Nd'", () => {
    const result = formatDueDateDisplay("2026-06-16", TODAY, false);
    expect(result).toEqual({ label: "Overdue · 2d", variant: "overdue" });
  });

  test("past due by 1 day: label says 'Overdue · 1d'", () => {
    const result = formatDueDateDisplay("2026-06-17", TODAY, false);
    expect(result).toEqual({ label: "Overdue · 1d", variant: "overdue" });
  });

  test("today: variant=today", () => {
    const result = formatDueDateDisplay("2026-06-18", TODAY, false);
    expect(result).toEqual({ label: "Due today", variant: "today" });
  });

  test("tomorrow: variant=tomorrow", () => {
    const result = formatDueDateDisplay("2026-06-19", TODAY, false);
    expect(result).toEqual({ label: "Due tomorrow", variant: "tomorrow" });
  });

  test("2 days out with nlEnabled=false: falls back to YYYY-MM-DD", () => {
    const result = formatDueDateDisplay("2026-06-20", TODAY, false);
    expect(result).toEqual({ label: "Due 2026-06-20", variant: "normal" });
  });

  test("2 days out with nlEnabled=true: 'Due this {Weekday}'", () => {
    // 2026-06-20 is Saturday
    const result = formatDueDateDisplay("2026-06-20", TODAY, true);
    expect(result).toEqual({ label: "Due this Saturday", variant: "normal" });
  });

  test("6 days out with nlEnabled=true: still 'Due this {Weekday}'", () => {
    // 2026-06-24 is Wednesday
    const result = formatDueDateDisplay("2026-06-24", TODAY, true);
    expect(result).toEqual({ label: "Due this Wednesday", variant: "normal" });
  });

  test("7 days out with nlEnabled=true: 'Due next {Weekday}'", () => {
    // 2026-06-25 is Thursday
    const result = formatDueDateDisplay("2026-06-25", TODAY, true);
    expect(result).toEqual({ label: "Due next Thursday", variant: "normal" });
  });

  test("13 days out with nlEnabled=true: still 'Due next {Weekday}'", () => {
    // 2026-07-01 is Wednesday
    const result = formatDueDateDisplay("2026-07-01", TODAY, true);
    expect(result).toEqual({ label: "Due next Wednesday", variant: "normal" });
  });

  test("14 days out with nlEnabled=true: falls back to YYYY-MM-DD", () => {
    const result = formatDueDateDisplay("2026-07-02", TODAY, true);
    expect(result).toEqual({ label: "Due 2026-07-02", variant: "normal" });
  });

  test("today and tomorrow are not affected by nlEnabled flag", () => {
    expect(formatDueDateDisplay("2026-06-18", TODAY, true)).toEqual({ label: "Due today", variant: "today" });
    expect(formatDueDateDisplay("2026-06-19", TODAY, true)).toEqual({ label: "Due tomorrow", variant: "tomorrow" });
  });

  test("overdue is not affected by nlEnabled flag", () => {
    expect(formatDueDateDisplay("2026-06-15", TODAY, true)).toEqual({ label: "Overdue · 3d", variant: "overdue" });
  });
});
