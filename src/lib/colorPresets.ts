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
 * Converts a CSS `oklch(...)` color to its 6-digit hex equivalent, using the
 * OKLab -> linear sRGB matrices from the CSS Color 4 specification. Already-hex
 * colors are lowercased and returned as-is; any other format is returned
 * unchanged (the caller's existing `CSS.supports` check flags those as invalid).
 */
export function cssColorToHex(value: string): string {
  const trimmed = value.trim();
  if (isHexColor(trimmed)) return trimmed.toLowerCase();

  const match = OKLCH_PATTERN.exec(trimmed);
  if (!match) return value;

  const [, lightnessRaw, percentSign, chromaRaw, hueRaw] = match;
  const lightness = percentSign ? Number(lightnessRaw) / 100 : Number(lightnessRaw);
  const chroma = Number(chromaRaw);
  const hueRadians = (Number(hueRaw) * Math.PI) / 180;

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

  return `#${toHexByte(srgbGammaEncode(red))}${toHexByte(srgbGammaEncode(green))}${toHexByte(srgbGammaEncode(blue))}`;
}
