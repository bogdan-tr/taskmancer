export const THEMES = ["light", "dark", "dark-blue"] as const;

export type Theme = (typeof THEMES)[number];

export const DEFAULT_THEME: Theme = "light";

export const THEME_LABELS: Record<Theme, string> = {
  light: "Light",
  dark: "Dark",
  "dark-blue": "Dark Blue",
};

/** Returns true if `value` is a recognized {@link Theme}. */
export function isTheme(value: unknown): value is Theme {
  return typeof value === "string" && (THEMES as readonly string[]).includes(value);
}
