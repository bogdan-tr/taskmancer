import { describe, expect, test } from "vitest";
import {
  formatMinutes,
  hoursAndMinutesFromMinutes,
  minutesFromHoursAndMinutes,
  normalizeHoursMinutes,
} from "./estimatedTime";

describe("minutesFromHoursAndMinutes", () => {
  test("combines hours and minutes into a total", () => {
    expect(minutesFromHoursAndMinutes(1, 30)).toBe(90);
  });

  test("handles zero hours", () => {
    expect(minutesFromHoursAndMinutes(0, 45)).toBe(45);
  });

  test("handles zero minutes", () => {
    expect(minutesFromHoursAndMinutes(2, 0)).toBe(120);
  });

  test("does not roll over on its own — combining is purely additive", () => {
    expect(minutesFromHoursAndMinutes(1, 90)).toBe(150);
  });

  test("clamps negative hours to zero", () => {
    expect(minutesFromHoursAndMinutes(-1, 30)).toBe(30);
  });

  test("clamps negative minutes to zero", () => {
    expect(minutesFromHoursAndMinutes(1, -30)).toBe(60);
  });

  test("treats a NaN hours value as zero instead of propagating NaN", () => {
    expect(minutesFromHoursAndMinutes(NaN, 30)).toBe(30);
  });

  test("treats a NaN minutes value as zero instead of propagating NaN", () => {
    expect(minutesFromHoursAndMinutes(1, NaN)).toBe(60);
  });
});

describe("hoursAndMinutesFromMinutes", () => {
  test("splits a total into hours and minutes", () => {
    expect(hoursAndMinutesFromMinutes(90)).toEqual({ hours: 1, minutes: 30 });
  });

  test("returns zero hours for a total under 60", () => {
    expect(hoursAndMinutesFromMinutes(45)).toEqual({ hours: 0, minutes: 45 });
  });

  test("returns zero minutes for an exact multiple of 60", () => {
    expect(hoursAndMinutesFromMinutes(120)).toEqual({ hours: 2, minutes: 0 });
  });

  test("returns zero/zero for zero", () => {
    expect(hoursAndMinutesFromMinutes(0)).toEqual({ hours: 0, minutes: 0 });
  });

  test("clamps a negative total to zero", () => {
    expect(hoursAndMinutesFromMinutes(-10)).toEqual({ hours: 0, minutes: 0 });
  });

  test("floors a fractional total", () => {
    expect(hoursAndMinutesFromMinutes(90.7)).toEqual({ hours: 1, minutes: 30 });
  });

  test("treats a NaN total as zero instead of propagating NaN", () => {
    expect(hoursAndMinutesFromMinutes(NaN)).toEqual({ hours: 0, minutes: 0 });
  });
});

describe("normalizeHoursMinutes", () => {
  test("rolls 90 minutes over into 1 hour 30 minutes", () => {
    expect(normalizeHoursMinutes(0, 90)).toEqual({ hours: 1, minutes: 30 });
  });

  test("rolls minutes on top of existing hours", () => {
    expect(normalizeHoursMinutes(1, 90)).toEqual({ hours: 2, minutes: 30 });
  });

  test("leaves an already-normalized value unchanged", () => {
    expect(normalizeHoursMinutes(2, 15)).toEqual({ hours: 2, minutes: 15 });
  });

  test("rolls an exact multiple of 60 minutes into whole hours with zero remainder", () => {
    expect(normalizeHoursMinutes(0, 120)).toEqual({ hours: 2, minutes: 0 });
  });
});

describe("formatMinutes", () => {
  test("formats hours and minutes together", () => {
    expect(formatMinutes(90)).toBe("1h 30m");
  });

  test("formats whole hours with no minutes", () => {
    expect(formatMinutes(120)).toBe("2h");
  });

  test("formats minutes only when under an hour", () => {
    expect(formatMinutes(45)).toBe("45m");
  });

  test("formats zero explicitly as 0m", () => {
    expect(formatMinutes(0)).toBe("0m");
  });
});
