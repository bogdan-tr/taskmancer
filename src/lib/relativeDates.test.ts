import { describe, expect, test } from "vitest";
import {
  DUE_RELATIVE_DATE_OPTIONS,
  SCHEDULED_RELATIVE_DATE_OPTIONS,
  dueRelativeDateLabel,
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
