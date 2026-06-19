import { describe, expect, it } from "vitest";
import { addMonths, monthDates, startOfMonth } from "./monthRange";
import { formatDateISO } from "./weekRange";

describe("monthRange", () => {
  describe("startOfMonth", () => {
    it("returns the 1st of the month for a mid-month date", () => {
      expect(formatDateISO(startOfMonth(new Date(2024, 0, 15)))).toBe("2024-01-01");
    });

    it("returns the same date when already the 1st", () => {
      expect(formatDateISO(startOfMonth(new Date(2024, 1, 1)))).toBe("2024-02-01");
    });

    it("returns the 1st for the last day of the month", () => {
      expect(formatDateISO(startOfMonth(new Date(2024, 1, 29)))).toBe("2024-02-01");
    });
  });

  describe("addMonths", () => {
    it("adds a positive number of months within the same year", () => {
      expect(formatDateISO(addMonths(new Date(2024, 0, 1), 1))).toBe("2024-02-01");
    });

    it("adds months across a year boundary", () => {
      expect(formatDateISO(addMonths(new Date(2024, 11, 1), 1))).toBe("2025-01-01");
    });

    it("subtracts months across a year boundary", () => {
      expect(formatDateISO(addMonths(new Date(2024, 0, 1), -1))).toBe("2023-12-01");
    });

    it("does not mutate the input date", () => {
      const original = new Date(2024, 0, 1);
      addMonths(original, 5);
      expect(formatDateISO(original)).toBe("2024-01-01");
    });
  });

  describe("monthDates", () => {
    it("returns a number of dates that's a multiple of 7", () => {
      const dates = monthDates(new Date(2024, 0, 1), "monday");
      expect(dates.length % 7).toBe(0);
    });

    it("includes every day of the month exactly once", () => {
      const dates = monthDates(new Date(2024, 0, 1), "monday");
      const januaryDates = dates.filter((d) => d.getFullYear() === 2024 && d.getMonth() === 0);
      expect(januaryDates).toHaveLength(31);
    });

    it("includes every day of a leap-year February", () => {
      const dates = monthDates(new Date(2024, 1, 1), "monday");
      const februaryDates = dates.filter((d) => d.getFullYear() === 2024 && d.getMonth() === 1);
      expect(februaryDates).toHaveLength(29);
    });

    it("starts the grid on the configured week-start day", () => {
      const mondayStart = monthDates(new Date(2024, 0, 1), "monday");
      expect(mondayStart[0].getDay()).toBe(1);

      const sundayStart = monthDates(new Date(2024, 0, 1), "sunday");
      expect(sundayStart[0].getDay()).toBe(0);
    });

    it("ends the grid on the day before the configured week-start day", () => {
      const mondayStart = monthDates(new Date(2024, 0, 1), "monday");
      expect(mondayStart[mondayStart.length - 1].getDay()).toBe(0); // Sunday

      const sundayStart = monthDates(new Date(2024, 0, 1), "sunday");
      expect(sundayStart[sundayStart.length - 1].getDay()).toBe(6); // Saturday
    });

    it("matches the exact known grid for January 2024 (Monday-start)", () => {
      // 2024-01-01 is already a Monday, so no leading days are needed; the
      // month's last week (Jan 29 - Feb 4) still needs trailing days to complete.
      const dates = monthDates(new Date(2024, 0, 1), "monday");
      expect(dates).toHaveLength(35);
      expect(formatDateISO(dates[0])).toBe("2024-01-01");
      expect(formatDateISO(dates[dates.length - 1])).toBe("2024-02-04");
    });

    it("matches the exact known grid for January 2024 (Sunday-start)", () => {
      const dates = monthDates(new Date(2024, 0, 1), "sunday");
      expect(dates).toHaveLength(35);
      expect(formatDateISO(dates[0])).toBe("2023-12-31");
      expect(formatDateISO(dates[dates.length - 1])).toBe("2024-02-03");
    });
  });
});
