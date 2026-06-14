import { describe, expect, test } from "vitest";
import { parseTaskInput, type KnownPriority } from "./naturalLanguage";

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
    // "next friday" skips the upcoming Friday (06-12) and resolves to the
    // following one, one week later than bare "friday".
    ["due next friday", "2026-06-19"],
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
    // "next wednesday" skips today (a Wednesday) and resolves to one week
    // later than bare "wednesday" would.
    ["sch next wednesday", "2026-06-17"],
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
    // NOW is Wednesday 2026-06-10. "next wednesday" skips today and resolves
    // to 2026-06-17; "next friday" skips 2026-06-12 and resolves to
    // 2026-06-19.
    const result = parseTaskInput(
      "Assignment 4 #class sch next wednesday due next friday high",
      NOW,
    );

    expect(result.title).toBe("Assignment 4");
    expect(result.tags).toEqual(["class"]);
    expect(result.priority).toBe("high");
    expect(result.scheduled).toBe("2026-06-17");
    expect(result.due).toBe("2026-06-19");
  });

  test.each([
    ["monday", "2026-06-15"],
    ["next monday", "2026-06-22"],
  ])("'due %s' resolves to %s (next skips the upcoming occurrence)", (phrase, expected) => {
    const result = parseTaskInput(`Pay rent due ${phrase}`, NOW);

    expect(result.title).toBe("Pay rent");
    expect(result.due).toBe(expected);
  });

  test("'next' immediately before a non-weekday is not recognized as a phrase", () => {
    const result = parseTaskInput("Pay rent due next tomorrow", NOW);

    expect(result.title).toBe("Pay rent due next tomorrow");
    expect(result.due).toBeUndefined();
  });

  test.each([
    // NOW is 2026-06-10.
    ["due june 11", "2026-06-11"], // future this year: used as-is
    ["due 11 june", "2026-06-11"], // day-before-month order
    ["due june 11th", "2026-06-11"], // ordinal suffix
    ["due jun 11", "2026-06-11"], // abbreviated month name
    ["due may 31", "2027-05-31"], // already passed this year: rolls to next year
    ["due june 9", "2027-06-09"], // already passed this year: rolls to next year
    ["due june 10", "2026-06-10"], // same as today: not rolled over
    ["due june 11 2027", "2027-06-11"], // explicit future year
    ["due june 11 2020", "2020-06-11"], // explicit past year: used as-is, no rollover
  ])("extracts an absolute date phrase from '%s'", (phrase, expected) => {
    const result = parseTaskInput(`Pay rent ${phrase}`, NOW);

    expect(result.title).toBe("Pay rent");
    expect(result.due).toBe(expected);
  });

  test("leaves an invalid absolute calendar date in the title", () => {
    const result = parseTaskInput("Pay rent due february 30", NOW);

    expect(result.title).toBe("Pay rent due february 30");
    expect(result.due).toBeUndefined();
  });
});

describe("parseTaskInput with knownPriorities", () => {
  const KNOWN_PRIORITIES: KnownPriority[] = [
    { id: "urgent", label: "Urgent" },
    { id: "someday", label: "Someday" },
  ];

  test("matches a bare word against a custom priority level's label, case-insensitively", () => {
    const result = parseTaskInput("Fix bug URGENT", NOW, KNOWN_PRIORITIES);

    expect(result.title).toBe("Fix bug");
    expect(result.priority).toBe("urgent");
  });

  test("matches a bare word against a custom priority level's id", () => {
    const result = parseTaskInput("Plan trip someday", NOW, KNOWN_PRIORITIES);

    expect(result.title).toBe("Plan trip");
    expect(result.priority).toBe("someday");
  });

  test("matches !<label> against a custom priority level", () => {
    const result = parseTaskInput("Fix bug !Urgent", NOW, KNOWN_PRIORITIES);

    expect(result.title).toBe("Fix bug");
    expect(result.priority).toBe("urgent");
  });

  test("matches !<id> against a custom priority level, case-insensitively", () => {
    const result = parseTaskInput("Fix bug !URGENT", NOW, KNOWN_PRIORITIES);

    expect(result.title).toBe("Fix bug");
    expect(result.priority).toBe("urgent");
  });

  test("falls back to the built-in !-prefixed tokens when knownPriorities has no match", () => {
    const result = parseTaskInput("Do thing !high", NOW, KNOWN_PRIORITIES);

    expect(result.title).toBe("Do thing");
    expect(result.priority).toBe("high");
  });

  test("falls back to the built-in bare words when knownPriorities has no match", () => {
    const result = parseTaskInput("Do thing medium", NOW, KNOWN_PRIORITIES);

    expect(result.title).toBe("Do thing");
    expect(result.priority).toBe("medium");
  });

  test("a knownPriorities match takes precedence over a built-in token sharing the same word", () => {
    // A custom level whose label is "High" but whose id differs from the
    // built-in "high" token - the custom id wins.
    const result = parseTaskInput("Do thing !high", NOW, [{ id: "p1", label: "High" }]);

    expect(result.priority).toBe("p1");
  });

  test("an empty knownPriorities array behaves the same as omitting the parameter", () => {
    const result = parseTaskInput("Do thing !high", NOW, []);

    expect(result.priority).toBe("high");
  });

  test("leaves an unrecognized bare word in the title when it matches neither knownPriorities nor built-ins", () => {
    const result = parseTaskInput("Buy whenever filament", NOW, KNOWN_PRIORITIES);

    expect(result.title).toBe("Buy whenever filament");
    expect(result.priority).toBeUndefined();
  });
});
