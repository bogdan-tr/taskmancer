import { describe, expect, test } from "vitest";
import { parseTaskInput } from "./naturalLanguage";

describe("parseTaskInput", () => {
  test("a priority word and a duration after sub <name> still parse normally", () => {
    const result = parseTaskInput("child task sub Cleaning high est 30m", new Date("2026-06-22"));

    expect(result.subtaskParentName).toBe("Cleaning");
    expect(result.priority).toBe("high");
    expect(result.estimatedMinutes).toBe(30);
  });
});
