import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// ---------------------------------------------------------------------------
// DOM stubs (no jsdom available in this project's test environment)
// vim.svelte.ts calls document.dispatchEvent(new CustomEvent(...)) for
// tab-switching and period-navigation events. We stub these globally so the
// module can run in the Node.js vitest environment.
// ---------------------------------------------------------------------------

class FakeCustomEvent {
  type: string;
  detail: unknown;
  constructor(type: string, opts?: { detail?: unknown }) {
    this.type = type;
    this.detail = opts?.detail;
  }
}

let capturedEvents: FakeCustomEvent[] = [];

const fakeDocument = {
  dispatchEvent: vi.fn((e: FakeCustomEvent) => { capturedEvents.push(e); }),
  addEventListener: vi.fn(),
  removeEventListener: vi.fn(),
};

vi.stubGlobal("CustomEvent", FakeCustomEvent);
vi.stubGlobal("document", fakeDocument);

// Mock $app/navigation before any vim.svelte import
vi.mock("$app/navigation", () => ({ goto: vi.fn().mockResolvedValue(undefined) }));

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// Duck-typed event stub — handleKeydown only reads key, ctrlKey, altKey, shiftKey
function key(k: string, mods: { ctrlKey?: boolean; altKey?: boolean; shiftKey?: boolean } = {}): KeyboardEvent {
  return {
    key: k,
    ctrlKey: mods.ctrlKey ?? false,
    altKey: mods.altKey ?? false,
    shiftKey: mods.shiftKey ?? false,
  } as unknown as KeyboardEvent;
}

type Task = { id: string };
type Bucket = { priorityId: null; label: string; tasks: Task[] };
type Column = { id: string; label: string; buckets: Bucket[] };

function col(id: string, ...taskIds: string[]): Column {
  return { id, label: id, buckets: [{ priorityId: null, label: "", tasks: taskIds.map((t) => ({ id: t })) }] };
}

function wday(date: string, ...taskIds: string[]): { date: string; taskIds: string[] } {
  return { date, taskIds };
}

const BASE_OPTS = {
  activeView: "board",
  vimEnabled: true,
  userBindings: [],
  statusBindings: [],
  statuses: [],
  sidebarItems: [{ route: "/" }],
  currentPageRoute: "/",
  currentProjectId: null,
  editDialogOpen: false,
  onEditTask: vi.fn(),
  onToggleTimer: vi.fn(),
  onNewTask: vi.fn(),
  onDeleteTask: vi.fn(),
  onChangeStatus: vi.fn(),
  onMoveTasksToAdjacentStatus: vi.fn(),
  onMoveTasksToAdjacentDay: vi.fn(),
  onToggleSidebarExpand: vi.fn(),
  weekDays: [],
};

// ---------------------------------------------------------------------------
// Test suite
// ---------------------------------------------------------------------------

describe("vim.svelte — state machine", () => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let vimState: any;

  beforeEach(async () => {
    capturedEvents = [];
    vi.clearAllMocks();
    vi.resetModules();
    vi.doMock("$app/navigation", () => ({ goto: vi.fn().mockResolvedValue(undefined) }));
    const mod = await import("./vim.svelte");
    vimState = mod.vimState;
  });

  afterEach(() => {
    vi.resetModules();
  });

  // -------------------------------------------------------------------------
  // Activation / deactivation
  // -------------------------------------------------------------------------

  describe("Escape — activate / deactivate", () => {
    it("activates vim on first Escape", () => {
      expect(vimState.active).toBe(false);
      const consumed = vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: [] });
      expect(consumed).toBe(true);
      expect(vimState.active).toBe(true);
      expect(vimState.mode).toBe("normal");
    });

    it("deactivates on second Escape in normal mode", () => {
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: [] });
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: [] });
      expect(vimState.active).toBe(false);
    });

    it("vimEnabled=false prevents all key handling", () => {
      const consumed = vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, vimEnabled: false, boardColumns: [] });
      expect(consumed).toBe(false);
      expect(vimState.active).toBe(false);
    });

    it("returns false for all keys when vim is inactive", () => {
      expect(vimState.handleKeydown(key("j"), { ...BASE_OPTS, boardColumns: [] })).toBe(false);
      expect(vimState.handleKeydown(key("k"), { ...BASE_OPTS, boardColumns: [] })).toBe(false);
    });
  });

  // -------------------------------------------------------------------------
  // Bug Round-2 #13 / Round-3 #21: edit dialog suppression
  // -------------------------------------------------------------------------

  describe("editDialogOpen — bug round-2 #13 + round-3 #21", () => {
    it("Escape when dialog is open returns false (dialog closes natively)", () => {
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: [] });
      expect(vimState.active).toBe(true);
      const consumed = vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, editDialogOpen: true, boardColumns: [] });
      expect(consumed).toBe(false);
      expect(vimState.active).toBe(true);
    });

    it("Escape with dialog open does NOT deactivate vim (Bug Round-2 #13)", () => {
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: [] });
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, editDialogOpen: true, boardColumns: [] });
      expect(vimState.active).toBe(true);
      expect(vimState.mode).toBe("normal");
    });

    it("j/k are suppressed when dialog is open (Bug Round-3 #21)", () => {
      const columns = [col("todo", "t1", "t2")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: columns });
      expect(vimState.focusedTaskId).toBe("t1");
      const consumed = vimState.handleKeydown(key("j"), { ...BASE_OPTS, editDialogOpen: true, boardColumns: columns });
      expect(consumed).toBe(false);
      expect(vimState.focusedTaskId).toBe("t1");
    });

    it("e for edit is suppressed when dialog is open", () => {
      const onEditTask = vi.fn();
      const columns = [col("todo", "t1")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: columns });
      const consumed = vimState.handleKeydown(key("e"), { ...BASE_OPTS, editDialogOpen: true, boardColumns: columns, onEditTask });
      expect(consumed).toBe(false);
      expect(onEditTask).not.toHaveBeenCalled();
    });
  });

  // -------------------------------------------------------------------------
  // Bug #3: visual mode must activate instantly on first 'v'
  // -------------------------------------------------------------------------

  describe("visual mode — Bug #3 (instant activation)", () => {
    it("v immediately enters visual mode without delay", () => {
      const columns = [col("todo", "t1", "t2")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: columns });
      const consumed = vimState.handleKeydown(key("v"), { ...BASE_OPTS, boardColumns: columns });
      expect(consumed).toBe(true);
      expect(vimState.mode).toBe("visual");
    });

    it("entering visual mode selects the focused task", () => {
      const columns = [col("todo", "t1", "t2")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: columns });
      expect(vimState.focusedTaskId).toBe("t1");
      vimState.handleKeydown(key("v"), { ...BASE_OPTS, boardColumns: columns });
      expect(vimState.selectedTaskIds.has("t1")).toBe(true);
    });

    it("Escape from visual mode returns to normal, not deactivate", () => {
      const columns = [col("todo", "t1")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: columns });
      vimState.handleKeydown(key("v"), { ...BASE_OPTS, boardColumns: columns });
      expect(vimState.mode).toBe("visual");
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: columns });
      expect(vimState.mode).toBe("normal");
      expect(vimState.active).toBe(true);
    });
  });

  // -------------------------------------------------------------------------
  // Bug Round-2 #12: visual anchor — going up deselects
  // -------------------------------------------------------------------------

  describe("visual mode selection anchor — Bug Round-2 #12", () => {
    it("going down in visual mode extends selection", () => {
      const columns = [col("todo", "t1", "t2", "t3")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: columns });
      vimState.handleKeydown(key("v"), { ...BASE_OPTS, boardColumns: columns });
      vimState.handleKeydown(key("j"), { ...BASE_OPTS, boardColumns: columns });
      expect(vimState.selectedTaskIds.has("t1")).toBe(true);
      expect(vimState.selectedTaskIds.has("t2")).toBe(true);
    });

    it("going back up in visual mode shrinks selection", () => {
      const columns = [col("todo", "t1", "t2", "t3")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: columns });
      vimState.handleKeydown(key("v"), { ...BASE_OPTS, boardColumns: columns });
      vimState.handleKeydown(key("j"), { ...BASE_OPTS, boardColumns: columns }); // t1..t2
      vimState.handleKeydown(key("j"), { ...BASE_OPTS, boardColumns: columns }); // t1..t3
      vimState.handleKeydown(key("k"), { ...BASE_OPTS, boardColumns: columns }); // back to t1..t2
      expect(vimState.selectedTaskIds.has("t1")).toBe(true);
      expect(vimState.selectedTaskIds.has("t2")).toBe(true);
      expect(vimState.selectedTaskIds.has("t3")).toBe(false);
    });

    it("going back to anchor results in only anchor selected", () => {
      const columns = [col("todo", "t1", "t2", "t3")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: columns });
      vimState.handleKeydown(key("v"), { ...BASE_OPTS, boardColumns: columns });
      vimState.handleKeydown(key("j"), { ...BASE_OPTS, boardColumns: columns }); // extend to t2
      vimState.handleKeydown(key("k"), { ...BASE_OPTS, boardColumns: columns }); // back to anchor t1
      expect(vimState.selectedTaskIds.has("t1")).toBe(true);
      expect(vimState.selectedTaskIds.has("t2")).toBe(false);
    });
  });

  // -------------------------------------------------------------------------
  // Board j/k navigation
  // -------------------------------------------------------------------------

  describe("board j/k navigation", () => {
    it("activating vim auto-focuses first task in first non-empty column", () => {
      const columns = [col("todo", "t1", "t2"), col("done", "t3")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: columns });
      expect(vimState.focusedTaskId).toBe("t1");
    });

    it("j moves focus down within a column", () => {
      const columns = [col("todo", "t1", "t2")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: columns });
      vimState.handleKeydown(key("j"), { ...BASE_OPTS, boardColumns: columns });
      expect(vimState.focusedTaskId).toBe("t2");
    });

    it("k moves focus up within a column", () => {
      const columns = [col("todo", "t1", "t2")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: columns });
      vimState.handleKeydown(key("j"), { ...BASE_OPTS, boardColumns: columns });
      vimState.handleKeydown(key("k"), { ...BASE_OPTS, boardColumns: columns });
      expect(vimState.focusedTaskId).toBe("t1");
    });

    it("j at bottom of column stays on last task", () => {
      const columns = [col("todo", "t1")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: columns });
      vimState.handleKeydown(key("j"), { ...BASE_OPTS, boardColumns: columns });
      expect(vimState.focusedTaskId).toBe("t1");
    });

    it("k at top of column stays on first task", () => {
      const columns = [col("todo", "t1", "t2")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: columns });
      vimState.handleKeydown(key("k"), { ...BASE_OPTS, boardColumns: columns });
      expect(vimState.focusedTaskId).toBe("t1");
    });
  });

  // -------------------------------------------------------------------------
  // G (go-to-bottom)
  // -------------------------------------------------------------------------

  describe("G go-to-bottom", () => {
    it("G moves focus to last task in column", () => {
      const columns = [col("todo", "t1", "t2", "t3")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: columns });
      vimState.handleKeydown(key("G", { shiftKey: true }), { ...BASE_OPTS, boardColumns: columns });
      expect(vimState.focusedTaskId).toBe("t3");
    });
  });

  // -------------------------------------------------------------------------
  // setInitialFocus — Bug Round-2 #9
  // -------------------------------------------------------------------------

  describe("setInitialFocus — Bug Round-2 #9", () => {
    it("picks first task in first non-empty column", () => {
      const columns = [col("empty"), col("todo", "first", "second")];
      vimState.setInitialFocus(columns);
      expect(vimState.focusedTaskId).toBe("first");
    });

    it("sets null when all columns are empty", () => {
      vimState.setInitialFocus([col("empty")]);
      expect(vimState.focusedTaskId).toBeNull();
    });
  });

  // -------------------------------------------------------------------------
  // setWeekInitialFocus — Bug Round-2 #14
  // -------------------------------------------------------------------------

  describe("setWeekInitialFocus — Bug Round-2 #14", () => {
    it("picks today's task when today has tasks", () => {
      const today = new Date().toISOString().slice(0, 10);
      const days = [
        wday("2020-01-01", "old-task"),
        wday(today, "today-task1", "today-task2"),
        wday("2099-01-01", "future-task"),
      ];
      vimState.setWeekInitialFocus(days);
      expect(vimState.weekFocusedTaskId).toBe("today-task1");
      expect(vimState.weekFocusedDate).toBe(today);
    });

    it("falls back to first chronological task when today has no tasks", () => {
      const days = [wday("2020-01-01", "first-task"), wday("2099-01-01", "future-task")];
      vimState.setWeekInitialFocus(days);
      expect(vimState.weekFocusedTaskId).toBe("first-task");
    });

    it("does nothing when weekDays is empty", () => {
      vimState.setWeekInitialFocus([]);
      expect(vimState.weekFocusedTaskId).toBeNull();
    });

    it("skips empty days when falling back to first task", () => {
      const today = new Date().toISOString().slice(0, 10);
      const days = [wday(today), wday("2099-01-01", "future-task")]; // today has no tasks
      vimState.setWeekInitialFocus(days);
      expect(vimState.weekFocusedTaskId).toBe("future-task");
    });
  });

  // -------------------------------------------------------------------------
  // ] / [ period navigation — Bug Round-2 #15
  // -------------------------------------------------------------------------

  describe("] and [ dispatch period navigation events — Bug Round-2 #15", () => {
    it("] dispatches vim:next-period in week view", () => {
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: [] });
      vimState.handleKeydown(key("]"), { ...BASE_OPTS, activeView: "week", boardColumns: [], weekDays: [] });
      const evt = capturedEvents.find((e) => e.type === "vim:next-period");
      expect(evt).toBeDefined();
    });

    it("[ dispatches vim:prev-period in calendar view", () => {
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: [] });
      vimState.handleKeydown(key("["), { ...BASE_OPTS, activeView: "calendar", boardColumns: [], weekDays: [] });
      const evt = capturedEvents.find((e) => e.type === "vim:prev-period");
      expect(evt).toBeDefined();
    });

    it("] dispatches vim:next-period in calendar view", () => {
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: [] });
      vimState.handleKeydown(key("]"), { ...BASE_OPTS, activeView: "calendar", boardColumns: [], weekDays: [] });
      const evt = capturedEvents.find((e) => e.type === "vim:next-period");
      expect(evt).toBeDefined();
    });
  });

  // -------------------------------------------------------------------------
  // Calendar h/l day navigation — Bug Round-4 (current fix)
  // Fake system time so setWeekInitialFocus predictably picks the "today" entry.
  // -------------------------------------------------------------------------

  describe("calendar h/l day navigation (current fix)", () => {
    it("h in calendar normal mode moves to previous day", () => {
      vi.setSystemTime(new Date("2026-06-15"));
      // today=June 15; setWeekInitialFocus will pick t2
      const days = [wday("2026-06-01", "t1"), wday("2026-06-15", "t2"), wday("2026-06-20", "t3")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: [] });
      vimState.setWeekInitialFocus(days);
      expect(vimState.weekFocusedTaskId).toBe("t2");

      const consumed = vimState.handleKeydown(key("h"), {
        ...BASE_OPTS, activeView: "calendar", boardColumns: [], weekDays: days,
      });
      expect(consumed).toBe(true);
      expect(vimState.weekFocusedDate).toBe("2026-06-01");
      expect(vimState.weekFocusedTaskId).toBe("t1");
      vi.useRealTimers();
    });

    it("l in calendar normal mode moves to next day", () => {
      vi.setSystemTime(new Date("2026-06-15"));
      const days = [wday("2026-06-01", "t1"), wday("2026-06-15", "t2"), wday("2026-06-20", "t3")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: [] });
      vimState.setWeekInitialFocus(days);

      vimState.handleKeydown(key("l"), {
        ...BASE_OPTS, activeView: "calendar", boardColumns: [], weekDays: days,
      });
      expect(vimState.weekFocusedDate).toBe("2026-06-20");
      expect(vimState.weekFocusedTaskId).toBe("t3");
      vi.useRealTimers();
    });

    it("h at first day dispatches vim:prev-period (wraps to prev month like week view)", () => {
      vi.setSystemTime(new Date("2026-06-01"));
      const days = [wday("2026-06-01", "t1"), wday("2026-06-15", "t2")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: [] });
      vimState.setWeekInitialFocus(days); // picks June 1 (today)
      capturedEvents = [];

      vimState.handleKeydown(key("h"), {
        ...BASE_OPTS, activeView: "calendar", boardColumns: [], weekDays: days,
      });
      expect(capturedEvents.some((e) => e.type === "vim:prev-period")).toBe(true);
      vi.useRealTimers();
    });

    it("l at last day dispatches vim:next-period (wraps to next month like week view)", () => {
      vi.setSystemTime(new Date("2026-06-15"));
      const days = [wday("2026-06-01", "t1"), wday("2026-06-15", "t2")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: [] });
      vimState.setWeekInitialFocus(days); // picks June 15 (today)
      capturedEvents = [];

      vimState.handleKeydown(key("l"), {
        ...BASE_OPTS, activeView: "calendar", boardColumns: [], weekDays: days,
      });
      expect(capturedEvents.some((e) => e.type === "vim:next-period")).toBe(true);
      vi.useRealTimers();
    });

    it("calendar visual mode: h/l move selected tasks to adjacent day", () => {
      vi.setSystemTime(new Date("2026-06-01"));
      const onMoveTasksToAdjacentDay = vi.fn();
      const days = [wday("2026-06-01", "t1"), wday("2026-06-15", "t2")];

      // Activate vim and focus t1 in calendar view
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: [] });
      vimState.setWeekInitialFocus(days); // picks June 1 / t1
      // Enter visual mode via v
      vimState.handleKeydown(key("v"), { ...BASE_OPTS, activeView: "calendar", boardColumns: [], weekDays: days });
      expect(vimState.mode).toBe("visual");

      const consumed = vimState.handleKeydown(key("l"), {
        ...BASE_OPTS, activeView: "calendar", boardColumns: [], weekDays: days, onMoveTasksToAdjacentDay,
      });
      expect(consumed).toBe(true);
      expect(onMoveTasksToAdjacentDay).toHaveBeenCalledWith(["t1"], "right");
      vi.useRealTimers();
    });
  });

  // -------------------------------------------------------------------------
  // Week view h/l — day navigation with edge-wrap (Bug Round-2 #16)
  // -------------------------------------------------------------------------

  describe("week h/l — day navigation and edge-wrap", () => {
    it("l moves to next day in week view", () => {
      vi.setSystemTime(new Date("2026-06-01"));
      const days = [wday("2026-06-01", "t1"), wday("2026-06-02", "t2")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: [] });
      vimState.setWeekInitialFocus(days); // picks June 1

      vimState.handleKeydown(key("l"), { ...BASE_OPTS, activeView: "week", boardColumns: [], weekDays: days });
      expect(vimState.weekFocusedDate).toBe("2026-06-02");
      expect(vimState.weekFocusedTaskId).toBe("t2");
      vi.useRealTimers();
    });

    it("l at last day of week dispatches vim:next-period (edge-wrap)", () => {
      vi.setSystemTime(new Date("2026-06-01"));
      const days = [wday("2026-06-01", "t1")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: [] });
      vimState.setWeekInitialFocus(days);
      capturedEvents = [];

      vimState.handleKeydown(key("l"), { ...BASE_OPTS, activeView: "week", boardColumns: [], weekDays: days });
      expect(capturedEvents.some((e) => e.type === "vim:next-period")).toBe(true);
      vi.useRealTimers();
    });

    it("h moves to previous day in week view", () => {
      vi.setSystemTime(new Date("2026-06-02"));
      const days = [wday("2026-06-01", "t1"), wday("2026-06-02", "t2")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: [] });
      vimState.setWeekInitialFocus(days); // picks June 2 (today)

      vimState.handleKeydown(key("h"), { ...BASE_OPTS, activeView: "week", boardColumns: [], weekDays: days });
      expect(vimState.weekFocusedDate).toBe("2026-06-01");
      vi.useRealTimers();
    });
  });

  // -------------------------------------------------------------------------
  // Tab switching via ArrowRight / ArrowLeft
  // -------------------------------------------------------------------------

  describe("ArrowRight / ArrowLeft dispatch vim:set-tab", () => {
    it("ArrowRight dispatches vim:set-tab with next tab", () => {
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: [] });
      capturedEvents = [];
      vimState.handleKeydown(key("ArrowRight"), { ...BASE_OPTS, activeView: "board", boardColumns: [] });
      const evt = capturedEvents.find((e) => e.type === "vim:set-tab") as FakeCustomEvent | undefined;
      expect(evt).toBeDefined();
      expect(evt?.detail).toBe("week");
    });

    it("ArrowLeft wraps from board to dashboard", () => {
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: [] });
      capturedEvents = [];
      vimState.handleKeydown(key("ArrowLeft"), { ...BASE_OPTS, activeView: "board", boardColumns: [] });
      const evt = capturedEvents.find((e) => e.type === "vim:set-tab") as FakeCustomEvent | undefined;
      expect(evt?.detail).toBe("dashboard");
    });
  });

  // -------------------------------------------------------------------------
  // Suspended state
  // -------------------------------------------------------------------------

  describe("suspended state", () => {
    it("keys return false when vim is suspended", () => {
      const columns = [col("todo", "t1", "t2")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: columns });
      vimState.suspend();
      const consumed = vimState.handleKeydown(key("j"), { ...BASE_OPTS, boardColumns: columns });
      expect(consumed).toBe(false);
      expect(vimState.focusedTaskId).toBe("t1");
    });

    it("keys work again after resume", () => {
      const columns = [col("todo", "t1", "t2")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: columns });
      vimState.suspend();
      vimState.resume();
      const consumed = vimState.handleKeydown(key("j"), { ...BASE_OPTS, boardColumns: columns });
      expect(consumed).toBe(true);
      expect(vimState.focusedTaskId).toBe("t2");
    });
  });

  // -------------------------------------------------------------------------
  // e — edit task
  // -------------------------------------------------------------------------

  describe("e — edit task", () => {
    it("e calls onEditTask with focused task id in board view", () => {
      const onEditTask = vi.fn();
      const columns = [col("todo", "t1")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: columns });
      vimState.handleKeydown(key("e"), { ...BASE_OPTS, boardColumns: columns, onEditTask });
      expect(onEditTask).toHaveBeenCalledWith("t1");
    });

    it("e calls onEditTask with weekFocusedTaskId in week view", () => {
      vi.setSystemTime(new Date("2026-06-01"));
      const onEditTask = vi.fn();
      const days = [wday("2026-06-01", "w1")];
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: [] });
      vimState.setWeekInitialFocus(days); // focuses w1
      expect(vimState.weekFocusedTaskId).toBe("w1");
      vimState.handleKeydown(key("e"), {
        ...BASE_OPTS, activeView: "week", boardColumns: [], weekDays: days, onEditTask,
      });
      expect(onEditTask).toHaveBeenCalledWith("w1");
      vi.useRealTimers();
    });

    it("e does nothing when no task is focused", () => {
      const onEditTask = vi.fn();
      vimState.handleKeydown(key("Escape"), { ...BASE_OPTS, boardColumns: [] });
      vimState.handleKeydown(key("e"), { ...BASE_OPTS, boardColumns: [], onEditTask });
      expect(onEditTask).not.toHaveBeenCalled();
    });
  });
});
