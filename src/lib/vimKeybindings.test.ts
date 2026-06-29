import { describe, expect, it } from "vitest";

import {
  DEFAULT_BINDINGS,
  DEFAULT_STATUS_SHORTCUTS,
  effectiveCombos,
  matchesAction,
  matchesCombo,
  parseKeyCombo,
  type VimActionId,
} from "./vimKeybindings";
import type { VimKeybinding } from "$lib/types";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// Duck-typed event stub — parseKeyCombo / matchesCombo / matchesAction only read
// key, ctrlKey, altKey, and shiftKey; no DOM API is required.
function key(k: string, mods: { ctrlKey?: boolean; altKey?: boolean; shiftKey?: boolean } = {}): KeyboardEvent {
  return {
    key: k,
    ctrlKey: mods.ctrlKey ?? false,
    altKey: mods.altKey ?? false,
    shiftKey: mods.shiftKey ?? false,
  } as unknown as KeyboardEvent;
}

// ---------------------------------------------------------------------------
// parseKeyCombo
// ---------------------------------------------------------------------------

describe("parseKeyCombo", () => {
  it("returns the bare key for single lower-case letters", () => {
    expect(parseKeyCombo(key("j"))).toBe("j");
    expect(parseKeyCombo(key("k"))).toBe("k");
    expect(parseKeyCombo(key("h"))).toBe("h");
    expect(parseKeyCombo(key("l"))).toBe("l");
    expect(parseKeyCombo(key("e"))).toBe("e");
    expect(parseKeyCombo(key("v"))).toBe("v");
    expect(parseKeyCombo(key("n"))).toBe("n");
    expect(parseKeyCombo(key("s"))).toBe("s");
  });

  it("returns uppercase for shifted letter keys (shift implied by casing)", () => {
    // Shift + g produces key="G"; shift should NOT be prepended for letters
    expect(parseKeyCombo(key("G", { shiftKey: true }))).toBe("G");
    expect(parseKeyCombo(key("D", { shiftKey: true }))).toBe("D");
  });

  it("returns Shift+ prefix for non-letter shifted keys (e.g. ArrowLeft)", () => {
    expect(parseKeyCombo(key("ArrowLeft", { shiftKey: true }))).toBe("Shift+ArrowLeft");
    expect(parseKeyCombo(key("]", { shiftKey: true }))).toBe("]"); // ] is a single char, shift NOT prepended
  });

  it("handles Ctrl modifier", () => {
    expect(parseKeyCombo(key("k", { ctrlKey: true }))).toBe("Ctrl+k");
    expect(parseKeyCombo(key("s", { ctrlKey: true }))).toBe("Ctrl+s");
    expect(parseKeyCombo(key("Enter", { ctrlKey: true }))).toBe("Ctrl+Enter");
  });

  it("handles Alt modifier", () => {
    expect(parseKeyCombo(key("j", { altKey: true }))).toBe("Alt+j");
  });

  it("handles Ctrl+Alt together", () => {
    expect(parseKeyCombo(key("t", { ctrlKey: true, altKey: true }))).toBe("Ctrl+Alt+t");
  });

  it("returns bare key for special keys without modifiers", () => {
    expect(parseKeyCombo(key("Escape"))).toBe("Escape");
    expect(parseKeyCombo(key("Enter"))).toBe("Enter");
    expect(parseKeyCombo(key("ArrowLeft"))).toBe("ArrowLeft");
    expect(parseKeyCombo(key("ArrowRight"))).toBe("ArrowRight");
    expect(parseKeyCombo(key("ArrowUp"))).toBe("ArrowUp");
    expect(parseKeyCombo(key("ArrowDown"))).toBe("ArrowDown");
  });

  it("returns space character for Space key", () => {
    expect(parseKeyCombo(key(" "))).toBe(" ");
  });

  it("returns bracket characters for ] and [", () => {
    expect(parseKeyCombo(key("]"))).toBe("]");
    expect(parseKeyCombo(key("["))).toBe("[");
  });
});

// ---------------------------------------------------------------------------
// matchesCombo
// ---------------------------------------------------------------------------

describe("matchesCombo", () => {
  it("returns true when event matches combo exactly", () => {
    expect(matchesCombo(key("j"), "j")).toBe(true);
    expect(matchesCombo(key("G", { shiftKey: true }), "G")).toBe(true);
    expect(matchesCombo(key("k", { ctrlKey: true }), "Ctrl+k")).toBe(true);
  });

  it("returns false for mismatched combos", () => {
    expect(matchesCombo(key("j"), "k")).toBe(false);
    expect(matchesCombo(key("j"), "Ctrl+j")).toBe(false);
    expect(matchesCombo(key("j", { ctrlKey: true }), "j")).toBe(false);
  });

  it("matches space as bare space character", () => {
    expect(matchesCombo(key(" "), " ")).toBe(true);
    expect(matchesCombo(key("j"), " ")).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// DEFAULT_BINDINGS — regression guards
// ---------------------------------------------------------------------------

describe("DEFAULT_BINDINGS", () => {
  it("has expected movement keys", () => {
    expect(DEFAULT_BINDINGS.move_down).toContain("j");
    expect(DEFAULT_BINDINGS.move_up).toContain("k");
    expect(DEFAULT_BINDINGS.move_left).toContain("h");
    expect(DEFAULT_BINDINGS.move_right).toContain("l");
  });

  it("has G for go_to_bottom (Bug #1 regression)", () => {
    expect(DEFAULT_BINDINGS.go_to_bottom).toContain("G");
  });

  it("has gg listed for go_to_top (two-key sequence doc)", () => {
    expect(DEFAULT_BINDINGS.go_to_top).toContain("gg");
  });

  it("has ArrowLeft/Right for tab switching", () => {
    expect(DEFAULT_BINDINGS.next_tab).toContain("ArrowRight");
    expect(DEFAULT_BINDINGS.prev_tab).toContain("ArrowLeft");
  });

  it("has ArrowDown/Up for project switching", () => {
    expect(DEFAULT_BINDINGS.next_project).toContain("ArrowDown");
    expect(DEFAULT_BINDINGS.prev_project).toContain("ArrowUp");
  });

  it("has ] and [ for period navigation (Bug Round-2 #15)", () => {
    expect(DEFAULT_BINDINGS.next_period).toContain("]");
    expect(DEFAULT_BINDINGS.prev_period).toContain("[");
  });

  // Bug Round-2 #11: Space and Enter must be configurable actions
  it("has Space as toggle action (Bug Round-2 #11)", () => {
    expect(DEFAULT_BINDINGS.toggle).toContain(" ");
  });

  it("has Enter as confirm action (Bug Round-2 #11)", () => {
    expect(DEFAULT_BINDINGS.confirm).toContain("Enter");
  });

  it("has e for edit_task", () => {
    expect(DEFAULT_BINDINGS.edit_task).toContain("e");
  });

  it("has D for delete_task (uppercase shift-d)", () => {
    expect(DEFAULT_BINDINGS.delete_task).toContain("D");
  });

  it("covers all VimActionId values", () => {
    const allIds: VimActionId[] = [
      "move_down", "move_up", "move_left", "move_right",
      "go_to_top", "go_to_bottom",
      "next_tab", "prev_tab", "next_project", "prev_project",
      "next_period", "prev_period",
      "enter_visual", "enter_sparse",
      "toggle", "confirm",
      "edit_task", "toggle_timer", "new_task", "delete_task",
    ];
    for (const id of allIds) {
      expect(DEFAULT_BINDINGS[id], `Missing binding for "${id}"`).toBeDefined();
    }
  });
});

// ---------------------------------------------------------------------------
// DEFAULT_STATUS_SHORTCUTS — Bug #5 regression
// ---------------------------------------------------------------------------

describe("DEFAULT_STATUS_SHORTCUTS (Bug #5 regression)", () => {
  it("ships defaults for built-in statuses so they appear in settings panel", () => {
    expect(DEFAULT_STATUS_SHORTCUTS.done).toBe("d");
    expect(DEFAULT_STATUS_SHORTCUTS["in progress"]).toBe("i");
    expect(DEFAULT_STATUS_SHORTCUTS.blocked).toBe("b");
    expect(DEFAULT_STATUS_SHORTCUTS.cancelled).toBe("c");
  });
});

// ---------------------------------------------------------------------------
// effectiveCombos
// ---------------------------------------------------------------------------

describe("effectiveCombos", () => {
  const noOverrides: VimKeybinding[] = [];

  it("returns default combos when no user overrides exist", () => {
    expect(effectiveCombos("move_down", noOverrides)).toEqual(["j"]);
    expect(effectiveCombos("move_up", noOverrides)).toEqual(["k"]);
  });

  // Bug #4: hotkey changes must apply at runtime — effectiveCombos drives this
  it("returns user override combos when one is provided (Bug #4 regression)", () => {
    const overrides: VimKeybinding[] = [{ action_id: "move_down", combos: ["n", "Ctrl+j"] }];
    expect(effectiveCombos("move_down", overrides)).toEqual(["n", "Ctrl+j"]);
  });

  it("user override for one action does not affect other actions", () => {
    const overrides: VimKeybinding[] = [{ action_id: "move_down", combos: ["n"] }];
    // move_up should still use its default
    expect(effectiveCombos("move_up", overrides)).toEqual(["k"]);
  });

  it("user override entirely replaces the default (no merging)", () => {
    const overrides: VimKeybinding[] = [{ action_id: "edit_task", combos: ["Ctrl+e"] }];
    const result = effectiveCombos("edit_task", overrides);
    // Should NOT include the default "e" — override replaces, not merges
    expect(result).not.toContain("e");
    expect(result).toContain("Ctrl+e");
  });

  it("returns empty array for an action not in DEFAULT_BINDINGS and no override", () => {
    // Using a cast since this is an edge case
    const result = effectiveCombos("unknown_action" as VimActionId, noOverrides);
    expect(result).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// matchesAction
// ---------------------------------------------------------------------------

describe("matchesAction", () => {
  const noOverrides: VimKeybinding[] = [];

  it("returns true for default binding", () => {
    expect(matchesAction("move_down", key("j"), noOverrides)).toBe(true);
    expect(matchesAction("move_up", key("k"), noOverrides)).toBe(true);
    expect(matchesAction("move_left", key("h"), noOverrides)).toBe(true);
    expect(matchesAction("move_right", key("l"), noOverrides)).toBe(true);
    expect(matchesAction("go_to_bottom", key("G", { shiftKey: true }), noOverrides)).toBe(true);
    expect(matchesAction("next_tab", key("ArrowRight"), noOverrides)).toBe(true);
    expect(matchesAction("prev_tab", key("ArrowLeft"), noOverrides)).toBe(true);
    expect(matchesAction("next_period", key("]"), noOverrides)).toBe(true);
    expect(matchesAction("prev_period", key("["), noOverrides)).toBe(true);
    expect(matchesAction("edit_task", key("e"), noOverrides)).toBe(true);
    expect(matchesAction("toggle", key(" "), noOverrides)).toBe(true);
    expect(matchesAction("confirm", key("Enter"), noOverrides)).toBe(true);
  });

  it("returns false for wrong key", () => {
    expect(matchesAction("move_down", key("k"), noOverrides)).toBe(false);
    expect(matchesAction("edit_task", key("j"), noOverrides)).toBe(false);
  });

  // Bug #4: user overrides must be respected by matchesAction
  it("matches user-overridden combo instead of default (Bug #4 regression)", () => {
    const overrides: VimKeybinding[] = [{ action_id: "move_down", combos: ["n"] }];
    expect(matchesAction("move_down", key("n"), overrides)).toBe(true);
    expect(matchesAction("move_down", key("j"), overrides)).toBe(false); // default no longer active
  });

  it("returns false for any key when combo list is empty", () => {
    const overrides: VimKeybinding[] = [{ action_id: "edit_task", combos: [] }];
    expect(matchesAction("edit_task", key("e"), overrides)).toBe(false);
  });

  it("does NOT match two-key sequences (gg / vv) — those are handled by sequence buffer", () => {
    // go_to_top has ["gg"] as its only combo; matchesAction should skip it
    expect(matchesAction("go_to_top", key("g"), noOverrides)).toBe(false);
    // enter_sparse has ["vv"]; matchesAction should skip it
    expect(matchesAction("enter_sparse", key("v"), noOverrides)).toBe(false);
  });

  it("matches enter_visual with single v (not skipped as sequence)", () => {
    expect(matchesAction("enter_visual", key("v"), noOverrides)).toBe(true);
  });

  it("handles multiple combos for an action — any match returns true", () => {
    const overrides: VimKeybinding[] = [{ action_id: "move_down", combos: ["j", "Ctrl+j"] }];
    expect(matchesAction("move_down", key("j"), overrides)).toBe(true);
    expect(matchesAction("move_down", key("j", { ctrlKey: true }), overrides)).toBe(true);
    expect(matchesAction("move_down", key("k"), overrides)).toBe(false);
  });
});
