import { describe, expect, test } from "vitest";
import { formatHms } from "./liveTimer";

describe("formatHms", () => {
  test("formats zero seconds", () => {
    expect(formatHms(0)).toBe("0:00");
  });

  test("formats a single-digit seconds value with a leading zero", () => {
    expect(formatHms(45)).toBe("0:45");
  });

  test("formats 59 seconds without rolling over to minutes", () => {
    expect(formatHms(59)).toBe("0:59");
  });

  test("formats exactly one minute", () => {
    expect(formatHms(60)).toBe("1:00");
  });

  test("formats minutes and seconds together", () => {
    expect(formatHms(90)).toBe("1:30");
  });

  test("formats just under an hour without rolling over to hours", () => {
    expect(formatHms(3599)).toBe("59:59");
  });

  test("formats exactly one hour with zero-padded minutes and seconds", () => {
    expect(formatHms(3600)).toBe("1:00:00");
  });

  test("formats hours, minutes, and seconds together", () => {
    expect(formatHms(3661)).toBe("1:01:01");
  });

  test("zero-pads single-digit minutes in the H:MM:SS form", () => {
    expect(formatHms(3605)).toBe("1:00:05");
  });

  test("formats multiple hours", () => {
    expect(formatHms(7384)).toBe("2:03:04");
  });

  test("clamps a negative value to zero", () => {
    expect(formatHms(-10)).toBe("0:00");
  });

  test("clamps NaN to zero instead of propagating it", () => {
    expect(formatHms(NaN)).toBe("0:00");
  });

  test("clamps Infinity to zero", () => {
    expect(formatHms(Infinity)).toBe("0:00");
  });

  test("floors a fractional seconds value", () => {
    expect(formatHms(90.9)).toBe("1:30");
  });
});
