import { describe, expect, test } from "vitest";
import {
  clampDayOfMonth,
  clampNonNegativeInteger,
  clampPositiveInteger,
  daysBetween,
  dueRuleFromDefaultCode,
  formatDueRule,
  formatRecurrenceFrequency,
  resolveDueRule,
  resolveNonRecurringDue,
  resolveSeriesDueRule,
  type DueRule,
  type RecurrenceFrequency,
} from "./recurrence";

describe("formatRecurrenceFrequency", () => {
  test("formats a daily interval", () => {
    expect(formatRecurrenceFrequency({ kind: "EveryNDays", interval: 1 })).toBe("Daily");
  });

  test("formats an every-other-day interval", () => {
    expect(formatRecurrenceFrequency({ kind: "EveryNDays", interval: 2 })).toBe("Every other day");
  });

  test("formats an arbitrary day interval", () => {
    expect(formatRecurrenceFrequency({ kind: "EveryNDays", interval: 5 })).toBe("Every 5 days");
  });

  test("formats a single weekday", () => {
    const frequency: RecurrenceFrequency = { kind: "Weekly", weekdays: [1], interval_weeks: 1 };
    expect(formatRecurrenceFrequency(frequency)).toBe("Weekly on Mon");
  });

  test("formats multiple weekdays in day-of-week order regardless of input order", () => {
    const frequency: RecurrenceFrequency = { kind: "Weekly", weekdays: [5, 1, 3], interval_weeks: 1 };
    expect(formatRecurrenceFrequency(frequency)).toBe("Weekly on Mon, Wed, Fri");
  });

  test("formats an every-other-week interval", () => {
    const frequency: RecurrenceFrequency = { kind: "Weekly", weekdays: [6], interval_weeks: 2 };
    expect(formatRecurrenceFrequency(frequency)).toBe("Every 2 weeks on Sat");
  });

  test("formats the Mon-Fri shortcut as 'weekdays'", () => {
    const frequency: RecurrenceFrequency = { kind: "Weekly", weekdays: [1, 2, 3, 4, 5], interval_weeks: 1 };
    expect(formatRecurrenceFrequency(frequency)).toBe("Weekly on weekdays");
  });

  test("formats the Sat+Sun shortcut as 'weekends'", () => {
    const frequency: RecurrenceFrequency = { kind: "Weekly", weekdays: [0, 6], interval_weeks: 1 };
    expect(formatRecurrenceFrequency(frequency)).toBe("Weekly on weekends");
  });

  test("formats a day-of-month with the correct ordinal suffix", () => {
    expect(formatRecurrenceFrequency({ kind: "MonthlyByDay", day: 1 })).toBe("Monthly on the 1st");
    expect(formatRecurrenceFrequency({ kind: "MonthlyByDay", day: 2 })).toBe("Monthly on the 2nd");
    expect(formatRecurrenceFrequency({ kind: "MonthlyByDay", day: 3 })).toBe("Monthly on the 3rd");
    expect(formatRecurrenceFrequency({ kind: "MonthlyByDay", day: 4 })).toBe("Monthly on the 4th");
  });

  test("uses 'th' for the 11th-13th, not 'st'/'nd'/'rd'", () => {
    expect(formatRecurrenceFrequency({ kind: "MonthlyByDay", day: 11 })).toBe("Monthly on the 11th");
    expect(formatRecurrenceFrequency({ kind: "MonthlyByDay", day: 12 })).toBe("Monthly on the 12th");
    expect(formatRecurrenceFrequency({ kind: "MonthlyByDay", day: 13 })).toBe("Monthly on the 13th");
  });

  test("formats the 31st correctly", () => {
    expect(formatRecurrenceFrequency({ kind: "MonthlyByDay", day: 31 })).toBe("Monthly on the 31st");
  });
});

describe("formatDueRule", () => {
  test("formats Never", () => {
    expect(formatDueRule({ kind: "Never" })).toBe("Never");
  });

  test("formats a recognized default code via its existing label", () => {
    expect(formatDueRule({ kind: "DefaultCode", code: "next_day" })).toBe("Next day");
  });

  test("formats a zero-day offset as 'same day'", () => {
    expect(formatDueRule({ kind: "AfterScheduled", days: 0 })).toBe("Same day as each occurrence");
  });

  test("formats a positive one-day offset in the singular", () => {
    expect(formatDueRule({ kind: "AfterScheduled", days: 1 })).toBe("1 day after each occurrence");
  });

  test("formats a positive multi-day offset in the plural", () => {
    expect(formatDueRule({ kind: "AfterScheduled", days: 5 })).toBe("5 days after each occurrence");
  });

  test("formats a negative one-day offset in the singular", () => {
    expect(formatDueRule({ kind: "AfterScheduled", days: -1 })).toBe("1 day before each occurrence");
  });

  test("formats a negative multi-day offset in the plural", () => {
    expect(formatDueRule({ kind: "AfterScheduled", days: -3 })).toBe("3 days before each occurrence");
  });

  test("formats a weekday rule with interval 1 as 'Every <weekday>'", () => {
    const rule: DueRule = { kind: "Weekday", weekday: 1, interval_weeks: 1 };
    expect(formatDueRule(rule)).toBe("Every Mon");
  });

  test("formats a weekday rule with interval 2 as 'Every other <weekday>'", () => {
    const rule: DueRule = { kind: "Weekday", weekday: 5, interval_weeks: 2 };
    expect(formatDueRule(rule)).toBe("Every other Fri");
  });
});

describe("daysBetween", () => {
  test("returns 0 for the same date", () => {
    expect(daysBetween("2026-06-15", "2026-06-15")).toBe(0);
  });

  test("returns a positive count when the second date is later", () => {
    expect(daysBetween("2026-06-15", "2026-06-20")).toBe(5);
  });

  test("returns a negative count when the second date is earlier", () => {
    expect(daysBetween("2026-06-20", "2026-06-15")).toBe(-5);
  });

  test("correctly counts across a month boundary", () => {
    expect(daysBetween("2026-06-28", "2026-07-02")).toBe(4);
  });
});

describe("resolveDueRule", () => {
  test("Never resolves to undefined", () => {
    expect(resolveDueRule({ kind: "Never" }, "2026-06-15")).toBeUndefined();
  });

  test("DefaultCode resolves via the existing relative-date resolver", () => {
    expect(resolveDueRule({ kind: "DefaultCode", code: "next_day" }, "2026-06-15")).toBe("2026-06-16");
  });

  test("DefaultCode 'none' resolves to undefined", () => {
    expect(resolveDueRule({ kind: "DefaultCode", code: "none" }, "2026-06-15")).toBeUndefined();
  });

  test("AfterScheduled zero days is the same day", () => {
    expect(resolveDueRule({ kind: "AfterScheduled", days: 0 }, "2026-06-15")).toBe("2026-06-15");
  });

  test("AfterScheduled positive days is later", () => {
    expect(resolveDueRule({ kind: "AfterScheduled", days: 5 }, "2026-06-15")).toBe("2026-06-20");
  });

  test("AfterScheduled negative days is earlier", () => {
    expect(resolveDueRule({ kind: "AfterScheduled", days: -3 }, "2026-06-15")).toBe("2026-06-12");
  });

  test("Weekday rule resolves to the same day when scheduled is already that weekday", () => {
    // 2026-06-15 is a Monday.
    expect(resolveDueRule({ kind: "Weekday", weekday: 1, interval_weeks: 1 }, "2026-06-15")).toBe("2026-06-15");
  });

  test("Weekday rule resolves to the next occurrence of that weekday", () => {
    // 2026-06-15 is a Monday; the next Friday is 2026-06-19.
    expect(resolveDueRule({ kind: "Weekday", weekday: 5, interval_weeks: 1 }, "2026-06-15")).toBe("2026-06-19");
  });

  test("Weekday rule wraps around to next week when the weekday already passed", () => {
    // 2026-06-19 is a Friday; the next Monday is 2026-06-22, not earlier.
    expect(resolveDueRule({ kind: "Weekday", weekday: 1, interval_weeks: 1 }, "2026-06-19")).toBe("2026-06-22");
  });

  test("Weekday rule with interval 2 skips the next occurrence", () => {
    // 2026-06-15 is a Monday; the next Friday is 2026-06-19, the one after is 2026-06-26.
    expect(resolveDueRule({ kind: "Weekday", weekday: 5, interval_weeks: 2 }, "2026-06-15")).toBe("2026-06-26");
  });
});

describe("resolveSeriesDueRule", () => {
  test("uses an already-structured dueRule as-is, ignoring due", () => {
    const rule: DueRule = { kind: "AfterScheduled", days: 5 };

    expect(resolveSeriesDueRule(undefined, rule, "2026-06-15")).toEqual(rule);
  });

  test("a dueRule of Weekday is also used as-is", () => {
    const rule: DueRule = { kind: "Weekday", weekday: 1, interval_weeks: 2 };

    expect(resolveSeriesDueRule(undefined, rule, "2026-06-15")).toEqual(rule);
  });

  test("due === 'none' with no dueRule resolves to Never", () => {
    expect(resolveSeriesDueRule("none", undefined, "2026-06-15")).toEqual({ kind: "Never" });
  });

  test("a resolved absolute due date becomes a generic AfterScheduled offset from scheduled", () => {
    expect(resolveSeriesDueRule("2026-06-20", undefined, "2026-06-15")).toEqual({
      kind: "AfterScheduled",
      days: 5,
    });
  });

  test("a due date before the scheduled date produces a negative offset", () => {
    expect(resolveSeriesDueRule("2026-06-12", undefined, "2026-06-15")).toEqual({
      kind: "AfterScheduled",
      days: -3,
    });
  });

  test("a due date equal to the scheduled date produces a zero offset", () => {
    expect(resolveSeriesDueRule("2026-06-15", undefined, "2026-06-15")).toEqual({
      kind: "AfterScheduled",
      days: 0,
    });
  });

  test("neither due nor dueRule set resolves to undefined, deferring to the backend default", () => {
    expect(resolveSeriesDueRule(undefined, undefined, "2026-06-15")).toBeUndefined();
  });
});

describe("resolveNonRecurringDue", () => {
  test("an already-resolved absolute due date is used as-is, ignoring any dueRule", () => {
    expect(resolveNonRecurringDue("2026-06-20", { kind: "AfterScheduled", days: 5 }, "2026-06-15")).toBe(
      "2026-06-20",
    );
  });

  test("the 'none' sentinel is used as-is, ignoring any dueRule", () => {
    expect(resolveNonRecurringDue("none", { kind: "AfterScheduled", days: 5 }, "2026-06-15")).toBe("none");
  });

  test("an AfterScheduled dueRule with no due set resolves to an absolute date relative to scheduled", () => {
    // This is the bug this function exists to fix: 'due in 5 days' on a
    // non-recurring task resolves to `dueRule`, not `due` — without this
    // resolution, createTask (which only reads `due`) would silently get
    // no due date at all despite the preview showing one.
    expect(resolveNonRecurringDue(undefined, { kind: "AfterScheduled", days: 5 }, "2026-06-15")).toBe("2026-06-20");
  });

  test("a Weekday dueRule with no due set resolves to an absolute date relative to scheduled", () => {
    // 2026-06-15 is a Monday; the next Friday is 2026-06-19.
    expect(resolveNonRecurringDue(undefined, { kind: "Weekday", weekday: 5, interval_weeks: 1 }, "2026-06-15")).toBe(
      "2026-06-19",
    );
  });

  test("neither due nor dueRule set resolves to undefined", () => {
    expect(resolveNonRecurringDue(undefined, undefined, "2026-06-15")).toBeUndefined();
  });
});

describe("dueRuleFromDefaultCode", () => {
  test("wraps a resolved code as DefaultCode", () => {
    expect(dueRuleFromDefaultCode("next_day")).toEqual({ kind: "DefaultCode", code: "next_day" });
  });

  test("resolves to Never when there's no configured default at all", () => {
    expect(dueRuleFromDefaultCode(undefined)).toEqual({ kind: "Never" });
  });
});

describe("clampPositiveInteger", () => {
  test("passes through an already-valid positive integer", () => {
    expect(clampPositiveInteger(5)).toBe(5);
  });

  test("truncates a fractional value", () => {
    expect(clampPositiveInteger(2.7)).toBe(2);
  });

  test("defaults to 1 for NaN (e.g. a cleared number input)", () => {
    expect(clampPositiveInteger(NaN)).toBe(1);
  });

  test("defaults to 1 for zero", () => {
    expect(clampPositiveInteger(0)).toBe(1);
  });

  test("defaults to 1 for a negative value", () => {
    expect(clampPositiveInteger(-3)).toBe(1);
  });

  test("defaults to 1 for Infinity", () => {
    expect(clampPositiveInteger(Infinity)).toBe(1);
  });
});

describe("clampNonNegativeInteger", () => {
  test("passes through an already-valid non-negative integer", () => {
    expect(clampNonNegativeInteger(5)).toBe(5);
  });

  test("passes through zero", () => {
    expect(clampNonNegativeInteger(0)).toBe(0);
  });

  test("truncates a fractional value", () => {
    expect(clampNonNegativeInteger(2.7)).toBe(2);
  });

  test("defaults to 0 for NaN", () => {
    expect(clampNonNegativeInteger(NaN)).toBe(0);
  });

  test("defaults to 0 for a negative value", () => {
    expect(clampNonNegativeInteger(-3)).toBe(0);
  });
});

describe("clampDayOfMonth", () => {
  test("passes through an already-valid day", () => {
    expect(clampDayOfMonth(15)).toBe(15);
  });

  test("clamps a value above 31 down to 31", () => {
    expect(clampDayOfMonth(45)).toBe(31);
  });

  test("clamps a value below 1 up to 1", () => {
    expect(clampDayOfMonth(0)).toBe(1);
    expect(clampDayOfMonth(-5)).toBe(1);
  });

  test("truncates a fractional value", () => {
    expect(clampDayOfMonth(15.9)).toBe(15);
  });

  test("defaults to 1 for NaN", () => {
    expect(clampDayOfMonth(NaN)).toBe(1);
  });
});
