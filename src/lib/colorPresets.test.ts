import { describe, expect, test } from "vitest";
import { isHexColor, PRESET_COLOR_NAMES, PRESET_COLORS } from "./colorPresets";

describe("isHexColor", () => {
  test("accepts a 6-digit hex color", () => {
    expect(isHexColor("#3b82f6")).toBe(true);
  });

  test("accepts uppercase hex digits", () => {
    expect(isHexColor("#3B82F6")).toBe(true);
  });

  test("rejects a 3-digit hex color", () => {
    expect(isHexColor("#fff")).toBe(false);
  });

  test("rejects a value missing the leading #", () => {
    expect(isHexColor("3b82f6")).toBe(false);
  });

  test("rejects a CSS color keyword", () => {
    expect(isHexColor("blue")).toBe(false);
  });

  test("rejects an oklch color", () => {
    expect(isHexColor("oklch(58% 0.012 60)")).toBe(false);
  });

  test("rejects a hex color with non-hex digits", () => {
    expect(isHexColor("#3b82g6")).toBe(false);
  });
});

describe("PRESET_COLORS", () => {
  test("contains 8 distinct valid hex colors", () => {
    expect(PRESET_COLORS).toHaveLength(8);
    expect(new Set(PRESET_COLORS).size).toBe(8);
    expect(PRESET_COLORS.every(isHexColor)).toBe(true);
  });
});

describe("PRESET_COLOR_NAMES", () => {
  test("has one name per preset color", () => {
    expect(PRESET_COLOR_NAMES).toHaveLength(PRESET_COLORS.length);
    expect(new Set(PRESET_COLOR_NAMES).size).toBe(PRESET_COLOR_NAMES.length);
  });
});
