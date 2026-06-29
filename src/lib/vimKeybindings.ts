/**
 * Default keybindings for vim mode. Each entry maps an action_id to one or more
 * key combo strings. Format: bare key ("j"), shifted ("G"), modified ("Ctrl+k").
 *
 * Two-key sequences ("gg", "vv") are handled in the vim store via a sequence
 * buffer and timer — they are listed here as documentation only and are NOT
 * matched by `matchesCombo` or `matchesAction` (which operate on single key events).
 */

import type { VimKeybinding } from "$lib/types";

export type VimActionId =
  | "move_down"
  | "move_up"
  | "move_left"
  | "move_right"
  | "go_to_top"
  | "go_to_bottom"
  | "next_tab"
  | "prev_tab"
  | "next_project"
  | "prev_project"
  | "next_period"
  | "prev_period"
  | "enter_visual"
  | "enter_sparse"
  | "toggle"
  | "confirm"
  | "edit_task"
  | "toggle_timer"
  | "new_task"
  | "delete_task";

export const DEFAULT_BINDINGS: Record<VimActionId, string[]> = {
  move_down: ["j"],
  move_up: ["k"],
  move_left: ["h"],
  move_right: ["l"],
  go_to_top: ["gg"], // two-key sequence — matched in vim store
  go_to_bottom: ["G"],
  next_tab: ["ArrowRight"],
  prev_tab: ["ArrowLeft"],
  next_project: ["ArrowDown"],
  prev_project: ["ArrowUp"],
  next_period: ["]"],   // next week (week view) or next month (calendar view)
  prev_period: ["["],   // prev week / prev month
  enter_visual: ["v"],  // also first key of "vv" sequence
  enter_sparse: ["vv"], // two-key sequence — matched in vim store
  toggle: [" "],  // Space: expand/collapse in sidebar, toggle select in visual_sparse
  confirm: ["Enter"],   // navigate sidebar cursor OR edit focused task
  edit_task: ["e"],
  toggle_timer: ["s"],
  new_task: ["n"],
  delete_task: ["D"],
};

/** Default status → combo pairs for built-in statuses. Matched by label heuristic at runtime. */
export const DEFAULT_STATUS_SHORTCUTS: Record<string, string> = {
  done: "d",
  "in progress": "i",
  blocked: "b",
  cancelled: "c",
};

/**
 * Parses a KeyboardEvent into a normalized combo string.
 * Examples: "j", "G" (shift+g gives uppercase "G"), "Ctrl+k", "Shift+ArrowLeft",
 * "ArrowLeft", " " (space), "Escape".
 *
 * Shift is only prepended for non-letter keys (like "Shift+ArrowLeft") because
 * uppercase letters already encode the shift (e.g. "G" not "Shift+g").
 */
export function parseKeyCombo(event: KeyboardEvent): string {
  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  // Shift is implicit in uppercase letters; only prepend "Shift+" for non-letter keys
  if (event.shiftKey && event.key.length > 1) parts.push("Shift");
  parts.push(event.key); // e.g. "j", "G", "ArrowLeft", " ", "Escape"
  return parts.join("+");
}

/** Returns true if `event` matches the given combo string. */
export function matchesCombo(event: KeyboardEvent, combo: string): boolean {
  return parseKeyCombo(event) === combo;
}

/**
 * Resolve effective bindings for an action, merging user overrides over defaults.
 * User `keybindings` with the same `action_id` entirely replace the default for that action.
 */
export function effectiveCombos(actionId: VimActionId, userBindings: VimKeybinding[]): string[] {
  const override = userBindings.find((b) => b.action_id === actionId);
  return override ? override.combos : (DEFAULT_BINDINGS[actionId] ?? []);
}

/**
 * Check if `event` triggers `actionId` given the current user keybindings.
 * Two-key sequences ("gg", "vv") are excluded here — handled in the vim store.
 */
export function matchesAction(
  actionId: VimActionId,
  event: KeyboardEvent,
  userBindings: VimKeybinding[],
): boolean {
  return effectiveCombos(actionId, userBindings).some((combo) => {
    // Skip two-key sequences — the store handles these via its sequence buffer
    if (combo === "gg" || combo === "vv") return false;
    return matchesCombo(event, combo);
  });
}
