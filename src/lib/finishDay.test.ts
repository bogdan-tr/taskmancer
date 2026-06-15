import { describe, expect, test } from "vitest";
import { formatFinishDayResult } from "./finishDay";

describe("formatFinishDayResult", () => {
  test("uses singular noun for exactly one task", () => {
    expect(formatFinishDayResult(1)).toBe("1 task archived");
  });

  test("uses plural noun for zero tasks", () => {
    expect(formatFinishDayResult(0)).toBe("0 tasks archived");
  });

  test("uses plural noun for multiple tasks", () => {
    expect(formatFinishDayResult(5)).toBe("5 tasks archived");
  });
});
