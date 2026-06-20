import { describe, expect, test } from "vitest";
import {
  cssColorToHex,
  hexToOklch,
  isHexColor,
  isLightColor,
  legibleInkColor,
  neonCardColor,
  NEON_CARD_CHROMA_BOOST,
  NEON_CARD_LIGHTNESS,
  PRESET_COLOR_NAMES,
  PRESET_COLORS,
  relativeLuminance,
  WEEK_BAR_CHROMA_BOOST,
  WEEK_BAR_LIGHTNESS,
} from "./colorPresets";

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

describe("cssColorToHex", () => {
  test("converts an oklch color (percentage lightness) to its hex equivalent", () => {
    expect(cssColorToHex("oklch(58% 0.13 70)")).toBe("#aa6a00");
  });

  test("converts an oklch color with a high chroma/hue to its hex equivalent", () => {
    expect(cssColorToHex("oklch(54% 0.2 350)")).toBe("#bc267f");
  });

  test("converts an oklch color with a fractional (non-percentage) lightness", () => {
    expect(cssColorToHex("oklch(0.58 0.13 70)")).toBe("#aa6a00");
  });

  test("returns an already-hex color unchanged, lowercased", () => {
    expect(cssColorToHex("#3B82F6")).toBe("#3b82f6");
  });

  test("returns unrecognized color formats unchanged", () => {
    expect(cssColorToHex("rebeccapurple")).toBe("rebeccapurple");
  });
});

describe("relativeLuminance", () => {
  test("white has luminance 1", () => {
    expect(relativeLuminance("#ffffff")).toBeCloseTo(1, 5);
  });

  test("black has luminance 0", () => {
    expect(relativeLuminance("#000000")).toBeCloseTo(0, 5);
  });

  test("treats non-hex input as mid-gray (0.5)", () => {
    expect(relativeLuminance("oklch(58% 0.13 70)")).toBe(0.5);
  });
});

describe("isLightColor", () => {
  test("white is light", () => {
    expect(isLightColor("#ffffff")).toBe(true);
  });

  test("black is not light", () => {
    expect(isLightColor("#000000")).toBe(false);
  });

  test("a light pastel color is light", () => {
    expect(isLightColor("#fef3c7")).toBe(true);
  });

  test("every PRESET_COLORS swatch is mid-saturation enough to need light text", () => {
    // All 8 presets sit at medium lightness (relative luminance ~0.17-0.44),
    // where white text reads better than black — confirms the 0.45 threshold
    // doesn't accidentally flip to dark text for the app's own preset palette.
    expect(PRESET_COLORS.every((hex) => !isLightColor(hex))).toBe(true);
  });

  test("non-hex input falls back to mid-gray (luminance 0.5), which is above the light threshold", () => {
    expect(isLightColor("oklch(58% 0.13 70)")).toBe(true);
  });
});

describe("hexToOklch", () => {
  test("round-trips through cssColorToHex for a known oklch color", () => {
    const original = { l: 0.54, c: 0.2, h: 350 };
    const hex = cssColorToHex(`oklch(${original.l * 100}% ${original.c} ${original.h})`);
    const result = hexToOklch(hex);
    expect(result.l).toBeCloseTo(original.l, 1);
    expect(result.c).toBeCloseTo(original.c, 1);
    expect(result.h).toBeCloseTo(original.h, 0);
  });

  test("white has lightness 1 and zero chroma", () => {
    const result = hexToOklch("#ffffff");
    expect(result.l).toBeCloseTo(1, 2);
    expect(result.c).toBeCloseTo(0, 2);
  });

  test("black has lightness 0 and zero chroma", () => {
    const result = hexToOklch("#000000");
    expect(result.l).toBeCloseTo(0, 2);
    expect(result.c).toBeCloseTo(0, 2);
  });

  test("returns all-zero for non-hex input", () => {
    expect(hexToOklch("oklch(58% 0.13 70)")).toEqual({ l: 0, c: 0, h: 0 });
  });
});

describe("neonCardColor", () => {
  test("preserves the original hue at a fixed target lightness", () => {
    const { h: originalHue } = hexToOklch("#3b82f6");
    const result = neonCardColor("#3b82f6", 0.85);
    const match = /^oklch\(85\.0% ([\d.]+) ([\d.]+)\)$/.exec(result);
    expect(match).not.toBeNull();
    expect(Number(match?.[2])).toBeCloseTo(originalHue, 0);
  });

  test("chromaBoost scales the original chroma", () => {
    const base = neonCardColor("#3b82f6", 0.85, 1);
    const boosted = neonCardColor("#3b82f6", 0.85, 1.5);
    const baseChroma = Number(/oklch\([\d.]+% ([\d.]+) /.exec(base)?.[1]);
    const boostedChroma = Number(/oklch\([\d.]+% ([\d.]+) /.exec(boosted)?.[1]);
    expect(boostedChroma).toBeCloseTo(baseChroma * 1.5, 2);
  });

  test("clamps chroma to a believable ceiling instead of producing an absurd value", () => {
    const result = neonCardColor("#3b82f6", 0.85, 100);
    const chroma = Number(/oklch\([\d.]+% ([\d.]+) /.exec(result)?.[1]);
    expect(chroma).toBeLessThanOrEqual(0.4);
  });
});

describe("legibleInkColor", () => {
  test("returns light ink for the default project color at the default Kanban card lightness", () => {
    // Regression test: an earlier version of this function picked purely
    // off the OKLCH lightness channel (>= 0.45 => dark ink) and got this
    // exact, very common case wrong — vivid blue at 50% lightness has a
    // WCAG luminance of ~0.11 (far darker than 50% suggests), so light ink
    // has much better contrast (~5.8:1 vs ~2.8:1), not dark.
    const background = neonCardColor("#3b82f6", NEON_CARD_LIGHTNESS, NEON_CARD_CHROMA_BOOST);
    const { l } = hexToOklch(cssColorToHex(legibleInkColor(background)));
    expect(l).toBeGreaterThan(0.5);
  });

  test("returns light ink for the default project color at the default week-bar lightness", () => {
    const background = neonCardColor("#3b82f6", WEEK_BAR_LIGHTNESS, WEEK_BAR_CHROMA_BOOST);
    const { l } = hexToOklch(cssColorToHex(legibleInkColor(background)));
    expect(l).toBeGreaterThan(0.5);
  });

  test("returns dark ink for a bright, low-chroma background", () => {
    const background = "oklch(95% 0.02 90)";
    const { l } = hexToOklch(cssColorToHex(legibleInkColor(background)));
    expect(l).toBeLessThan(0.5);
  });

  test("returns light ink for a near-black background", () => {
    const background = "oklch(5% 0.02 250)";
    const { l } = hexToOklch(cssColorToHex(legibleInkColor(background)));
    expect(l).toBeGreaterThan(0.5);
  });

  test("accepts a hex background directly, not just oklch()", () => {
    const { l } = hexToOklch(cssColorToHex(legibleInkColor("#ffffff")));
    expect(l).toBeLessThan(0.5);
  });

  test("defaults to auto contrast when no mode is given", () => {
    const background = neonCardColor("#3b82f6", NEON_CARD_LIGHTNESS, NEON_CARD_CHROMA_BOOST);
    expect(legibleInkColor(background)).toEqual(legibleInkColor(background, "auto"));
  });

  test("mode 'white' forces light ink even against a background where dark ink would win on contrast", () => {
    const background = "oklch(95% 0.02 90)";
    const { l } = hexToOklch(cssColorToHex(legibleInkColor(background, "white")));
    expect(l).toBeGreaterThan(0.5);
  });

  test("mode 'black' forces dark ink even against a background where light ink would win on contrast", () => {
    const background = "oklch(5% 0.02 250)";
    const { l } = hexToOklch(cssColorToHex(legibleInkColor(background, "black")));
    expect(l).toBeLessThan(0.5);
  });
});
