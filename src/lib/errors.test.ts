import { describe, expect, test } from "vitest";
import { getErrorMessage } from "./errors";

describe("getErrorMessage", () => {
  test("returns a plain string thrown/rejected as-is, mirroring Tauri's Result::Err(String) rejection", () => {
    expect(getErrorMessage("a project named 'Work' already exists", "fallback")).toBe(
      "a project named 'Work' already exists",
    );
  });

  test("returns an Error instance's message", () => {
    expect(getErrorMessage(new Error("boom"), "fallback")).toBe("boom");
  });

  test("returns the fallback for an empty string", () => {
    expect(getErrorMessage("", "fallback")).toBe("fallback");
  });

  test("returns the fallback for an Error with an empty message", () => {
    expect(getErrorMessage(new Error(""), "fallback")).toBe("fallback");
  });

  test("returns the fallback for a non-Error, non-string value", () => {
    expect(getErrorMessage({ unexpected: true }, "fallback")).toBe("fallback");
    expect(getErrorMessage(undefined, "fallback")).toBe("fallback");
    expect(getErrorMessage(null, "fallback")).toBe("fallback");
  });
});
