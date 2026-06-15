import { describe, expect, it } from "vitest";
import { addDays, addWeeks, formatDateISO, startOfWeek, weekDates } from "./weekRange";

describe("weekRange", () => {
  describe("formatDateISO", () => {
    it("pads single-digit months and days", () => {
      expect(formatDateISO(new Date(2024, 0, 1))).toBe("2024-01-01");
    });

    it("formats double-digit months and days without extra padding", () => {
      expect(formatDateISO(new Date(2024, 8, 5))).toBe("2024-09-05");
      expect(formatDateISO(new Date(2024, 11, 31))).toBe("2024-12-31");
    });
  });

  describe("addDays", () => {
    it("adds positive days within the same month", () => {
      const result = addDays(new Date(2024, 0, 1), 5);
      expect(formatDateISO(result)).toBe("2024-01-06");
    });

    it("subtracts days across a year boundary", () => {
      const result = addDays(new Date(2024, 0, 1), -1);
      expect(formatDateISO(result)).toBe("2023-12-31");
    });

    it("adds days across a year boundary", () => {
      const result = addDays(new Date(2024, 11, 31), 1);
      expect(formatDateISO(result)).toBe("2025-01-01");
    });

    it("does not mutate the input date", () => {
      const original = new Date(2024, 0, 1);
      addDays(original, 10);
      expect(formatDateISO(original)).toBe("2024-01-01");
    });
  });

  describe("addWeeks", () => {
    it("adds a positive number of weeks", () => {
      const result = addWeeks(new Date(2024, 0, 1), 1);
      expect(formatDateISO(result)).toBe("2024-01-08");
    });

    it("subtracts weeks across a year boundary", () => {
      const result = addWeeks(new Date(2024, 0, 1), -1);
      expect(formatDateISO(result)).toBe("2023-12-25");
    });
  });

  describe("startOfWeek", () => {
    // 2024-01-01 is a Monday, so 2023-12-31 is a Sunday.
    it("returns the same date when it is already the Monday start", () => {
      expect(formatDateISO(startOfWeek(new Date(2024, 0, 1), "monday"))).toBe("2024-01-01");
    });

    it("returns the preceding Monday for a mid-week date", () => {
      expect(formatDateISO(startOfWeek(new Date(2024, 0, 3), "monday"))).toBe("2024-01-01");
    });

    it("returns the preceding Monday for a Sunday, per Monday-start convention", () => {
      expect(formatDateISO(startOfWeek(new Date(2024, 0, 7), "monday"))).toBe("2024-01-01");
    });

    it("returns the same date when it is already the Sunday start", () => {
      expect(formatDateISO(startOfWeek(new Date(2024, 0, 7), "sunday"))).toBe("2024-01-07");
    });

    it("returns the preceding Sunday for a mid-week date, per Sunday-start convention", () => {
      expect(formatDateISO(startOfWeek(new Date(2024, 0, 3), "sunday"))).toBe("2023-12-31");
    });
  });

  describe("weekDates", () => {
    it("returns 7 consecutive dates starting at weekStart", () => {
      const dates = weekDates(new Date(2024, 0, 1));
      expect(dates).toHaveLength(7);
      expect(dates.map(formatDateISO)).toEqual([
        "2024-01-01",
        "2024-01-02",
        "2024-01-03",
        "2024-01-04",
        "2024-01-05",
        "2024-01-06",
        "2024-01-07",
      ]);
    });

    it("spans a month boundary correctly", () => {
      const dates = weekDates(new Date(2024, 0, 29));
      expect(dates.map(formatDateISO)).toEqual([
        "2024-01-29",
        "2024-01-30",
        "2024-01-31",
        "2024-02-01",
        "2024-02-02",
        "2024-02-03",
        "2024-02-04",
      ]);
    });
  });
});
