import { describe, expect, test } from "vitest";
import { RELATIVE_DATE_OPTIONS, relativeDateLabel } from "./relativeDates";

describe("RELATIVE_DATE_OPTIONS", () => {
  test("contains the six fixed relative-date codes matching the Rust side", () => {
    expect(RELATIVE_DATE_OPTIONS.map((option) => option.id)).toEqual([
      "today",
      "tomorrow",
      "in_2_days",
      "in_3_days",
      "in_1_week",
      "in_1_month",
    ]);
  });

  test("every option has a non-empty label", () => {
    expect(RELATIVE_DATE_OPTIONS.every((option) => option.label.length > 0)).toBe(true);
  });
});

describe("relativeDateLabel", () => {
  test("returns the label for a known option id", () => {
    expect(relativeDateLabel("tomorrow")).toBe("Tomorrow");
  });

  test("returns the id itself for an unrecognized option id", () => {
    expect(relativeDateLabel("next_quarter")).toBe("next_quarter");
  });
});
