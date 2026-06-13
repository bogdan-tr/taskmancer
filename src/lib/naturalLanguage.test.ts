import { describe, expect, test } from "vitest";
import { parseTaskInput } from "./naturalLanguage";

/** A fixed reference "now" so date-relative parsing is deterministic. Wednesday. */
const NOW = new Date(2026, 5, 10); // 2026-06-10 (June 10, 2026 is a Wednesday)

describe("parseTaskInput", () => {
  test("returns the input unchanged as the title when there are no tokens", () => {
    const result = parseTaskInput("Buy milk", NOW);

    expect(result).toEqual({
      title: "Buy milk",
      tags: [],
      project: undefined,
      priority: undefined,
      due: undefined,
    });
  });

  test("returns an empty title for empty input", () => {
    const result = parseTaskInput("", NOW);

    expect(result.title).toBe("");
    expect(result.tags).toEqual([]);
  });

  test("extracts a single hashtag as a tag", () => {
    const result = parseTaskInput("Buy milk #shopping", NOW);

    expect(result.title).toBe("Buy milk");
    expect(result.tags).toEqual(["shopping"]);
  });

  test("extracts multiple hashtags as tags", () => {
    const result = parseTaskInput("Buy milk #shopping #errand", NOW);

    expect(result.title).toBe("Buy milk");
    expect(result.tags).toEqual(["shopping", "errand"]);
  });

  test("deduplicates repeated hashtags", () => {
    const result = parseTaskInput("Buy milk #shopping #errand #shopping", NOW);

    expect(result.tags).toEqual(["shopping", "errand"]);
  });

  test("extracts a project from a +token", () => {
    const result = parseTaskInput("Plan trip +Vacation", NOW);

    expect(result.title).toBe("Plan trip");
    expect(result.project).toBe("Vacation");
  });

  test("uses the last +token when multiple are present", () => {
    const result = parseTaskInput("Plan trip +Work +Vacation", NOW);

    expect(result.project).toBe("Vacation");
  });

  test.each([
    ["!high", "high"],
    ["!medium", "medium"],
    ["!low", "low"],
    ["!HIGH", "high"],
    ["!h", "high"],
    ["!m", "medium"],
    ["!l", "low"],
  ])("extracts priority from %s", (token, expected) => {
    const result = parseTaskInput(`Do thing ${token}`, NOW);

    expect(result.title).toBe("Do thing");
    expect(result.priority).toBe(expected);
  });

  test("ignores an unrecognized priority token, leaving it in the title", () => {
    const result = parseTaskInput("Do thing !urgent", NOW);

    expect(result.title).toBe("Do thing !urgent");
    expect(result.priority).toBeUndefined();
  });

  test("extracts due:today as the current date", () => {
    const result = parseTaskInput("Pay rent due:today", NOW);

    expect(result.title).toBe("Pay rent");
    expect(result.due).toBe("2026-06-10");
  });

  test("extracts due:tomorrow as the next day", () => {
    const result = parseTaskInput("Pay rent due:tomorrow", NOW);

    expect(result.due).toBe("2026-06-11");
  });

  test("extracts due:YYYY-MM-DD as a literal date", () => {
    const result = parseTaskInput("Submit report due:2026-12-25", NOW);

    expect(result.title).toBe("Submit report");
    expect(result.due).toBe("2026-12-25");
  });

  test("extracts due:<weekday> as the next occurrence of that weekday", () => {
    // NOW is Wednesday 2026-06-10; next Friday is 2026-06-12.
    const result = parseTaskInput("Call mom due:friday", NOW);

    expect(result.due).toBe("2026-06-12");
  });

  test("treats due:<today's weekday> as today", () => {
    // NOW is Wednesday 2026-06-10.
    const result = parseTaskInput("Call mom due:wednesday", NOW);

    expect(result.due).toBe("2026-06-10");
  });

  test("supports abbreviated weekday names case-insensitively", () => {
    const result = parseTaskInput("Call mom due:Fri", NOW);

    expect(result.due).toBe("2026-06-12");
  });

  test("ignores an unrecognized due: expression, leaving it in the title", () => {
    const result = parseTaskInput("Call mom due:whenever", NOW);

    expect(result.title).toBe("Call mom due:whenever");
    expect(result.due).toBeUndefined();
  });

  test("passes through an ISO-shaped but invalid calendar date for backend validation", () => {
    const result = parseTaskInput("Submit report due:2026-13-99", NOW);

    expect(result.title).toBe("Submit report");
    expect(result.due).toBe("2026-13-99");
  });

  test("parses a combination of tags, project, priority, and due date", () => {
    const result = parseTaskInput(
      "Finish slides #work +ProjectX !high due:tomorrow for the review",
      NOW,
    );

    expect(result.title).toBe("Finish slides for the review");
    expect(result.tags).toEqual(["work"]);
    expect(result.project).toBe("ProjectX");
    expect(result.priority).toBe("high");
    expect(result.due).toBe("2026-06-11");
  });

  test("collapses extra whitespace left behind after removing tokens", () => {
    const result = parseTaskInput("  Buy   milk   #shopping   due:today  ", NOW);

    expect(result.title).toBe("Buy milk");
  });

  test.each([
    ["high", "high"],
    ["HIGH", "high"],
    ["medium", "medium"],
    ["Medium", "medium"],
    ["low", "low"],
    ["LOW", "low"],
  ])("extracts a bare priority word '%s'", (word, expected) => {
    const result = parseTaskInput(`Do thing ${word}`, NOW);

    expect(result.title).toBe("Do thing");
    expect(result.priority).toBe(expected);
  });

  test("strips a bare priority word even from an ordinary title (accepted tradeoff)", () => {
    // "high"/"medium"/"low" as standalone words always set priority and are
    // removed from the title, per the project constitution's quick-add example.
    const result = parseTaskInput("Buy high quality filament", NOW);

    expect(result.title).toBe("Buy quality filament");
    expect(result.priority).toBe("high");
  });

  test.each([
    ["due today", "2026-06-10"],
    ["due tomorrow", "2026-06-11"],
    ["due friday", "2026-06-12"],
    ["due next friday", "2026-06-12"],
    ["due 2026-12-25", "2026-12-25"],
    ["Due tomorrow", "2026-06-11"],
  ])("extracts a due-date phrase from '%s'", (phrase, expected) => {
    const result = parseTaskInput(`Pay rent ${phrase}`, NOW);

    expect(result.title).toBe("Pay rent");
    expect(result.due).toBe(expected);
  });

  test("leaves a trailing 'due' with no recognizable phrase in the title", () => {
    const result = parseTaskInput("Pay rent due", NOW);

    expect(result.title).toBe("Pay rent due");
    expect(result.due).toBeUndefined();
  });

  test("leaves 'due <unrecognized word>' in the title", () => {
    const result = parseTaskInput("Rent is due whenever", NOW);

    expect(result.title).toBe("Rent is due whenever");
    expect(result.due).toBeUndefined();
  });

  test.each([
    ["sch today", "2026-06-10"],
    ["sch tomorrow", "2026-06-11"],
    ["sch wednesday", "2026-06-10"],
    ["sch next wednesday", "2026-06-10"],
    ["sch:friday", "2026-06-12"],
  ])("extracts a scheduled-date phrase from '%s'", (phrase, expected) => {
    const result = parseTaskInput(`Plan trip ${phrase}`, NOW);

    expect(result.title).toBe("Plan trip");
    expect(result.scheduled).toBe(expected);
  });

  test("leaves a trailing 'sch' with no recognizable phrase in the title", () => {
    const result = parseTaskInput("Plan sch", NOW);

    expect(result.title).toBe("Plan sch");
    expect(result.scheduled).toBeUndefined();
  });

  test("'next' without a preceding due/sch keyword stays in the title", () => {
    const result = parseTaskInput("Plan for next quarter", NOW);

    expect(result.title).toBe("Plan for next quarter");
    expect(result.due).toBeUndefined();
    expect(result.scheduled).toBeUndefined();
  });

  test("parses the full natural-language example from the project constitution", () => {
    // NOW is Wednesday 2026-06-10: "wednesday" resolves to today (the next
    // occurrence including today), and "friday" resolves to 2026-06-12.
    const result = parseTaskInput(
      "Assignment 4 #class sch next wednesday due next friday high",
      NOW,
    );

    expect(result.title).toBe("Assignment 4");
    expect(result.tags).toEqual(["class"]);
    expect(result.priority).toBe("high");
    expect(result.scheduled).toBe("2026-06-10");
    expect(result.due).toBe("2026-06-12");
  });
});
