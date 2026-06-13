import { describe, expect, test } from "vitest";
import { DEFAULT_THEME, isTheme, THEME_LABELS, THEMES } from "./theme";

describe("THEMES", () => {
  test("lists every theme exactly once", () => {
    expect(THEMES).toEqual(["light", "dark", "dark-blue"]);
  });

  test("has a label for every theme", () => {
    for (const theme of THEMES) {
      expect(THEME_LABELS[theme]).toBeTruthy();
    }
  });
});

describe("DEFAULT_THEME", () => {
  test("is a valid theme", () => {
    expect(THEMES).toContain(DEFAULT_THEME);
  });
});

describe("isTheme", () => {
  test("accepts every known theme", () => {
    for (const theme of THEMES) {
      expect(isTheme(theme)).toBe(true);
    }
  });

  test("rejects an unknown string", () => {
    expect(isTheme("solarized")).toBe(false);
  });

  test("rejects null", () => {
    expect(isTheme(null)).toBe(false);
  });

  test("rejects non-string values", () => {
    expect(isTheme(42)).toBe(false);
  });
});
