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
