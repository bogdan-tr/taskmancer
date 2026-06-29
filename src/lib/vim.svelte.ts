import { goto } from "$app/navigation";
import type { VimKeybinding } from "$lib/types";
import type { BoardColumn } from "$lib/kanbanGrouping";
import type { StatusDefinition } from "$lib/types";
import {
  parseKeyCombo,
  matchesCombo,
  matchesAction,
  DEFAULT_STATUS_SHORTCUTS,
} from "$lib/vimKeybindings";

export type VimMode = "normal" | "visual" | "visual_sparse";

interface VimState {
  active: boolean;
  mode: VimMode;
  focusedTaskId: string | null;
  selectedTaskIds: Set<string>;
  /** ID of the task where visual selection started (anchor for range calculation). */
  visualAnchorId: string | null;
  suspended: boolean;
  sequenceBuffer: string;
  sequenceTimer: ReturnType<typeof setTimeout> | null;
  weekFocusedDate: string | null;
  /** Shared focused task ID for week view AND calendar view (mutually exclusive views). */
  weekFocusedTaskId: string | null;
  sidebarCursorRoute: string | null;
}

function createVimState() {
  let state = $state<VimState>({
    active: false,
    mode: "normal",
    focusedTaskId: null,
    selectedTaskIds: new Set(),
    visualAnchorId: null,
    suspended: false,
    sequenceBuffer: "",
    sequenceTimer: null,
    weekFocusedDate: null,
    weekFocusedTaskId: null,
    sidebarCursorRoute: null,
  });

  function resetSequence() {
    if (state.sequenceTimer !== null) {
      clearTimeout(state.sequenceTimer);
      state.sequenceTimer = null;
    }
    state.sequenceBuffer = "";
  }

  function activate() {
    state.active = true;
    state.mode = "normal";
    state.selectedTaskIds = new Set();
    state.visualAnchorId = null;
    state.sequenceBuffer = "";
  }

  function deactivate() {
    state.active = false;
    state.mode = "normal";
    state.focusedTaskId = null;
    state.selectedTaskIds = new Set();
    state.visualAnchorId = null;
    state.weekFocusedDate = null;
    state.weekFocusedTaskId = null;
    state.sidebarCursorRoute = null;
    resetSequence();
  }

  function suspend() {
    state.suspended = true;
  }

  function resume() {
    state.suspended = false;
  }

  function flatColumnTasks(
    boardColumns: BoardColumn[],
  ): Array<{ colIdx: number; tasks: string[] }> {
    return boardColumns.map((col, colIdx) => ({
      colIdx,
      tasks: col.buckets.flatMap((b) => b.tasks.map((t) => t.id)),
    }));
  }

  function findFocusPosition(
    boardColumns: BoardColumn[],
    taskId: string | null,
  ): { ci: number; ti: number } | null {
    if (!taskId) return null;
    const cols = flatColumnTasks(boardColumns);
    for (let ci = 0; ci < cols.length; ci++) {
      const ti = cols[ci].tasks.indexOf(taskId);
      if (ti !== -1) return { ci, ti };
    }
    return null;
  }

  function setInitialFocus(boardColumns: BoardColumn[]) {
    const cols = flatColumnTasks(boardColumns);
    for (const col of cols) {
      if (col.tasks.length > 0) {
        state.focusedTaskId = col.tasks[0];
        return;
      }
    }
    state.focusedTaskId = null;
  }

  /** Auto-select the first task in week/calendar view when vim activates or view switches. */
  function setWeekInitialFocus(weekDays: Array<{ date: string; taskIds: string[] }>) {
    const todayStr = new Date().toISOString().slice(0, 10);
    const todayDay = weekDays.find((d) => d.date === todayStr && d.taskIds.length > 0);
    const firstWithTasks = weekDays.find((d) => d.taskIds.length > 0);
    const target = todayDay ?? firstWithTasks;
    if (target) {
      state.weekFocusedDate = target.date;
      state.weekFocusedTaskId = target.taskIds[0];
    }
  }

  /**
   * Recalculate the visual selection as the contiguous range between the anchor
   * and the current task within the same column task list.
   */
  function recalcVisualRange(tasks: string[], currentId: string) {
    const anchorIdx = tasks.indexOf(state.visualAnchorId ?? "");
    const currentIdx = tasks.indexOf(currentId);
    if (anchorIdx === -1 || currentIdx === -1) {
      state.selectedTaskIds = new Set([currentId]);
      return;
    }
    const minIdx = Math.min(anchorIdx, currentIdx);
    const maxIdx = Math.max(anchorIdx, currentIdx);
    state.selectedTaskIds = new Set(tasks.slice(minIdx, maxIdx + 1));
  }

  function handleKeydown(
    event: KeyboardEvent,
    opts: {
      boardColumns: BoardColumn[];
      activeView: string;
      vimEnabled: boolean;
      userBindings: VimKeybinding[];
      statusBindings: VimKeybinding[];
      statuses: StatusDefinition[];
      sidebarItems: Array<{ route: string }>;
      currentPageRoute: string;
      currentProjectId: string | null;
      editDialogOpen?: boolean;
      onEditTask: (id: string) => void;
      onToggleTimer: (id: string) => void;
      onNewTask: () => void;
      onDeleteTask: (id: string) => void;
      onChangeStatus: (ids: string[], statusId: string) => void;
      onMoveTasksToAdjacentStatus: (taskIds: string[], direction: "left" | "right") => void;
      onMoveTasksToAdjacentDay?: (taskIds: string[], direction: "left" | "right") => void;
      onToggleSidebarExpand?: (projectId: string) => void;
      weekDays?: Array<{ date: string; taskIds: string[] }>;
    },
  ): boolean {
    if (!opts.vimEnabled) return false;

    const combo = parseKeyCombo(event);

    // Escape is special — always handle it (not affected by suspension)
    if (combo === "Escape") {
      // When the edit dialog is open, let the event reach the dialog's native handler
      if (opts.editDialogOpen) return false;

      if (!state.active) {
        activate();
        if (!state.focusedTaskId) setInitialFocus(opts.boardColumns);
        return true;
      }
      if (state.sidebarCursorRoute !== null) {
        state.sidebarCursorRoute = null;
        return true;
      }
      if (state.mode !== "normal") {
        state.mode = "normal";
        state.selectedTaskIds = new Set();
        state.visualAnchorId = null;
        resetSequence();
        return true;
      }
      deactivate();
      return true;
    }

    if (!state.active || state.suspended) return false;
    // When any edit dialog is open, suppress all vim keys (Escape is handled above)
    if (opts.editDialogOpen) return false;

    const cols = flatColumnTasks(opts.boardColumns);
    const pos = findFocusPosition(opts.boardColumns, state.focusedTaskId);

    // --- Tab switching ---
    if (matchesAction("next_tab", event, opts.userBindings)) {
      const tabs: string[] = ["board", "week", "calendar", "dashboard"];
      const idx = tabs.indexOf(opts.activeView);
      if (idx !== -1) {
        document.dispatchEvent(new CustomEvent("vim:set-tab", { detail: tabs[(idx + 1) % tabs.length] }));
        return true;
      }
    }
    if (matchesAction("prev_tab", event, opts.userBindings)) {
      const tabs: string[] = ["board", "week", "calendar", "dashboard"];
      const idx = tabs.indexOf(opts.activeView);
      if (idx !== -1) {
        document.dispatchEvent(new CustomEvent("vim:set-tab", { detail: tabs[(idx - 1 + tabs.length) % tabs.length] }));
        return true;
      }
    }

    // --- Project switching (sidebar cursor) ---
    const isNextProject = matchesAction("next_project", event, opts.userBindings);
    const isPrevProject = matchesAction("prev_project", event, opts.userBindings);

    if (isNextProject || isPrevProject) {
      const routes = opts.sidebarItems.map((i) => i.route);
      const isDown = isNextProject;
      const currentCursor = state.sidebarCursorRoute ?? opts.currentPageRoute;
      const idx = routes.indexOf(currentCursor);
      const baseIdx = idx === -1 ? 0 : idx;
      const nextIdx = isDown
        ? (baseIdx + 1) % routes.length
        : (baseIdx - 1 + routes.length) % routes.length;
      state.sidebarCursorRoute = routes[nextIdx];
      return true;
    }

    // --- Confirm (Enter): navigate sidebar cursor OR edit focused task ---
    if (matchesAction("confirm", event, opts.userBindings)) {
      if (state.sidebarCursorRoute !== null) {
        void goto(state.sidebarCursorRoute);
        state.sidebarCursorRoute = null;
        return true;
      }
      const focusedId = opts.activeView === "week" || opts.activeView === "calendar"
        ? state.weekFocusedTaskId
        : state.focusedTaskId;
      if (state.mode === "normal" && focusedId) {
        opts.onEditTask(focusedId);
        return true;
      }
    }

    // --- Toggle (Space): expand/collapse sidebar project OR toggle task in visual_sparse ---
    if (matchesAction("toggle", event, opts.userBindings)) {
      if (state.sidebarCursorRoute !== null) {
        // Extract project ID from "/projects/<id>" route
        const match = state.sidebarCursorRoute.match(/^\/projects\/(.+)$/);
        if (match) {
          opts.onToggleSidebarExpand?.(match[1]);
          return true;
        }
      }
      if (state.mode === "visual_sparse") {
        const id = opts.activeView === "week" || opts.activeView === "calendar"
          ? state.weekFocusedTaskId
          : state.focusedTaskId;
        if (id) {
          const next = new Set(state.selectedTaskIds);
          if (next.has(id)) {
            next.delete(id);
          } else {
            next.add(id);
          }
          state.selectedTaskIds = next;
        }
        return true;
      }
      return true; // consume space to prevent page scroll in all modes
    }

    // --- Visual mode entry: shared across all views ---
    if (combo === "v") {
      if (state.sequenceBuffer === "v") {
        // vv → sparse-visual
        resetSequence();
        state.mode = "visual_sparse";
        state.visualAnchorId = null;
        if (state.selectedTaskIds.size === 0) {
          const focused = opts.activeView === "week" || opts.activeView === "calendar"
            ? state.weekFocusedTaskId
            : state.focusedTaskId;
          if (focused) state.selectedTaskIds = new Set([focused]);
        }
        return true;
      } else {
        // First v → visual mode immediately; buffer for potential vv
        resetSequence();
        const focused = opts.activeView === "week" || opts.activeView === "calendar"
          ? state.weekFocusedTaskId
          : state.focusedTaskId;
        state.mode = "visual";
        state.visualAnchorId = focused;
        state.selectedTaskIds = focused ? new Set([focused]) : new Set();
        state.sequenceBuffer = "v";
        state.sequenceTimer = setTimeout(() => {
          state.sequenceBuffer = "";
          state.sequenceTimer = null;
        }, 800);
        return true;
      }
    }

    // --- Edit task: shared across board, week, and calendar ---
    if (matchesAction("edit_task", event, opts.userBindings) && state.mode === "normal") {
      const focusedId = opts.activeView === "week" || opts.activeView === "calendar"
        ? state.weekFocusedTaskId
        : state.focusedTaskId;
      if (focusedId) {
        opts.onEditTask(focusedId);
        return true;
      }
    }

    // --- Board-view-only navigation ---
    if (opts.activeView === "board") {
      // "g" prefix for "gg"
      if (combo === "g") {
        if (state.sequenceBuffer === "g") {
          resetSequence();
          if (!pos) {
            const firstCol = cols.find((c) => c.tasks.length > 0);
            if (firstCol) {
              state.focusedTaskId = firstCol.tasks[0];
              if (state.mode === "visual") recalcVisualRange(firstCol.tasks, firstCol.tasks[0]);
            }
          } else {
            const col = cols[pos.ci];
            if (col.tasks.length > 0) {
              const top = col.tasks[0];
              state.focusedTaskId = top;
              if (state.mode === "visual") recalcVisualRange(col.tasks, top);
            }
          }
          return true;
        } else {
          resetSequence();
          state.sequenceBuffer = "g";
          state.sequenceTimer = setTimeout(() => resetSequence(), 800);
          return true;
        }
      }

      // Clear g-sequence buffer on other keys (v buffer managed in shared section)
      if (state.sequenceBuffer === "g") resetSequence();

      // G — go to bottom
      if (matchesAction("go_to_bottom", event, opts.userBindings)) {
        if (!pos) {
          const firstCol = cols.find((c) => c.tasks.length > 0);
          if (firstCol) {
            const last = firstCol.tasks[firstCol.tasks.length - 1];
            state.focusedTaskId = last;
            if (state.mode === "visual") recalcVisualRange(firstCol.tasks, last);
          }
        } else {
          const col = cols[pos.ci];
          const last = col.tasks[col.tasks.length - 1] ?? state.focusedTaskId;
          state.focusedTaskId = last;
          if (last && state.mode === "visual") recalcVisualRange(col.tasks, last);
        }
        return true;
      }

      // j — move down
      if (matchesAction("move_down", event, opts.userBindings)) {
        if (pos) {
          const col = cols[pos.ci];
          const next = col.tasks[pos.ti + 1];
          if (next) {
            state.focusedTaskId = next;
            if (state.mode === "visual") recalcVisualRange(col.tasks, next);
          }
        }
        return true;
      }

      // k — move up
      if (matchesAction("move_up", event, opts.userBindings)) {
        if (pos) {
          const col = cols[pos.ci];
          const prev = col.tasks[pos.ti - 1];
          if (prev) {
            state.focusedTaskId = prev;
            if (state.mode === "visual") recalcVisualRange(col.tasks, prev);
          }
        }
        return true;
      }

      // h — move left
      if (matchesAction("move_left", event, opts.userBindings)) {
        if (state.mode === "normal") {
          if (!pos) {
            const firstNonEmpty = cols.find((c) => c.tasks.length > 0);
            if (firstNonEmpty) state.focusedTaskId = firstNonEmpty.tasks[0];
          } else if (pos.ci > 0) {
            let targetCi = pos.ci - 1;
            while (targetCi > 0 && cols[targetCi].tasks.length === 0) targetCi--;
            const prevCol = cols[targetCi];
            if (prevCol.tasks.length > 0) {
              const target = prevCol.tasks[Math.min(pos.ti, prevCol.tasks.length - 1)];
              if (target) state.focusedTaskId = target;
            }
          }
        } else if ((state.mode === "visual" || state.mode === "visual_sparse") && state.selectedTaskIds.size > 0) {
          opts.onMoveTasksToAdjacentStatus([...state.selectedTaskIds], "left");
        }
        return true;
      }

      // l — move right
      if (matchesAction("move_right", event, opts.userBindings)) {
        if (state.mode === "normal") {
          if (!pos) {
            const firstNonEmpty = cols.find((c) => c.tasks.length > 0);
            if (firstNonEmpty) state.focusedTaskId = firstNonEmpty.tasks[0];
          } else if (pos.ci < cols.length - 1) {
            let targetCi = pos.ci + 1;
            while (targetCi < cols.length - 1 && cols[targetCi].tasks.length === 0) targetCi++;
            const nextCol = cols[targetCi];
            if (nextCol.tasks.length > 0) {
              const target = nextCol.tasks[Math.min(pos.ti, nextCol.tasks.length - 1)];
              if (target) state.focusedTaskId = target;
            }
          }
        } else if ((state.mode === "visual" || state.mode === "visual_sparse") && state.selectedTaskIds.size > 0) {
          opts.onMoveTasksToAdjacentStatus([...state.selectedTaskIds], "right");
        }
        return true;
      }

      // --- Actions (normal mode only) ---
      if (state.mode === "normal" && state.focusedTaskId) {
        const focused = state.focusedTaskId;

        if (matchesAction("toggle_timer", event, opts.userBindings)) {
          opts.onToggleTimer(focused);
          return true;
        }

        if (matchesAction("new_task", event, opts.userBindings)) {
          opts.onNewTask();
          return true;
        }

        if (matchesAction("delete_task", event, opts.userBindings)) {
          opts.onDeleteTask(focused);
          return true;
        }
      }

      // --- Status shortcuts (normal mode single task + visual/sparse batch) ---
      const targetIds: string[] =
        state.mode === "normal" && state.focusedTaskId
          ? [state.focusedTaskId]
          : [...state.selectedTaskIds];

      if (targetIds.length > 0) {
        for (const binding of opts.statusBindings) {
          if (binding.combos.some((c) => matchesCombo(event, c))) {
            opts.onChangeStatus(targetIds, binding.action_id);
            if (state.mode !== "normal") {
              state.mode = "normal";
              state.selectedTaskIds = new Set();
              state.visualAnchorId = null;
            }
            return true;
          }
        }
        for (const [label, defaultCombo] of Object.entries(DEFAULT_STATUS_SHORTCUTS)) {
          if (matchesCombo(event, defaultCombo)) {
            const status = opts.statuses.find((s) => s.label.toLowerCase() === label);
            if (status) {
              opts.onChangeStatus(targetIds, status.id);
              if (state.mode !== "normal") {
                state.mode = "normal";
                state.selectedTaskIds = new Set();
                state.visualAnchorId = null;
              }
              return true;
            }
          }
        }
      }
    } // end board-only

    // --- Week & calendar navigation ---
    if (opts.activeView === "week" || opts.activeView === "calendar") {
      const days = opts.weekDays ?? [];
      const todayStr = new Date().toISOString().slice(0, 10);

      const isLeft = matchesAction("move_left", event, opts.userBindings);
      const isRight = matchesAction("move_right", event, opts.userBindings);

      if (isLeft || isRight) {
        // Visual mode: move selected tasks one day left/right (both week and calendar)
        if ((state.mode === "visual" || state.mode === "visual_sparse") && state.selectedTaskIds.size > 0) {
          opts.onMoveTasksToAdjacentDay?.([...state.selectedTaskIds], isLeft ? "left" : "right");
          return true;
        }

        // Normal mode h/l: navigate between days with edge-wrap to prev/next period.
        // opts.weekDays is pre-filtered to only the dates visible in the current period
        // (week or month), so hitting the boundary dispatches vim:prev/next-period and
        // WeekView/CalendarView advance the period — the onDateStringsChange callback then
        // fires and auto-focus re-places the cursor on the first task in the new period.
        const focused = state.weekFocusedDate ?? todayStr;
        const idx = days.findIndex((d) => d.date === focused);
        const baseIdx = idx === -1 ? days.findIndex((d) => d.date === todayStr) : idx;

        if (baseIdx !== -1) {
          const nextIdx = isLeft ? baseIdx - 1 : baseIdx + 1;
          if (nextIdx >= 0 && nextIdx < days.length) {
            state.weekFocusedDate = days[nextIdx].date;
            state.weekFocusedTaskId = days[nextIdx].taskIds[0] ?? null;
          } else {
            document.dispatchEvent(new CustomEvent(isLeft ? "vim:prev-period" : "vim:next-period"));
          }
        } else if (days.length > 0) {
          state.weekFocusedDate = days[0].date;
          state.weekFocusedTaskId = days[0].taskIds[0] ?? null;
        }
        return true;
      }

      const isDown = matchesAction("move_down", event, opts.userBindings);
      const isUp = matchesAction("move_up", event, opts.userBindings);

      if (isDown || isUp) {
        const focused = state.weekFocusedDate ?? todayStr;
        const day =
          days.find((d) => d.date === focused) ??
          days.find((d) => d.date === todayStr) ??
          days[0];
        if (day) {
          state.weekFocusedDate = day.date;
          const ti = day.taskIds.indexOf(state.weekFocusedTaskId ?? "");
          if (ti === -1) {
            state.weekFocusedTaskId = day.taskIds[0] ?? null;
          } else {
            const nextTi = isDown ? ti + 1 : ti - 1;
            if (nextTi >= 0 && nextTi < day.taskIds.length) {
              state.weekFocusedTaskId = day.taskIds[nextTi];
            }
          }
          if (state.mode === "visual" && state.weekFocusedTaskId) {
            state.selectedTaskIds = new Set([...state.selectedTaskIds, state.weekFocusedTaskId]);
          }
        }
        return true;
      }

      // Next/prev period (next week in week view, next month in calendar view)
      if (matchesAction("next_period", event, opts.userBindings)) {
        document.dispatchEvent(new CustomEvent("vim:next-period"));
        return true;
      }
      if (matchesAction("prev_period", event, opts.userBindings)) {
        document.dispatchEvent(new CustomEvent("vim:prev-period"));
        return true;
      }
    }

    return false;
  }

  return {
    get active() { return state.active; },
    get mode() { return state.mode; },
    get focusedTaskId() { return state.focusedTaskId; },
    get selectedTaskIds() { return state.selectedTaskIds; },
    get suspended() { return state.suspended; },
    get weekFocusedDate() { return state.weekFocusedDate; },
    get weekFocusedTaskId() { return state.weekFocusedTaskId; },
    get sidebarCursorRoute() { return state.sidebarCursorRoute; },
    activate,
    deactivate,
    suspend,
    resume,
    handleKeydown,
    setInitialFocus,
    setWeekInitialFocus,
  };
}

export const vimState = createVimState();
