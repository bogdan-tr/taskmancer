/**
 * Preset color swatches offered by `ColorPicker`, shared across the new
 * project modal and the project/priority/status color pickers. Avoids
 * `<input type="color">`, which opens a native GTK color picker under
 * webkit2gtk that can hang the window (see the Phase 3b date-input bug).
 */
export const PRESET_COLORS: readonly string[] = [
  "#3b82f6", // blue
  "#22c55e", // green
  "#ec4899", // pink
  "#f59e0b", // amber
  "#8b5cf6", // violet
  "#ef4444", // red
  "#14b8a6", // teal
  "#64748b", // slate
];

/** Human-readable names for `PRESET_COLORS`, in the same order, used for accessible labels. */
export const PRESET_COLOR_NAMES: readonly string[] = [
  "Blue",
  "Green",
  "Pink",
  "Amber",
  "Violet",
  "Red",
  "Teal",
  "Slate",
];

/** Returns `true` if `value` is a 6-digit hex color (`#RRGGBB`, case-insensitive). */
export function isHexColor(value: string): boolean {
  return /^#[0-9a-fA-F]{6}$/.test(value);
}

/** Matches `oklch(L C H)` or `oklch(L% C H)`, with an optional `/ alpha` component. */
const OKLCH_PATTERN = /^oklch\(\s*([\d.]+)(%?)\s+([\d.]+)\s+([\d.]+)(?:\s*\/\s*[\d.]+%?)?\s*\)$/i;

/** Gamma-encodes a linear-light sRGB channel (0-1) per the CSS Color 4 spec. */
function srgbGammaEncode(channel: number): number {
  const clamped = Math.min(Math.max(channel, 0), 1);
  return clamped <= 0.0031308 ? 12.92 * clamped : 1.055 * clamped ** (1 / 2.4) - 0.055;
}

/** Converts a 0-1 channel value to a 2-digit hex byte. */
function toHexByte(channel: number): string {
  return Math.round(Math.min(Math.max(channel, 0), 1) * 255)
    .toString(16)
    .padStart(2, "0");
}

/**
 * Converts OKLCH coordinates to linear-light sRGB channels (not yet
 * gamma-encoded or clamped to `[0, 1]`), using the OKLab -> linear sRGB
 * matrices from the CSS Color 4 specification. Shared by `cssColorToHex`
 * (which gamma-encodes and clamps the result) and `shadesOf`'s gamut check
 * (which needs the unclamped channels to detect when clamping would distort
 * hue, before that clamping happens).
 */
function oklchToLinearSrgb(lightness: number, chroma: number, hueDegrees: number): [number, number, number] {
  const hueRadians = (hueDegrees * Math.PI) / 180;
  const a = chroma * Math.cos(hueRadians);
  const b = chroma * Math.sin(hueRadians);

  const lightCubeRoot = lightness + 0.3963377774 * a + 0.2158037573 * b;
  const midCubeRoot = lightness - 0.1055613458 * a - 0.0638541728 * b;
  const shortCubeRoot = lightness - 0.0894841775 * a - 1.2914855480 * b;

  const long = lightCubeRoot ** 3;
  const mid = midCubeRoot ** 3;
  const short = shortCubeRoot ** 3;

  const red = 4.0767416621 * long - 3.3077115913 * mid + 0.2309699292 * short;
  const green = -1.2684380046 * long + 2.6097574011 * mid - 0.3413193965 * short;
  const blue = -0.0041960863 * long - 0.7034186147 * mid + 1.7076147010 * short;

  return [red, green, blue];
}

/**
 * Converts a CSS `oklch(...)` color to its 6-digit hex equivalent.
 * Already-hex colors are lowercased and returned as-is; any other format is
 * returned unchanged (the caller's existing `CSS.supports` check flags those
 * as invalid).
 */
export function cssColorToHex(value: string): string {
  const trimmed = value.trim();
  if (isHexColor(trimmed)) return trimmed.toLowerCase();

  const match = OKLCH_PATTERN.exec(trimmed);
  if (!match) return value;

  const [, lightnessRaw, percentSign, chromaRaw, hueRaw] = match;
  const lightness = percentSign ? Number(lightnessRaw) / 100 : Number(lightnessRaw);
  const chroma = Number(chromaRaw);

  const [red, green, blue] = oklchToLinearSrgb(lightness, chroma, Number(hueRaw));

  return `#${toHexByte(srgbGammaEncode(red))}${toHexByte(srgbGammaEncode(green))}${toHexByte(srgbGammaEncode(blue))}`;
}

/** Decodes a gamma-encoded sRGB channel (0-1) to linear light, per the WCAG relative luminance formula. */
function srgbLinearize(channel: number): number {
  return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
}

/**
 * Returns the WCAG relative luminance (0-1) of a 6-digit hex color.
 * Non-hex input (e.g. an unmigrated legacy `oklch(...)` value) is treated as
 * mid-gray (0.5), so callers fall back to a neutral light/dark decision
 * rather than guessing wrong in either direction.
 */
export function relativeLuminance(hex: string): number {
  if (!isHexColor(hex)) return 0.5;

  const r = srgbLinearize(parseInt(hex.slice(1, 3), 16) / 255);
  const g = srgbLinearize(parseInt(hex.slice(3, 5), 16) / 255);
  const b = srgbLinearize(parseInt(hex.slice(5, 7), 16) / 255);

  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/**
 * Returns `true` if `hex` is light enough that near-black text reads better
 * on it than near-white text (used by "color code" card mode to keep task
 * titles legible against an arbitrary project color, including white).
 */
export function isLightColor(hex: string): boolean {
  return relativeLuminance(hex) > 0.45;
}

/**
 * Converts a 6-digit hex color to OKLCH lightness (0-1), chroma, and hue
 * (degrees), using the linear-sRGB -> OKLab matrices from the CSS Color 4
 * specification — the inverse of `cssColorToHex`'s OKLab -> linear-sRGB
 * conversion. Returns all-zero for non-hex input.
 */
export function hexToOklch(hex: string): { l: number; c: number; h: number } {
  if (!isHexColor(hex)) return { l: 0, c: 0, h: 0 };

  const r = srgbLinearize(parseInt(hex.slice(1, 3), 16) / 255);
  const g = srgbLinearize(parseInt(hex.slice(3, 5), 16) / 255);
  const b = srgbLinearize(parseInt(hex.slice(5, 7), 16) / 255);

  const longCone = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const midCone = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const shortCone = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);

  const l = 0.2104542553 * longCone + 0.793617785 * midCone - 0.0040720468 * shortCone;
  const a = 1.9779984951 * longCone - 2.428592205 * midCone + 0.4505937099 * shortCone;
  const bComponent = 0.0259040371 * longCone + 0.7827717662 * midCone - 0.808675766 * shortCone;

  const c = Math.sqrt(a * a + bComponent * bComponent);
  const hueDegrees = (Math.atan2(bComponent, a) * 180) / Math.PI;

  return { l, c, h: hueDegrees < 0 ? hueDegrees + 360 : hueDegrees };
}

/**
 * Returns a CSS `oklch(...)` string using `hex`'s hue and chroma but a fixed
 * `lightness` (0-1), for "neon" card backgrounds: vivid/saturated rather
 * than the washed-out pastel a `color-mix(..., white)` dilution produces,
 * while staying bright enough for a single fixed text color (chosen by the
 * caller, not computed per-color) to stay legible across every project
 * color. `chromaBoost` scales the original chroma for extra vibrancy;
 * out-of-gamut results are clamped to a believable ceiling — browsers also
 * gamut-map `oklch()` automatically, so this is a defensive belt-and-braces
 * cap, not the only thing preventing an absurd value.
 */
export function neonCardColor(hex: string, lightness: number, chromaBoost = 1): string {
  const { c, h } = hexToOklch(hex);
  const boostedChroma = Math.min(c * chromaBoost, 0.4);
  return `oklch(${(lightness * 100).toFixed(1)}% ${boostedChroma.toFixed(4)} ${h.toFixed(1)})`;
}

/**
 * Shared `neonCardColor` lightness/chroma-boost for "color code" Kanban card
 * mode (TaskCard). A single source of truth so the fixed-lightness target
 * (and therefore "dark text is always legible") can't drift out of sync
 * with what's actually rendered.
 */
export const NEON_CARD_LIGHTNESS = 0.5;
export const NEON_CARD_CHROMA_BOOST = 1.55;

/** Lightness range `shadesOf` spreads its suggestions across — stays clear of near-black/near-white extremes where hue becomes visually indistinct. */
const SHADE_MIN_LIGHTNESS = 0.3;
const SHADE_MAX_LIGHTNESS = 0.7;

/** Returns `true` if all three linear-sRGB channels are within `[0, 1]` (within a small tolerance for floating-point error) — i.e. representable without per-channel clamping, which would otherwise distort hue. */
function isInGamut(rgb: readonly [number, number, number]): boolean {
  const tolerance = 1e-4;
  return rgb.every((channel) => channel >= -tolerance && channel <= 1 + tolerance);
}

/**
 * Returns the largest chroma in `[0, chroma]` (at the given lightness/hue)
 * that stays within the sRGB gamut, via binary search. Used by `shadesOf` so
 * its hex output never needs the per-channel clamping `cssColorToHex` would
 * otherwise apply for an out-of-gamut color — clamping each channel
 * independently shifts the resulting hue away from the parent's, which is
 * exactly what `shadesOf` exists to avoid.
 */
function maxInGamutChroma(lightness: number, chroma: number, hueDegrees: number): number {
  if (chroma <= 0 || isInGamut(oklchToLinearSrgb(lightness, chroma, hueDegrees))) return chroma;

  let low = 0;
  let high = chroma;
  for (let i = 0; i < 20; i++) {
    const mid = (low + high) / 2;
    if (isInGamut(oklchToLinearSrgb(lightness, mid, hueDegrees))) {
      low = mid;
    } else {
      high = mid;
    }
  }
  return low;
}

/**
 * Returns `count` color suggestions derived from `parentHex`'s hue and
 * chroma, varying only lightness, spread evenly across
 * `[SHADE_MIN_LIGHTNESS, SHADE_MAX_LIGHTNESS]` — the model for a
 * subproject's color picker defaults, so its suggested colors read as
 * "shades of the parent" rather than unrelated hues. Built on
 * `neonCardColor` (same hue/chroma, different fixed lightness) — the
 * existing precedent for this exact kind of derivation — converted to hex
 * since `Project.color` is hex-only. Chroma is reduced per-shade (via
 * `maxInGamutChroma`) when the parent's chroma would otherwise fall outside
 * the sRGB gamut at that lightness, so the hue stays true to the parent
 * across the whole range rather than drifting from clamped channels.
 */
export function shadesOf(parentHex: string, count: number): string[] {
  const { c: parentChroma, h: hue } = hexToOklch(parentHex);
  const step = count > 1 ? (SHADE_MAX_LIGHTNESS - SHADE_MIN_LIGHTNESS) / (count - 1) : 0;
  return Array.from({ length: count }, (_, index) => {
    const lightness = SHADE_MIN_LIGHTNESS + step * index;
    const safeChromaBoost =
      parentChroma > 0 ? maxInGamutChroma(lightness, parentChroma, hue) / parentChroma : 1;
    return cssColorToHex(neonCardColor(parentHex, lightness, safeChromaBoost));
  });
}

/**
 * Same idea as `NEON_CARD_LIGHTNESS`/`NEON_CARD_CHROMA_BOOST`, but for Week
 * view's bars specifically — deliberately darker/more saturated than the
 * Kanban card treatment, by request, rather than sharing one constant.
 */
export const WEEK_BAR_LIGHTNESS = 0.38;
export const WEEK_BAR_CHROMA_BOOST = 1.6;

/** WCAG contrast ratio between two relative luminances (each 0-1, as returned by `relativeLuminance`). */
function contrastRatio(luminanceA: number, luminanceB: number): number {
  const lighter = Math.max(luminanceA, luminanceB);
  const darker = Math.min(luminanceA, luminanceB);
  return (lighter + 0.05) / (darker + 0.05);
}

const DARK_INK = "oklch(20% 0.014 50)";
const LIGHT_INK = "oklch(96% 0.01 0)";

/** Mirrors the backend's `settings::INK_MODES` — keep both lists in sync. */
export type InkMode = "auto" | "white" | "black";

/**
 * Returns whichever of two literal ink colors (not theme tokens — the
 * color-coded background's brightness has nothing to do with which theme
 * is active, so a theme-variant token like `--color-ink` would pick the
 * wrong color in plenty of cases) has the higher WCAG contrast ratio
 * against `backgroundColor` (any CSS color `cssColorToHex` understands,
 * e.g. a `neonCardColor` result), unless `mode` forces a specific choice.
 *
 * `mode` defaults to `"auto"` (real contrast math against the *resolved*
 * color, not a lightness-only heuristic: chroma and hue both affect a
 * color's actual luminance just as much as its OKLCH lightness does — see
 * the regression test pinned to the app's default project color/lightness
 * for the exact case an earlier lightness-only heuristic got wrong).
 * `"white"`/`"black"` force that choice regardless of contrast, for users
 * who'd rather pick a fixed ink color than have it computed per-task.
 */
export function legibleInkColor(backgroundColor: string, mode: InkMode = "auto"): string {
  if (mode === "white") return LIGHT_INK;
  if (mode === "black") return DARK_INK;

  const backgroundLuminance = relativeLuminance(cssColorToHex(backgroundColor));
  const darkInkLuminance = relativeLuminance(cssColorToHex(DARK_INK));
  const lightInkLuminance = relativeLuminance(cssColorToHex(LIGHT_INK));
  return contrastRatio(backgroundLuminance, darkInkLuminance) >=
    contrastRatio(backgroundLuminance, lightInkLuminance)
    ? DARK_INK
    : LIGHT_INK;
}
