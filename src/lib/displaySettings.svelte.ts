const FONT_SCALE_KEY = "taskmancer:font-scale";
const COLUMN_WIDTH_KEY = "taskmancer:column-width";
const BOARD_WIDTH_KEY = "taskmancer:board-width";
const SHOW_PRIORITY_GROUPS_KEY = "taskmancer:show-priority-groups";
const SHOW_PRIORITY_CHIP_KEY = "taskmancer:show-priority-chip";

export const MIN_FONT_SCALE = 80;
export const MAX_FONT_SCALE = 140;
export const FONT_SCALE_STEP = 5;
export const DEFAULT_FONT_SCALE = 100;

export const MIN_COLUMN_WIDTH = 200;
export const MAX_COLUMN_WIDTH = 400;
export const COLUMN_WIDTH_STEP = 10;
export const DEFAULT_COLUMN_WIDTH = 240;

export const MIN_BOARD_WIDTH = 1000;
export const MAX_BOARD_WIDTH = 2000;
export const BOARD_WIDTH_STEP = 50;
export const DEFAULT_BOARD_WIDTH = 1200;

export const DEFAULT_SHOW_PRIORITY_GROUPS = true;
export const DEFAULT_SHOW_PRIORITY_CHIP = true;

/**
 * Boxed in an object because Svelte 5 forbids exporting a reassigned `$state`
 * binding directly from a module — only its properties may be mutated.
 */
export const displayState = $state<{
  fontScale: number;
  columnWidth: number;
  boardWidth: number;
  showPriorityGroups: boolean;
  showPriorityChip: boolean;
}>({
  fontScale: DEFAULT_FONT_SCALE,
  columnWidth: DEFAULT_COLUMN_WIDTH,
  boardWidth: DEFAULT_BOARD_WIDTH,
  showPriorityGroups: DEFAULT_SHOW_PRIORITY_GROUPS,
  showPriorityChip: DEFAULT_SHOW_PRIORITY_CHIP,
});

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

/** Sets the whole-app font-size scale (as a percentage of the browser default) and persists it. */
export function setFontScale(value: number): void {
  const clamped = clamp(value, MIN_FONT_SCALE, MAX_FONT_SCALE);
  displayState.fontScale = clamped;
  document.documentElement.style.fontSize = `${clamped}%`;
  try {
    localStorage.setItem(FONT_SCALE_KEY, String(clamped));
  } catch {
    // Persistence is best-effort; the choice still applies for this session.
  }
}

/** Sets the Kanban status column width (in pixels) and persists it. */
export function setColumnWidth(value: number): void {
  const clamped = clamp(value, MIN_COLUMN_WIDTH, MAX_COLUMN_WIDTH);
  displayState.columnWidth = clamped;
  document.documentElement.style.setProperty("--column-width", `${clamped}px`);
  try {
    localStorage.setItem(COLUMN_WIDTH_KEY, String(clamped));
  } catch {
    // Persistence is best-effort; the choice still applies for this session.
  }
}

/** Sets the Kanban board's overall width (in pixels) and persists it. */
export function setBoardWidth(value: number): void {
  const clamped = clamp(value, MIN_BOARD_WIDTH, MAX_BOARD_WIDTH);
  displayState.boardWidth = clamped;
  document.documentElement.style.setProperty("--board-width", `${clamped}px`);
  try {
    localStorage.setItem(BOARD_WIDTH_KEY, String(clamped));
  } catch {
    // Persistence is best-effort; the choice still applies for this session.
  }
}

/** Sets whether Kanban columns divide tasks into priority groups and persists it. */
export function setShowPriorityGroups(value: boolean): void {
  displayState.showPriorityGroups = value;
  try {
    localStorage.setItem(SHOW_PRIORITY_GROUPS_KEY, value ? "true" : "false");
  } catch {
    // Persistence is best-effort; the choice still applies for this session.
  }
}

/** Sets whether task cards show their priority chip (name + dot) and persists it. */
export function setShowPriorityChip(value: boolean): void {
  displayState.showPriorityChip = value;
  try {
    localStorage.setItem(SHOW_PRIORITY_CHIP_KEY, value ? "true" : "false");
  } catch {
    // Persistence is best-effort; the choice still applies for this session.
  }
}

/** Parses a persisted numeric setting, falling back to `fallback` when missing, invalid, or out of range. */
function parseStoredNumber(raw: string | null, min: number, max: number, fallback: number): number {
  if (raw === null) return fallback;
  const parsed = Number(raw);
  if (!Number.isFinite(parsed)) return fallback;
  return clamp(parsed, min, max);
}

/** Restores previously persisted display settings, falling back to defaults. */
export function initDisplay(): void {
  let storedFontScale: string | null = null;
  let storedColumnWidth: string | null = null;
  let storedBoardWidth: string | null = null;
  let storedShowPriorityGroups: string | null = null;
  let storedShowPriorityChip: string | null = null;
  try {
    storedFontScale = localStorage.getItem(FONT_SCALE_KEY);
    storedColumnWidth = localStorage.getItem(COLUMN_WIDTH_KEY);
    storedBoardWidth = localStorage.getItem(BOARD_WIDTH_KEY);
    storedShowPriorityGroups = localStorage.getItem(SHOW_PRIORITY_GROUPS_KEY);
    storedShowPriorityChip = localStorage.getItem(SHOW_PRIORITY_CHIP_KEY);
  } catch {
    // Fall back to defaults below.
  }

  setFontScale(parseStoredNumber(storedFontScale, MIN_FONT_SCALE, MAX_FONT_SCALE, DEFAULT_FONT_SCALE));
  setColumnWidth(parseStoredNumber(storedColumnWidth, MIN_COLUMN_WIDTH, MAX_COLUMN_WIDTH, DEFAULT_COLUMN_WIDTH));
  setBoardWidth(parseStoredNumber(storedBoardWidth, MIN_BOARD_WIDTH, MAX_BOARD_WIDTH, DEFAULT_BOARD_WIDTH));
  setShowPriorityGroups(
    storedShowPriorityGroups === null ? DEFAULT_SHOW_PRIORITY_GROUPS : storedShowPriorityGroups === "true",
  );
  setShowPriorityChip(
    storedShowPriorityChip === null ? DEFAULT_SHOW_PRIORITY_CHIP : storedShowPriorityChip === "true",
  );
}
