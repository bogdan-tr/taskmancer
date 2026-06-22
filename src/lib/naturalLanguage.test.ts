import { describe, expect, test } from "vitest";
import { parseTaskInput, type KnownPriority, type KnownStatus } from "./naturalLanguage";

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

  test("a bare +token with no slash is unaffected by path resolution", () => {
    const result = parseTaskInput("Plan trip +Vacation and relax", NOW);

    expect(result.title).toBe("Plan trip and relax");
    expect(result.project).toBe("Vacation");
  });

  test("extracts a single-level project path with no quoting needed", () => {
    const result = parseTaskInput("Fix the bug +Work/ClientA", NOW);

    expect(result.title).toBe("Fix the bug");
    expect(result.project).toBe("Work/ClientA");
  });

  test("extracts a multi-level project path", () => {
    const result = parseTaskInput("Fix the bug +Work/ClientA/Phase1", NOW);

    expect(result.project).toBe("Work/ClientA/Phase1");
  });

  test("extracts a project path with a quoted segment that doesn't span tokens", () => {
    const result = parseTaskInput('Fix the bug +Work/"ClientA"', NOW);

    expect(result.project).toBe("Work/ClientA");
  });

  test("extracts a project path with a quoted segment spanning multiple whitespace tokens", () => {
    const result = parseTaskInput('Fix the bug +Work/"Client A"', NOW);

    expect(result.title).toBe("Fix the bug");
    expect(result.project).toBe("Work/Client A");
  });

  test("a quoted segment can appear first in the path", () => {
    const result = parseTaskInput('Fix the bug +"Client A"/Phase1', NOW);

    expect(result.project).toBe("Client A/Phase1");
  });

  test("text after a project path continues as part of the title", () => {
    const result = parseTaskInput('Fix the bug +Work/"Client A" before friday', NOW);

    expect(result.title).toBe("Fix the bug before friday");
    expect(result.project).toBe("Work/Client A");
  });

  test("falls back to the bare token when a quote is never closed", () => {
    const result = parseTaskInput('Fix the bug +Work/"Client A', NOW);

    expect(result.project).toBe('Work/"Client');
    expect(result.title).toBe("Fix the bug A");
  });

  test("falls back to the bare token for an empty path segment", () => {
    const result = parseTaskInput("Fix the bug +Work//ClientA", NOW);

    expect(result.project).toBe("Work//ClientA");
  });

  test("extracts a single-word subtask parent name with no quoting needed", () => {
    const result = parseTaskInput("Reproduce it sub Refactor", NOW);

    expect(result.title).toBe("Reproduce it");
    expect(result.subtaskParentName).toBe("Refactor");
  });

  test("extracts a quoted multi-word subtask parent name spanning multiple tokens", () => {
    const result = parseTaskInput('Reproduce it sub "Fix the bug"', NOW);

    expect(result.title).toBe("Reproduce it");
    expect(result.subtaskParentName).toBe("Fix the bug");
  });

  test("text after a quoted subtask parent name continues as part of the title", () => {
    const result = parseTaskInput('sub "Fix the bug" Reproduce it', NOW);

    expect(result.title).toBe("Reproduce it");
    expect(result.subtaskParentName).toBe("Fix the bug");
  });

  test("leaves an unquoted multi-word subtask parent name only as the first word", () => {
    const result = parseTaskInput("sub Fix the bug", NOW);

    expect(result.subtaskParentName).toBe("Fix");
    expect(result.title).toBe("the bug");
  });

  test("leaves sub untouched in the title when its quote is never closed", () => {
    const result = parseTaskInput('Reproduce it sub "Fix the bug', NOW);

    expect(result.subtaskParentName).toBeUndefined();
    expect(result.title).toBe('Reproduce it sub "Fix the bug');
  });

  test("leaves sub untouched in the title for an empty quoted parent name", () => {
    const result = parseTaskInput('Reproduce it sub ""', NOW);

    expect(result.subtaskParentName).toBeUndefined();
    expect(result.title).toBe('Reproduce it sub ""');
  });

  test("treats sub as a literal word when disableSubtaskKeyword is set", () => {
    const result = parseTaskInput('Reproduce it sub "Fix the bug"', NOW, undefined, undefined, {
      disableSubtaskKeyword: true,
    });

    expect(result.subtaskParentName).toBeUndefined();
    expect(result.title).toBe('Reproduce it sub "Fix the bug"');
  });

  test("a priority word and a duration after sub <name> still parse normally, not swallowed into the parent name", () => {
    const result = parseTaskInput("child task sub Cleaning high est 30m", NOW);

    expect(result.title).toBe("child task");
    expect(result.subtaskParentName).toBe("Cleaning");
    expect(result.priority).toBe("high");
    expect(result.estimatedMinutes).toBe(30);
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

  test("extracts due:na as the never-due sentinel", () => {
    const result = parseTaskInput("Pay rent due:na", NOW);

    expect(result.title).toBe("Pay rent");
    expect(result.due).toBe("none");
  });

  test("extracts a 'due na' phrase as the never-due sentinel", () => {
    const result = parseTaskInput("Pay rent due na", NOW);

    expect(result.title).toBe("Pay rent");
    expect(result.due).toBe("none");
  });

  test("'due na'/'due:na' are case-insensitive", () => {
    const colon = parseTaskInput("Pay rent due:NA", NOW);
    const phrase = parseTaskInput("Pay rent due NA", NOW);

    expect(colon.due).toBe("none");
    expect(phrase.due).toBe("none");
  });

  test("'due na' also sets dueRule to the Never sentinel", () => {
    const result = parseTaskInput("Pay rent due na", NOW);

    expect(result.dueRule).toEqual({ kind: "Never" });
  });

  test("'due:na' also sets dueRule to the Never sentinel", () => {
    const result = parseTaskInput("Pay rent due:na", NOW);

    expect(result.dueRule).toEqual({ kind: "Never" });
  });

  test("extracts 'due in <n> days' as an AfterScheduled rule, leaving due unresolved", () => {
    const result = parseTaskInput("Water plants due in 5 days", NOW);

    expect(result.title).toBe("Water plants");
    expect(result.due).toBeUndefined();
    expect(result.dueRule).toEqual({ kind: "AfterScheduled", days: 5 });
  });

  test("'due in <n> day' (singular) is recognized the same as 'days'", () => {
    const result = parseTaskInput("Water plants due in 1 day", NOW);

    expect(result.dueRule).toEqual({ kind: "AfterScheduled", days: 1 });
  });

  test("'due in 0 days' is a valid same-day offset", () => {
    const result = parseTaskInput("Water plants due in 0 days", NOW);

    expect(result.dueRule).toEqual({ kind: "AfterScheduled", days: 0 });
  });

  test("'due in' without a recognized day count falls through, leaving it in the title", () => {
    const result = parseTaskInput("Water plants due in a while", NOW);

    expect(result.title).toBe("Water plants due in a while");
    expect(result.due).toBeUndefined();
    expect(result.dueRule).toBeUndefined();
  });

  test("extracts a plural weekday ('due mondays') as a Weekday rule with interval 1 when the task is recurring", () => {
    const result = parseTaskInput("Gym every saturday due mondays", NOW);

    expect(result.title).toBe("Gym");
    expect(result.due).toBeUndefined();
    expect(result.dueRule).toEqual({ kind: "Weekday", weekday: 1, interval_weeks: 1 });
  });

  test("extracts 'due every <weekday>' (singular) as a Weekday rule with interval 1 when the task is recurring", () => {
    const result = parseTaskInput("Gym every saturday due every monday", NOW);

    expect(result.dueRule).toEqual({ kind: "Weekday", weekday: 1, interval_weeks: 1 });
  });

  test("a plain singular weekday with no 'every' or plural keeps its existing absolute-date meaning", () => {
    // NOW is Wednesday 2026-06-10; next Monday is 2026-06-15.
    const result = parseTaskInput("Gym due monday", NOW);

    expect(result.due).toBe("2026-06-15");
    expect(result.dueRule).toBeUndefined();
  });

  test("extracts 'due other mondays' (plural) as a Weekday rule with interval 2 when the task is recurring", () => {
    const result = parseTaskInput("Gym every saturday due other mondays", NOW);

    expect(result.due).toBeUndefined();
    expect(result.dueRule).toEqual({ kind: "Weekday", weekday: 1, interval_weeks: 2 });
  });

  test("extracts 'due every other <weekday>' (singular) as a Weekday rule with interval 2 when the task is recurring", () => {
    const result = parseTaskInput("Gym every saturday due every other monday", NOW);

    expect(result.dueRule).toEqual({ kind: "Weekday", weekday: 1, interval_weeks: 2 });
  });

  test("the weekday-rule phrase is case-insensitive", () => {
    const result = parseTaskInput("Gym every saturday due MONDAYS", NOW);

    expect(result.dueRule).toEqual({ kind: "Weekday", weekday: 1, interval_weeks: 1 });
  });

  test("recognizes a plural weekday immediately followed by a comma-separated list boundary", () => {
    const result = parseTaskInput("Gym every saturday due mondays, for real", NOW);

    expect(result.dueRule).toEqual({ kind: "Weekday", weekday: 1, interval_weeks: 1 });
    expect(result.title).toBe("Gym for real");
  });

  test("'due every' with no recognized weekday falls through, leaving it in the title", () => {
    const result = parseTaskInput("Gym every saturday due every banana", NOW);

    expect(result.title).toBe("Gym due every banana");
    expect(result.due).toBeUndefined();
    expect(result.dueRule).toBeUndefined();
  });

  test("recurring due rules work together with an 'every ...' recurrence token in the same title", () => {
    const result = parseTaskInput("Gym every saturday due in 2 days", NOW);

    expect(result.title).toBe("Gym");
    expect(result.recurrence?.frequency).toEqual({ kind: "Weekly", weekdays: [6], interval_weeks: 1 });
    expect(result.dueRule).toEqual({ kind: "AfterScheduled", days: 2 });
  });

  test("'due in <n> days' (the generic offset form) works even without an 'every ...' recurrence token", () => {
    const result = parseTaskInput("Water plants due in 2 days", NOW);

    expect(result.recurrence).toBeUndefined();
    expect(result.dueRule).toEqual({ kind: "AfterScheduled", days: 2 });
  });

  test("the weekday-rule due phrase is NOT recognized on a non-recurring task — 'due mondays' stays literal", () => {
    const result = parseTaskInput("Gym due mondays", NOW);

    expect(result.title).toBe("Gym due mondays");
    expect(result.due).toBeUndefined();
    expect(result.dueRule).toBeUndefined();
  });

  test("'due every <weekday>' with nothing else recurring: the due phrase itself isn't matched, but the standalone 'every <weekday>' that's left over is still a valid recurrence phrase in its own right (pre-existing grammar, unrelated to the due-rule gate)", () => {
    const result = parseTaskInput("Gym due every monday", NOW);

    expect(result.title).toBe("Gym due");
    expect(result.dueRule).toBeUndefined();
    expect(result.recurrence?.frequency).toEqual({ kind: "Weekly", weekdays: [1], interval_weeks: 1 });
  });

  test("the weekday-rule due phrase is NOT recognized on a non-recurring task — 'due other mondays' stays literal", () => {
    const result = parseTaskInput("Gym due other mondays", NOW);

    expect(result.title).toBe("Gym due other mondays");
    expect(result.dueRule).toBeUndefined();
  });

  test("a separate recurrence token appearing AFTER the due phrase in the title still counts as recurring", () => {
    // "due mondays" appears before "every saturday" — the weekday-rule
    // phrase still needs to be recognized correctly via the pre-scan.
    const result = parseTaskInput("Gym due mondays every saturday", NOW);

    expect(result.dueRule).toEqual({ kind: "Weekday", weekday: 1, interval_weeks: 1 });
    expect(result.recurrence?.frequency).toEqual({ kind: "Weekly", weekdays: [6], interval_weeks: 1 });
  });

  // The recurrence pre-scan must actually resolve a real recurrence phrase,
  // not just look for the bare word "every" — ordinary English uses of
  // "every" elsewhere in a title must never falsely gate the weekday due
  // rule open on what's actually a non-recurring task (a false positive
  // there would silently drop the due date entirely, since `createTask`
  // never reads `dueRule` — see api.ts).
  test.each([
    "Review every single PR due fridays",
    "Email the team every week or so due mondays",
    "Check on it every now and then due fridays",
    "Tell them every detail due mondays",
    "Call every time due mondays",
  ])("an ordinary, non-recurrence use of 'every' does not falsely enable the weekday due rule: %s", (title) => {
    const result = parseTaskInput(title, NOW);

    expect(result.dueRule).toBeUndefined();
    expect(result.recurrence).toBeUndefined();
  });

  test("'every day' is a real recurrence phrase, so a weekday due rule alongside it is correctly recognized", () => {
    const result = parseTaskInput("Call every day due mondays", NOW);

    expect(result.recurrence?.frequency).toEqual({ kind: "EveryNDays", interval: 1 });
    expect(result.dueRule).toEqual({ kind: "Weekday", weekday: 1, interval_weeks: 1 });
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

describe("parseTaskInput estimated time", () => {
  test("extracts hours and minutes with the 'est' keyword", () => {
    const result = parseTaskInput("Read book est 1h 30m", NOW);

    expect(result.title).toBe("Read book");
    expect(result.estimatedMinutes).toBe(90);
  });

  test("the 'est' keyword is optional", () => {
    const result = parseTaskInput("Read book 1h 30m", NOW);

    expect(result.title).toBe("Read book");
    expect(result.estimatedMinutes).toBe(90);
  });

  test("'m' is optional once an 'h' anchors the phrase", () => {
    const result = parseTaskInput("Read book 1h 30", NOW);

    expect(result.title).toBe("Read book");
    expect(result.estimatedMinutes).toBe(90);
  });

  test("'est' plus a bare hour-anchored number without 'm' also works", () => {
    const result = parseTaskInput("Read book est 1h 30", NOW);

    expect(result.title).toBe("Read book");
    expect(result.estimatedMinutes).toBe(90);
  });

  test("hours alone, with no trailing minutes", () => {
    const result = parseTaskInput("Read book est 2h", NOW);

    expect(result.title).toBe("Read book");
    expect(result.estimatedMinutes).toBe(120);
  });

  test("minutes alone, with no hours", () => {
    const result = parseTaskInput("Read book est 45m", NOW);

    expect(result.title).toBe("Read book");
    expect(result.estimatedMinutes).toBe(45);
  });

  test("minutes alone without 'est'", () => {
    const result = parseTaskInput("Read book 45m", NOW);

    expect(result.title).toBe("Read book");
    expect(result.estimatedMinutes).toBe(45);
  });

  test("a bare unitless number is never treated as a duration, with or without 'est'", () => {
    const withoutEst = parseTaskInput("Buy 30 stamps", NOW);
    expect(withoutEst.title).toBe("Buy 30 stamps");
    expect(withoutEst.estimatedMinutes).toBeUndefined();

    const withEst = parseTaskInput("Read book est 30", NOW);
    expect(withEst.title).toBe("Read book est 30");
    expect(withEst.estimatedMinutes).toBeUndefined();
  });

  test("'est' with no recognizable duration after it is left in the title", () => {
    const result = parseTaskInput("Read book est", NOW);

    expect(result.title).toBe("Read book est");
    expect(result.estimatedMinutes).toBeUndefined();
  });

  test("the unit suffix is case-insensitive", () => {
    const result = parseTaskInput("Read book est 1H 30M", NOW);

    expect(result.title).toBe("Read book");
    expect(result.estimatedMinutes).toBe(90);
  });

  test("last 'est' token wins when repeated", () => {
    const result = parseTaskInput("Read book est 1h est 30m", NOW);

    expect(result.title).toBe("Read book");
    expect(result.estimatedMinutes).toBe(30);
  });
});

describe("parseTaskInput recurrence", () => {
  test("'every day' sets a daily recurrence", () => {
    const result = parseTaskInput("Water plants every day", NOW);

    expect(result.title).toBe("Water plants");
    expect(result.recurrence).toEqual({ frequency: { kind: "EveryNDays", interval: 1 } });
  });

  test("'every other day' sets an every-2-days recurrence", () => {
    const result = parseTaskInput("Water plants every other day", NOW);

    expect(result.title).toBe("Water plants");
    expect(result.recurrence).toEqual({ frequency: { kind: "EveryNDays", interval: 2 } });
  });

  test("'every <n> days' sets an every-n-days recurrence", () => {
    const result = parseTaskInput("Water plants every 5 days", NOW);

    expect(result.title).toBe("Water plants");
    expect(result.recurrence).toEqual({ frequency: { kind: "EveryNDays", interval: 5 } });
  });

  test("'every weekend' sets a Saturday+Sunday weekly recurrence", () => {
    const result = parseTaskInput("Clean garage every weekend", NOW);

    expect(result.title).toBe("Clean garage");
    expect(result.recurrence).toEqual({
      frequency: { kind: "Weekly", weekdays: [0, 6], interval_weeks: 1 },
    });
  });

  test("'every weekday' sets a Monday-Friday weekly recurrence", () => {
    const result = parseTaskInput("Check email every weekday", NOW);

    expect(result.title).toBe("Check email");
    expect(result.recurrence).toEqual({
      frequency: { kind: "Weekly", weekdays: [1, 2, 3, 4, 5], interval_weeks: 1 },
    });
  });

  test("'every <ordinal>' sets a monthly-by-day recurrence", () => {
    const result = parseTaskInput("Pay rent every 4th", NOW);

    expect(result.title).toBe("Pay rent");
    expect(result.recurrence).toEqual({ frequency: { kind: "MonthlyByDay", day: 4 } });
  });

  test("'every <ordinal>' recognizes the 31st", () => {
    const result = parseTaskInput("Pay rent every 31st", NOW);

    expect(result.recurrence).toEqual({ frequency: { kind: "MonthlyByDay", day: 31 } });
  });

  test("'every <weekday>' sets a single-weekday weekly recurrence", () => {
    const result = parseTaskInput("Take out trash every monday", NOW);

    expect(result.title).toBe("Take out trash");
    expect(result.recurrence).toEqual({
      frequency: { kind: "Weekly", weekdays: [1], interval_weeks: 1 },
    });
  });

  test("'every other <weekday>' sets a biweekly single-weekday recurrence", () => {
    const result = parseTaskInput("Water plants every other saturday", NOW);

    expect(result.title).toBe("Water plants");
    expect(result.recurrence).toEqual({
      frequency: { kind: "Weekly", weekdays: [6], interval_weeks: 2 },
    });
  });

  test("multiple comma-separated weekdays are all collected", () => {
    const result = parseTaskInput("Gym every monday, wednesday, friday", NOW);

    expect(result.title).toBe("Gym");
    expect(result.recurrence).toEqual({
      frequency: { kind: "Weekly", weekdays: [1, 3, 5], interval_weeks: 1 },
    });
  });

  test("multiple weekdays work without commas too", () => {
    const result = parseTaskInput("Gym every monday wednesday friday", NOW);

    expect(result.recurrence).toEqual({
      frequency: { kind: "Weekly", weekdays: [1, 3, 5], interval_weeks: 1 },
    });
  });

  test("a duplicate weekday in the list is not added twice", () => {
    const result = parseTaskInput("Gym every monday, monday", NOW);

    expect(result.recurrence).toEqual({
      frequency: { kind: "Weekly", weekdays: [1], interval_weeks: 1 },
    });
  });

  test("'until <date-phrase>' sets an end date on any recurrence form", () => {
    const result = parseTaskInput("Gym every monday until july 31", NOW);

    expect(result.title).toBe("Gym");
    expect(result.recurrence).toEqual({
      frequency: { kind: "Weekly", weekdays: [1], interval_weeks: 1 },
      endDate: "2026-07-31",
    });
  });

  test("'every other <weekday>' followed directly by 'until' still parses despite the trailing comma it leaves behind", () => {
    const result = parseTaskInput("Gym every other monday, until july 31", NOW);

    expect(result.title).toBe("Gym");
    expect(result.recurrence).toEqual({
      frequency: { kind: "Weekly", weekdays: [1], interval_weeks: 2 },
      endDate: "2026-07-31",
    });
  });

  test("'until' with no recognizable date phrase after it is left in the title", () => {
    const result = parseTaskInput("Gym every monday until whenever", NOW);

    expect(result.title).toBe("Gym until whenever");
    expect(result.recurrence).toEqual({
      frequency: { kind: "Weekly", weekdays: [1], interval_weeks: 1 },
    });
  });

  test("'every' with no recognizable phrase after it is left in the title", () => {
    const result = parseTaskInput("Water plants every now and then", NOW);

    expect(result.title).toBe("Water plants every now and then");
    expect(result.recurrence).toBeUndefined();
  });

  test("a bare 'every' with nothing after it is left in the title", () => {
    const result = parseTaskInput("Water plants every", NOW);

    expect(result.title).toBe("Water plants every");
    expect(result.recurrence).toBeUndefined();
  });

  test("'every 0 days' is rejected (a non-positive interval doesn't make sense)", () => {
    const result = parseTaskInput("Water plants every 0 days", NOW);

    expect(result.title).toBe("Water plants every 0 days");
    expect(result.recurrence).toBeUndefined();
  });

  test("last 'every' token wins when repeated", () => {
    const result = parseTaskInput("Gym every day every monday", NOW);

    expect(result.title).toBe("Gym");
    expect(result.recurrence).toEqual({
      frequency: { kind: "Weekly", weekdays: [1], interval_weeks: 1 },
    });
  });

  test("recurrence keywords are case-insensitive", () => {
    const result = parseTaskInput("Gym EVERY MONDAY", NOW);

    expect(result.title).toBe("Gym");
    expect(result.recurrence).toEqual({
      frequency: { kind: "Weekly", weekdays: [1], interval_weeks: 1 },
    });
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

describe("parseTaskInput with knownStatuses", () => {
  const KNOWN_STATUSES: KnownStatus[] = [
    { id: "backlog", label: "Backlog" },
    { id: "do", label: "Do" },
    { id: "in-progress", label: "In Progress" },
  ];

  test("matches @<id> against a known status, case-insensitively", () => {
    const result = parseTaskInput("Fix bug @do", NOW, undefined, KNOWN_STATUSES);

    expect(result.title).toBe("Fix bug");
    expect(result.status).toBe("do");
  });

  test("matches @<id> uppercased against a known status", () => {
    const result = parseTaskInput("Fix bug @DO", NOW, undefined, KNOWN_STATUSES);

    expect(result.status).toBe("do");
  });

  test("matches @<label> against a known status's single-word label, case-insensitively", () => {
    const result = parseTaskInput("Fix bug @Backlog", NOW, undefined, KNOWN_STATUSES);

    expect(result.title).toBe("Fix bug");
    expect(result.status).toBe("backlog");
  });

  test("matches a hyphenated status id", () => {
    const result = parseTaskInput("Fix bug @in-progress", NOW, undefined, KNOWN_STATUSES);

    expect(result.status).toBe("in-progress");
  });

  test("last @status token wins when multiple are present", () => {
    const result = parseTaskInput("Fix bug @do @backlog", NOW, undefined, KNOWN_STATUSES);

    expect(result.status).toBe("backlog");
  });

  test("leaves an unrecognized @word in the title when knownStatuses has no match", () => {
    const result = parseTaskInput("Email @acme about renewal", NOW, undefined, KNOWN_STATUSES);

    expect(result.title).toBe("Email @acme about renewal");
    expect(result.status).toBeUndefined();
  });

  test("leaves @word in the title when knownStatuses is omitted entirely", () => {
    const result = parseTaskInput("Fix bug @do", NOW);

    expect(result.title).toBe("Fix bug @do");
    expect(result.status).toBeUndefined();
  });

  test("an empty knownStatuses array behaves the same as omitting it", () => {
    const result = parseTaskInput("Fix bug @do", NOW, undefined, []);

    expect(result.title).toBe("Fix bug @do");
    expect(result.status).toBeUndefined();
  });

  test("a bare '@' with nothing after it is left in the title", () => {
    const result = parseTaskInput("Fix bug @", NOW, undefined, KNOWN_STATUSES);

    expect(result.title).toBe("Fix bug @");
    expect(result.status).toBeUndefined();
  });
});
