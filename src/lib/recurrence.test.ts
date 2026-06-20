import { describe, expect, test } from "vitest";
import { formatRecurrenceFrequency, type RecurrenceFrequency } from "./recurrence";

describe("formatRecurrenceFrequency", () => {
  test("formats a daily interval", () => {
    expect(formatRecurrenceFrequency({ kind: "EveryNDays", interval: 1 })).toBe("Daily");
  });

  test("formats an every-other-day interval", () => {
    expect(formatRecurrenceFrequency({ kind: "EveryNDays", interval: 2 })).toBe("Every other day");
  });

  test("formats an arbitrary day interval", () => {
    expect(formatRecurrenceFrequency({ kind: "EveryNDays", interval: 5 })).toBe("Every 5 days");
  });

  test("formats a single weekday", () => {
    const frequency: RecurrenceFrequency = { kind: "Weekly", weekdays: [1], interval_weeks: 1 };
    expect(formatRecurrenceFrequency(frequency)).toBe("Weekly on Mon");
  });

  test("formats multiple weekdays in day-of-week order regardless of input order", () => {
    const frequency: RecurrenceFrequency = { kind: "Weekly", weekdays: [5, 1, 3], interval_weeks: 1 };
    expect(formatRecurrenceFrequency(frequency)).toBe("Weekly on Mon, Wed, Fri");
  });

  test("formats an every-other-week interval", () => {
    const frequency: RecurrenceFrequency = { kind: "Weekly", weekdays: [6], interval_weeks: 2 };
    expect(formatRecurrenceFrequency(frequency)).toBe("Every 2 weeks on Sat");
  });

  test("formats the Mon-Fri shortcut as 'weekdays'", () => {
    const frequency: RecurrenceFrequency = { kind: "Weekly", weekdays: [1, 2, 3, 4, 5], interval_weeks: 1 };
    expect(formatRecurrenceFrequency(frequency)).toBe("Weekly on weekdays");
  });

  test("formats the Sat+Sun shortcut as 'weekends'", () => {
    const frequency: RecurrenceFrequency = { kind: "Weekly", weekdays: [0, 6], interval_weeks: 1 };
    expect(formatRecurrenceFrequency(frequency)).toBe("Weekly on weekends");
  });

  test("formats a day-of-month with the correct ordinal suffix", () => {
    expect(formatRecurrenceFrequency({ kind: "MonthlyByDay", day: 1 })).toBe("Monthly on the 1st");
    expect(formatRecurrenceFrequency({ kind: "MonthlyByDay", day: 2 })).toBe("Monthly on the 2nd");
    expect(formatRecurrenceFrequency({ kind: "MonthlyByDay", day: 3 })).toBe("Monthly on the 3rd");
    expect(formatRecurrenceFrequency({ kind: "MonthlyByDay", day: 4 })).toBe("Monthly on the 4th");
  });

  test("uses 'th' for the 11th-13th, not 'st'/'nd'/'rd'", () => {
    expect(formatRecurrenceFrequency({ kind: "MonthlyByDay", day: 11 })).toBe("Monthly on the 11th");
    expect(formatRecurrenceFrequency({ kind: "MonthlyByDay", day: 12 })).toBe("Monthly on the 12th");
    expect(formatRecurrenceFrequency({ kind: "MonthlyByDay", day: 13 })).toBe("Monthly on the 13th");
  });

  test("formats the 31st correctly", () => {
    expect(formatRecurrenceFrequency({ kind: "MonthlyByDay", day: 31 })).toBe("Monthly on the 31st");
  });
});
