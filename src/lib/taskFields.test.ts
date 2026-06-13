import { describe, expect, test } from "vitest";
import { emptyToUndefined, formatTags, isValidOptionalDate, parseTags } from "./taskFields";

describe("parseTags", () => {
  test("splits comma-separated tags and trims whitespace", () => {
    expect(parseTags(" urgent, reading ,  home")).toEqual(["urgent", "reading", "home"]);
  });

  test("drops empty entries from repeated or trailing commas", () => {
    expect(parseTags("urgent,, reading, ")).toEqual(["urgent", "reading"]);
  });

  test("returns an empty array for an empty string", () => {
    expect(parseTags("")).toEqual([]);
  });

  test("returns an empty array for whitespace-only input", () => {
    expect(parseTags("   ")).toEqual([]);
  });
});

describe("formatTags", () => {
  test("joins tags with a comma and space", () => {
    expect(formatTags(["urgent", "reading"])).toBe("urgent, reading");
  });

  test("returns an empty string for an empty list", () => {
    expect(formatTags([])).toBe("");
  });
});

describe("emptyToUndefined", () => {
  test("returns undefined for an empty string", () => {
    expect(emptyToUndefined("")).toBeUndefined();
  });

  test("returns undefined for whitespace-only input", () => {
    expect(emptyToUndefined("   ")).toBeUndefined();
  });

  test("returns the trimmed string when non-empty", () => {
    expect(emptyToUndefined("  2026-07-01 ")).toBe("2026-07-01");
  });
});

describe("isValidOptionalDate", () => {
  test("accepts an empty string", () => {
    expect(isValidOptionalDate("")).toBe(true);
  });

  test("accepts a whitespace-only string", () => {
    expect(isValidOptionalDate("   ")).toBe(true);
  });

  test("accepts a YYYY-MM-DD date", () => {
    expect(isValidOptionalDate("2026-07-01")).toBe(true);
  });

  test("rejects a non-ISO date format", () => {
    expect(isValidOptionalDate("07/01/2026")).toBe(false);
  });

  test("rejects a string that isn't a date at all", () => {
    expect(isValidOptionalDate("tomorrow")).toBe(false);
  });
});
