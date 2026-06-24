import { describe, expect, it } from "vitest";
import {
  datetimeLocalValueToIso,
  formatEntryDuration,
  formatEntryTimestamp,
  isoToDatetimeLocalValue,
  validateManualEntryRange,
} from "./timeLogDisplay";

describe("formatEntryTimestamp", () => {
  it("formats a valid RFC3339 timestamp as a readable local date/time", () => {
    const result = formatEntryTimestamp("2026-06-15T09:00:00.000Z");

    expect(result).not.toBe("Invalid date");
    expect(result).toContain("2026");
  });

  it("returns 'Invalid date' for an unparseable input", () => {
    expect(formatEntryTimestamp("not-a-date")).toBe("Invalid date");
  });

  it("returns 'Invalid date' for an empty string", () => {
    expect(formatEntryTimestamp("")).toBe("Invalid date");
  });
});

describe("formatEntryDuration", () => {
  it("formats a 90-minute span as '1h 30m'", () => {
    expect(formatEntryDuration("2026-06-15T09:00:00.000Z", "2026-06-15T10:30:00.000Z")).toBe("1h 30m");
  });

  it("formats a sub-hour span in minutes only", () => {
    expect(formatEntryDuration("2026-06-15T09:00:00.000Z", "2026-06-15T09:45:00.000Z")).toBe("45m");
  });

  it("rounds to the nearest whole minute", () => {
    // 90 seconds rounds to 2 minutes (1.5 rounds up).
    expect(formatEntryDuration("2026-06-15T09:00:00.000Z", "2026-06-15T09:01:30.000Z")).toBe("2m");
  });

  it("returns '0m' when ended_at is before started_at", () => {
    expect(formatEntryDuration("2026-06-15T10:00:00.000Z", "2026-06-15T09:00:00.000Z")).toBe("0m");
  });

  it("returns '0m' when ended_at equals started_at", () => {
    expect(formatEntryDuration("2026-06-15T09:00:00.000Z", "2026-06-15T09:00:00.000Z")).toBe("0m");
  });

  it("returns '0m' for an unparseable started_at", () => {
    expect(formatEntryDuration("not-a-date", "2026-06-15T09:00:00.000Z")).toBe("0m");
  });

  it("returns '0m' for an unparseable ended_at", () => {
    expect(formatEntryDuration("2026-06-15T09:00:00.000Z", "not-a-date")).toBe("0m");
  });

  it("returns '0m' for a multi-day span rounded down to whole hours/minutes (24h example)", () => {
    expect(formatEntryDuration("2026-06-15T09:00:00.000Z", "2026-06-16T09:00:00.000Z")).toBe("24h");
  });
});

describe("validateManualEntryRange", () => {
  it("returns undefined for a valid range where end is strictly after start", () => {
    expect(validateManualEntryRange("2026-06-15T09:00:00.000Z", "2026-06-15T10:00:00.000Z")).toBeUndefined();
  });

  it("returns an error message when end equals start", () => {
    expect(validateManualEntryRange("2026-06-15T09:00:00.000Z", "2026-06-15T09:00:00.000Z")).toBe(
      "End must be after start",
    );
  });

  it("returns an error message when end is before start", () => {
    expect(validateManualEntryRange("2026-06-15T10:00:00.000Z", "2026-06-15T09:00:00.000Z")).toBe(
      "End must be after start",
    );
  });

  it("returns an error message for an invalid start", () => {
    expect(validateManualEntryRange("not-a-date", "2026-06-15T09:00:00.000Z")).toBe("Start date/time is invalid");
  });

  it("returns an error message for an invalid end", () => {
    expect(validateManualEntryRange("2026-06-15T09:00:00.000Z", "not-a-date")).toBe("End date/time is invalid");
  });

  it("returns an error message for both empty", () => {
    expect(validateManualEntryRange("", "")).toBe("Start date/time is invalid");
  });
});

describe("isoToDatetimeLocalValue", () => {
  it("formats a valid timestamp as YYYY-MM-DDTHH:mm", () => {
    const result = isoToDatetimeLocalValue("2026-06-15T09:05:00.000Z");

    expect(result).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/);
  });

  it("returns an empty string for an unparseable input", () => {
    expect(isoToDatetimeLocalValue("not-a-date")).toBe("");
  });

  it("returns an empty string for an empty input", () => {
    expect(isoToDatetimeLocalValue("")).toBe("");
  });

  it("round-trips through datetimeLocalValueToIso back to an equivalent instant", () => {
    const original = "2026-06-15T09:05:00.000Z";
    const localValue = isoToDatetimeLocalValue(original);
    const roundTripped = datetimeLocalValueToIso(localValue);

    expect(roundTripped).toBeDefined();
    expect(Date.parse(roundTripped as string)).toBe(Date.parse(original));
  });
});

describe("datetimeLocalValueToIso", () => {
  it("converts a valid datetime-local value to an RFC3339 instant", () => {
    const result = datetimeLocalValueToIso("2026-06-15T09:05");

    expect(result).toBeDefined();
    expect(Number.isNaN(Date.parse(result as string))).toBe(false);
  });

  it("returns undefined for an empty string", () => {
    expect(datetimeLocalValueToIso("")).toBeUndefined();
  });

  it("returns undefined for an unparseable value", () => {
    expect(datetimeLocalValueToIso("not-a-date")).toBeUndefined();
  });
});
