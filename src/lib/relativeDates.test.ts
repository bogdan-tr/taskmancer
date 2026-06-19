import { describe, expect, test } from "vitest";
import {
  DUE_RELATIVE_DATE_OPTIONS,
  SCHEDULED_RELATIVE_DATE_OPTIONS,
  dueRelativeDateLabel,
  resolveDueRelativeDate,
  resolveScheduledRelativeDate,
  scheduledRelativeDateLabel,
} from "./relativeDates";

describe("SCHEDULED_RELATIVE_DATE_OPTIONS", () => {
  test("contains the six fixed scheduled-date codes matching the Rust side", () => {
    expect(SCHEDULED_RELATIVE_DATE_OPTIONS.map((option) => option.id)).toEqual([
      "today",
      "tomorrow",
      "in_2_days",
      "in_3_days",
      "in_1_week",
      "in_1_month",
    ]);
  });

  test("every option has a non-empty label", () => {
    expect(SCHEDULED_RELATIVE_DATE_OPTIONS.every((option) => option.label.length > 0)).toBe(true);
  });
});

describe("DUE_RELATIVE_DATE_OPTIONS", () => {
  test("contains the seven fixed due-date codes matching the Rust side, starting with the 'none' sentinel", () => {
    expect(DUE_RELATIVE_DATE_OPTIONS.map((option) => option.id)).toEqual([
      "none",
      "same_day",
      "next_day",
      "in_2_days",
      "in_3_days",
      "in_1_week",
      "in_1_month",
    ]);
  });

  test("every option has a non-empty label", () => {
    expect(DUE_RELATIVE_DATE_OPTIONS.every((option) => option.label.length > 0)).toBe(true);
  });
});

describe("scheduledRelativeDateLabel", () => {
  test("returns the label for a known option id", () => {
    expect(scheduledRelativeDateLabel("tomorrow")).toBe("Tomorrow");
  });

  test("returns the id itself for an unrecognized option id", () => {
    expect(scheduledRelativeDateLabel("next_quarter")).toBe("next_quarter");
  });
});

describe("dueRelativeDateLabel", () => {
  test("returns 'Never' for the 'none' sentinel", () => {
    expect(dueRelativeDateLabel("none")).toBe("Never");
  });

  test("returns the label for a known option id", () => {
    expect(dueRelativeDateLabel("next_day")).toBe("Next day");
  });

  test("returns the id itself for an unrecognized option id", () => {
    expect(dueRelativeDateLabel("next_quarter")).toBe("next_quarter");
  });
});

describe("resolveScheduledRelativeDate", () => {
  const today = new Date(2026, 5, 14); // 2026-06-14

  test("resolves 'today' to today's date", () => {
    expect(resolveScheduledRelativeDate("today", today)).toBe("2026-06-14");
  });

  test("resolves 'tomorrow' to one day after today", () => {
    expect(resolveScheduledRelativeDate("tomorrow", today)).toBe("2026-06-15");
  });

  test("resolves 'in_2_days'/'in_3_days'/'in_1_week' as offsets from today", () => {
    expect(resolveScheduledRelativeDate("in_2_days", today)).toBe("2026-06-16");
    expect(resolveScheduledRelativeDate("in_3_days", today)).toBe("2026-06-17");
    expect(resolveScheduledRelativeDate("in_1_week", today)).toBe("2026-06-21");
  });

  test("resolves 'in_1_month' to the same day next month", () => {
    expect(resolveScheduledRelativeDate("in_1_month", today)).toBe("2026-07-14");
  });

  test("returns undefined for an unrecognized code", () => {
    expect(resolveScheduledRelativeDate("next_quarter", today)).toBeUndefined();
  });
});

describe("resolveDueRelativeDate", () => {
  const scheduled = "2026-06-15";

  test("returns undefined for the 'none' sentinel (never due)", () => {
    expect(resolveDueRelativeDate("none", scheduled)).toBeUndefined();
  });

  test("resolves 'same_day' to the scheduled date itself", () => {
    expect(resolveDueRelativeDate("same_day", scheduled)).toBe("2026-06-15");
  });

  test("resolves 'next_day' to one day after the scheduled date, not 'today'", () => {
    expect(resolveDueRelativeDate("next_day", scheduled)).toBe("2026-06-16");
  });

  test("resolves 'in_2_days'/'in_3_days'/'in_1_week' as offsets from the scheduled date", () => {
    expect(resolveDueRelativeDate("in_2_days", scheduled)).toBe("2026-06-17");
    expect(resolveDueRelativeDate("in_3_days", scheduled)).toBe("2026-06-18");
    expect(resolveDueRelativeDate("in_1_week", scheduled)).toBe("2026-06-22");
  });

  test("resolves 'in_1_month' to the same day next month, relative to scheduled", () => {
    expect(resolveDueRelativeDate("in_1_month", scheduled)).toBe("2026-07-15");
  });

  test("returns undefined for an unrecognized code", () => {
    expect(resolveDueRelativeDate("next_quarter", scheduled)).toBeUndefined();
  });
});
