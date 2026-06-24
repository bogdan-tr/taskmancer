import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Settings, StatusDefinition, Task, TimeEntry } from "./types";

function makeEntry(overrides: Partial<TimeEntry> = {}): TimeEntry {
  return {
    id: "entry-1",
    task_id: "task-1",
    started_at: "2026-06-15T09:00:00+00:00",
    ended_at: null,
    last_heartbeat_at: null,
    created_at: "2026-06-15T09:00:00+00:00",
    ...overrides,
  };
}

const SEEDED_STATUSES: StatusDefinition[] = [
  { id: "backlog", label: "Backlog", order: 1, color: "#6f7178" },
  { id: "do", label: "Do", order: 2, color: "#0073b6" },
  { id: "in-progress", label: "In Progress", order: 3, color: "#bd7d00" },
  { id: "blocked", label: "Blocked", order: 4, color: "#bc267f" },
  { id: "done", label: "Done", order: 5, color: "#0e9254" },
];

function makeSettings(overrides: Partial<Settings> = {}): Settings {
  return {
    priorities: [],
    statuses: SEEDED_STATUSES,
    defaults: { tags: [] },
    done_status: "done",
    cancelled_status: undefined,
    default_project_id: "general-id",
    show_previous_weeks_column: false,
    card_lightness: 0.5,
    bar_lightness: 0.38,
    ink_mode: "auto",
    show_subproject_tasks_default: false,
    parent_estimate_includes_own_value: false,
    max_visible_subtasks: 5,
    tracking_auto_transition_enabled: false,
    tracking_auto_transition_status_id: undefined,
    ...overrides,
  };
}

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: "task-1",
    title: "Sample task",
    status: "backlog",
    tags: [],
    priority: "medium",
    order: 0,
    created: "2026-06-15T09:00:00+00:00",
    depends_on: [],
    tracked_minutes: 0,
    hidden: false,
    notes: "",
    ...overrides,
  };
}

vi.mock("./api", () => ({
  getActiveSessions: vi.fn(),
  heartbeat: vi.fn(),
  startTracking: vi.fn(),
  stopTracking: vi.fn(),
  startProjectTracking: vi.fn(),
  stopProjectTracking: vi.fn(),
  updateTask: vi.fn(),
}));

vi.mock("./tasks.svelte", () => ({
  refreshTasks: vi.fn(),
  tasksState: { items: [] },
  upsertCachedTask: vi.fn(),
}));

vi.mock("./settings.svelte", () => ({
  settingsState: { current: undefined },
}));

describe("tracking.svelte", () => {
  afterEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
    vi.useRealTimers();
  });

  describe("refreshActiveSessions", () => {
    it("populates state with the full active-sessions list", async () => {
      const { getActiveSessions } = await import("./api");
      const entries = [makeEntry({ id: "a" }), makeEntry({ id: "b", task_id: "task-2" })];
      vi.mocked(getActiveSessions).mockResolvedValue(entries);
      const { trackingState, refreshActiveSessions } = await import("./tracking.svelte");

      await refreshActiveSessions();

      expect(trackingState.activeSessions).toEqual(entries);
    });

    it("preserves the prior list when the request fails", async () => {
      const { getActiveSessions } = await import("./api");
      vi.mocked(getActiveSessions)
        .mockResolvedValueOnce([makeEntry({ id: "a" })])
        .mockRejectedValueOnce(new Error("network error"));
      const { trackingState, refreshActiveSessions } = await import("./tracking.svelte");

      await refreshActiveSessions();
      expect(trackingState.activeSessions.map((e) => e.id)).toEqual(["a"]);

      await expect(refreshActiveSessions()).resolves.toBeUndefined();
      expect(trackingState.activeSessions.map((e) => e.id)).toEqual(["a"]);
    });
  });

  describe("isTaskActive", () => {
    it("returns true when the task has an active session", async () => {
      const { trackingState, isTaskActive } = await import("./tracking.svelte");
      trackingState.activeSessions = [makeEntry({ task_id: "task-1" })];

      expect(isTaskActive("task-1")).toBe(true);
    });

    it("returns false when the task has no active session", async () => {
      const { trackingState, isTaskActive } = await import("./tracking.svelte");
      trackingState.activeSessions = [makeEntry({ task_id: "task-1" })];

      expect(isTaskActive("task-2")).toBe(false);
    });

    it("returns false when there are no active sessions at all", async () => {
      const { trackingState, isTaskActive } = await import("./tracking.svelte");
      trackingState.activeSessions = [];

      expect(isTaskActive("task-1")).toBe(false);
    });
  });

  describe("activeSessionFor", () => {
    it("returns the matching entry for an active task", async () => {
      const { trackingState, activeSessionFor } = await import("./tracking.svelte");
      const entry = makeEntry({ task_id: "task-1" });
      trackingState.activeSessions = [entry, makeEntry({ id: "other", task_id: "task-2" })];

      expect(activeSessionFor("task-1")).toEqual(entry);
    });

    it("returns undefined when the task has no active session", async () => {
      const { trackingState, activeSessionFor } = await import("./tracking.svelte");
      trackingState.activeSessions = [makeEntry({ task_id: "task-1" })];

      expect(activeSessionFor("task-2")).toBeUndefined();
    });
  });

  describe("elapsedSecondsFor", () => {
    it("returns undefined when the task has no active session", async () => {
      const { trackingState, elapsedSecondsFor } = await import("./tracking.svelte");
      trackingState.activeSessions = [];

      expect(elapsedSecondsFor("task-1")).toBeUndefined();
    });

    it("computes elapsed seconds from started_at to the current nowMs", async () => {
      const { trackingState, elapsedSecondsFor } = await import("./tracking.svelte");
      trackingState.activeSessions = [
        makeEntry({ task_id: "task-1", started_at: "2026-06-15T09:00:00+00:00" }),
      ];
      trackingState.nowMs = Date.parse("2026-06-15T09:00:45+00:00");

      expect(elapsedSecondsFor("task-1")).toBe(45);
    });

    it("floors a fractional number of elapsed seconds", async () => {
      const { trackingState, elapsedSecondsFor } = await import("./tracking.svelte");
      trackingState.activeSessions = [
        makeEntry({ task_id: "task-1", started_at: "2026-06-15T09:00:00+00:00" }),
      ];
      trackingState.nowMs = Date.parse("2026-06-15T09:00:00+00:00") + 1500;

      expect(elapsedSecondsFor("task-1")).toBe(1);
    });

    it("clamps to zero rather than going negative on clock skew", async () => {
      const { trackingState, elapsedSecondsFor } = await import("./tracking.svelte");
      trackingState.activeSessions = [
        makeEntry({ task_id: "task-1", started_at: "2026-06-15T09:00:10+00:00" }),
      ];
      trackingState.nowMs = Date.parse("2026-06-15T09:00:00+00:00");

      expect(elapsedSecondsFor("task-1")).toBe(0);
    });
  });

  describe("liveTrackedSecondsFor", () => {
    it("returns undefined when the task has no active session", async () => {
      const { trackingState, liveTrackedSecondsFor } = await import("./tracking.svelte");
      trackingState.activeSessions = [];

      expect(liveTrackedSecondsFor(makeTask({ id: "task-1", tracked_minutes: 30 }))).toBeUndefined();
    });

    it("adds the task's already-tracked minutes (in seconds) to the current session's elapsed seconds", async () => {
      const { trackingState, liveTrackedSecondsFor } = await import("./tracking.svelte");
      trackingState.activeSessions = [
        makeEntry({ task_id: "task-1", started_at: "2026-06-15T09:00:00+00:00" }),
      ];
      trackingState.nowMs = Date.parse("2026-06-15T09:00:45+00:00");

      // 5 past completed minutes (300s) + 45s of the current, still-running session.
      expect(liveTrackedSecondsFor(makeTask({ id: "task-1", tracked_minutes: 5 }))).toBe(345);
    });

    it("resumes from the prior cumulative total rather than restarting from zero, across a pause/resume cycle", async () => {
      const { trackingState, liveTrackedSecondsFor } = await import("./tracking.svelte");

      // First session: 0 prior minutes, ran for 2 minutes (120s), now stopped
      // — `tracked_minutes` reflects that on the task once recomputed server-side.
      const taskAfterFirstSession = makeTask({ id: "task-1", tracked_minutes: 2 });

      // Resumed: a brand-new active session starts ticking from 0 elapsed again.
      trackingState.activeSessions = [
        makeEntry({ task_id: "task-1", started_at: "2026-06-15T10:00:00+00:00" }),
      ];
      trackingState.nowMs = Date.parse("2026-06-15T10:00:10+00:00");

      // Must read 2m10s (130s) total, not restart the display at 0:10.
      expect(liveTrackedSecondsFor(taskAfterFirstSession)).toBe(130);
    });

    it("treats zero prior tracked minutes as a pure session-elapsed read, matching elapsedSecondsFor", async () => {
      const { trackingState, liveTrackedSecondsFor, elapsedSecondsFor } = await import("./tracking.svelte");
      trackingState.activeSessions = [
        makeEntry({ task_id: "task-1", started_at: "2026-06-15T09:00:00+00:00" }),
      ];
      trackingState.nowMs = Date.parse("2026-06-15T09:00:20+00:00");

      expect(liveTrackedSecondsFor(makeTask({ id: "task-1", tracked_minutes: 0 }))).toBe(
        elapsedSecondsFor("task-1"),
      );
    });
  });

  describe("startTaskTracking", () => {
    it("calls startTracking then refreshes active sessions", async () => {
      const { startTracking, getActiveSessions } = await import("./api");
      vi.mocked(startTracking).mockResolvedValue(undefined);
      vi.mocked(getActiveSessions).mockResolvedValue([makeEntry({ task_id: "task-1" })]);
      const { trackingState, startTaskTracking } = await import("./tracking.svelte");

      await startTaskTracking("task-1");

      expect(startTracking).toHaveBeenCalledWith("task-1");
      expect(getActiveSessions).toHaveBeenCalled();
      expect(trackingState.activeSessions.map((e) => e.task_id)).toEqual(["task-1"]);
    });

    it("does not attempt an auto-transition when the setting is disabled", async () => {
      const { startTracking, getActiveSessions, updateTask } = await import("./api");
      vi.mocked(startTracking).mockResolvedValue(undefined);
      vi.mocked(getActiveSessions).mockResolvedValue([]);
      const { settingsState } = await import("./settings.svelte");
      settingsState.current = makeSettings({ tracking_auto_transition_enabled: false });
      const { tasksState } = await import("./tasks.svelte");
      tasksState.items = [makeTask({ id: "task-1", status: "backlog" })];
      const { startTaskTracking } = await import("./tracking.svelte");

      await startTaskTracking("task-1");

      expect(updateTask).not.toHaveBeenCalled();
    });

    it("auto-transitions the task to the resolved status when enabled and the task can be found", async () => {
      const { startTracking, getActiveSessions, updateTask } = await import("./api");
      vi.mocked(startTracking).mockResolvedValue(undefined);
      vi.mocked(getActiveSessions).mockResolvedValue([]);
      const { settingsState } = await import("./settings.svelte");
      settingsState.current = makeSettings({ tracking_auto_transition_enabled: true });
      const { tasksState, upsertCachedTask } = await import("./tasks.svelte");
      const task = makeTask({ id: "task-1", status: "backlog" });
      tasksState.items = [task];
      vi.mocked(updateTask).mockResolvedValue({ ...task, status: "do" });
      const { startTaskTracking } = await import("./tracking.svelte");

      await startTaskTracking("task-1");

      expect(updateTask).toHaveBeenCalledWith({ ...task, status: "do" });
      expect(upsertCachedTask).toHaveBeenCalledWith({ ...task, status: "do" });
    });

    it("skips the auto-transition when the task's status already matches the target", async () => {
      const { startTracking, getActiveSessions, updateTask } = await import("./api");
      vi.mocked(startTracking).mockResolvedValue(undefined);
      vi.mocked(getActiveSessions).mockResolvedValue([]);
      const { settingsState } = await import("./settings.svelte");
      settingsState.current = makeSettings({ tracking_auto_transition_enabled: true });
      const { tasksState } = await import("./tasks.svelte");
      tasksState.items = [makeTask({ id: "task-1", status: "do" })];
      const { startTaskTracking } = await import("./tracking.svelte");

      await startTaskTracking("task-1");

      expect(updateTask).not.toHaveBeenCalled();
    });

    it("skips the auto-transition when the task can't be found in the cache", async () => {
      const { startTracking, getActiveSessions, updateTask } = await import("./api");
      vi.mocked(startTracking).mockResolvedValue(undefined);
      vi.mocked(getActiveSessions).mockResolvedValue([]);
      const { settingsState } = await import("./settings.svelte");
      settingsState.current = makeSettings({ tracking_auto_transition_enabled: true });
      const { tasksState } = await import("./tasks.svelte");
      tasksState.items = [];
      const { startTaskTracking } = await import("./tracking.svelte");

      await startTaskTracking("task-1");

      expect(updateTask).not.toHaveBeenCalled();
    });

    it("does not let a failed auto-transition reject startTaskTracking itself", async () => {
      const { startTracking, getActiveSessions, updateTask } = await import("./api");
      vi.mocked(startTracking).mockResolvedValue(undefined);
      vi.mocked(getActiveSessions).mockResolvedValue([]);
      const { settingsState } = await import("./settings.svelte");
      settingsState.current = makeSettings({ tracking_auto_transition_enabled: true });
      const { tasksState } = await import("./tasks.svelte");
      tasksState.items = [makeTask({ id: "task-1", status: "backlog" })];
      vi.mocked(updateTask).mockRejectedValue(new Error("save failed"));
      const { startTaskTracking } = await import("./tracking.svelte");

      await expect(startTaskTracking("task-1")).resolves.toBeUndefined();
    });

    it("never auto-transitions when settingsState.current is undefined", async () => {
      const { startTracking, getActiveSessions, updateTask } = await import("./api");
      vi.mocked(startTracking).mockResolvedValue(undefined);
      vi.mocked(getActiveSessions).mockResolvedValue([]);
      const { settingsState } = await import("./settings.svelte");
      settingsState.current = undefined;
      const { tasksState } = await import("./tasks.svelte");
      tasksState.items = [makeTask({ id: "task-1", status: "backlog" })];
      const { startTaskTracking } = await import("./tracking.svelte");

      await startTaskTracking("task-1");

      expect(updateTask).not.toHaveBeenCalled();
    });
  });

  describe("stopTaskTracking", () => {
    it("calls stopTracking, refreshes active sessions, and returns the new tracked minutes", async () => {
      const { stopTracking, getActiveSessions } = await import("./api");
      vi.mocked(stopTracking).mockResolvedValue(42);
      vi.mocked(getActiveSessions).mockResolvedValue([]);
      const { stopTaskTracking } = await import("./tracking.svelte");

      const result = await stopTaskTracking("task-1");

      expect(stopTracking).toHaveBeenCalledWith("task-1");
      expect(getActiveSessions).toHaveBeenCalled();
      expect(result).toBe(42);
    });
  });

  describe("startProjectTracking", () => {
    it("calls startProjectTracking then refreshes active sessions", async () => {
      const { startProjectTracking: apiStartProjectTracking, getActiveSessions } = await import("./api");
      vi.mocked(apiStartProjectTracking).mockResolvedValue(undefined);
      vi.mocked(getActiveSessions).mockResolvedValue([makeEntry({ task_id: "hidden-task" })]);
      const { trackingState, startProjectTracking } = await import("./tracking.svelte");

      await startProjectTracking("project-1");

      expect(apiStartProjectTracking).toHaveBeenCalledWith("project-1");
      expect(trackingState.activeSessions.map((e) => e.task_id)).toEqual(["hidden-task"]);
    });
  });

  describe("stopProjectTracking", () => {
    it("calls stopProjectTracking, refreshes active sessions and tasks, and returns the new tracked minutes", async () => {
      const { stopProjectTracking: apiStopProjectTracking, getActiveSessions } = await import("./api");
      const { refreshTasks } = await import("./tasks.svelte");
      vi.mocked(apiStopProjectTracking).mockResolvedValue(99);
      vi.mocked(getActiveSessions).mockResolvedValue([]);
      vi.mocked(refreshTasks).mockResolvedValue(undefined);
      const { stopProjectTracking } = await import("./tracking.svelte");

      const result = await stopProjectTracking("project-1");

      expect(apiStopProjectTracking).toHaveBeenCalledWith("project-1");
      expect(refreshTasks).toHaveBeenCalled();
      expect(result).toBe(99);
    });
  });

  describe("initTracking", () => {
    beforeEach(() => {
      vi.useFakeTimers();
    });

    it("does not call heartbeat when there are no active sessions", async () => {
      const { heartbeat } = await import("./api");
      const { initTracking } = await import("./tracking.svelte");

      initTracking();
      await vi.advanceTimersByTimeAsync(60_000);

      expect(heartbeat).not.toHaveBeenCalled();
    });

    it("calls heartbeat once per active session's task_id every 30 seconds while sessions are running", async () => {
      const { heartbeat } = await import("./api");
      const { trackingState, initTracking } = await import("./tracking.svelte");

      initTracking();
      trackingState.activeSessions = [makeEntry({ task_id: "task-1" }), makeEntry({ task_id: "task-2" })];

      await vi.advanceTimersByTimeAsync(30_000);

      expect(heartbeat).toHaveBeenCalledWith("task-1");
      expect(heartbeat).toHaveBeenCalledWith("task-2");
      expect(heartbeat).toHaveBeenCalledTimes(2);
    });

    it("stops calling heartbeat once activeSessions becomes empty again", async () => {
      const { heartbeat } = await import("./api");
      const { trackingState, initTracking } = await import("./tracking.svelte");

      initTracking();
      trackingState.activeSessions = [makeEntry({ task_id: "task-1" })];
      await vi.advanceTimersByTimeAsync(30_000);
      expect(heartbeat).toHaveBeenCalledTimes(1);

      trackingState.activeSessions = [];
      await vi.advanceTimersByTimeAsync(30_000);

      expect(heartbeat).toHaveBeenCalledTimes(1);
    });

    it("does not advance nowMs on the per-second tick when there are no active sessions", async () => {
      const { trackingState, initTracking } = await import("./tracking.svelte");
      const initialNowMs = trackingState.nowMs;

      initTracking();
      await vi.advanceTimersByTimeAsync(5_000);

      expect(trackingState.nowMs).toBe(initialNowMs);
    });

    it("advances nowMs roughly every second while a session is active", async () => {
      const { trackingState, initTracking } = await import("./tracking.svelte");

      initTracking();
      trackingState.activeSessions = [makeEntry({ task_id: "task-1" })];
      const nowMsAfterStart = trackingState.nowMs;
      await vi.advanceTimersByTimeAsync(3_000);

      expect(trackingState.nowMs).toBeGreaterThan(nowMsAfterStart);
    });
  });

  describe("resolveAutoTransitionStatusId", () => {
    it("returns the explicitly configured status id outright, with no fallback computation", async () => {
      const { resolveAutoTransitionStatusId } = await import("./tracking.svelte");
      const settings = makeSettings({ tracking_auto_transition_status_id: "blocked" });

      expect(resolveAutoTransitionStatusId(settings)).toBe("blocked");
    });

    it("falls back to the first non-backlog/non-done/non-cancelled status against the default seeded list", async () => {
      const { resolveAutoTransitionStatusId } = await import("./tracking.svelte");
      const settings = makeSettings({ tracking_auto_transition_status_id: undefined });

      expect(resolveAutoTransitionStatusId(settings)).toBe("do");
    });

    it("excludes the cancelled status from the fallback candidates", async () => {
      const { resolveAutoTransitionStatusId } = await import("./tracking.svelte");
      const settings = makeSettings({
        tracking_auto_transition_status_id: undefined,
        statuses: [
          { id: "backlog", label: "Backlog", order: 1, color: "#000" },
          { id: "cancelled", label: "Cancelled", order: 2, color: "#000" },
          { id: "done", label: "Done", order: 3, color: "#000" },
          { id: "do", label: "Do", order: 4, color: "#000" },
        ],
        cancelled_status: "cancelled",
        done_status: "done",
      });

      expect(resolveAutoTransitionStatusId(settings)).toBe("do");
    });

    it("returns undefined when only backlog/done/cancelled statuses are defined", async () => {
      const { resolveAutoTransitionStatusId } = await import("./tracking.svelte");
      const settings = makeSettings({
        tracking_auto_transition_status_id: undefined,
        statuses: [
          { id: "backlog", label: "Backlog", order: 1, color: "#000" },
          { id: "done", label: "Done", order: 2, color: "#000" },
        ],
        done_status: "done",
        cancelled_status: undefined,
      });

      expect(resolveAutoTransitionStatusId(settings)).toBeUndefined();
    });

    it("treats the lowest-order status as the implicit backlog even if it isn't literally id 'backlog'", async () => {
      const { resolveAutoTransitionStatusId } = await import("./tracking.svelte");
      const settings = makeSettings({
        tracking_auto_transition_status_id: undefined,
        statuses: [
          { id: "todo", label: "Todo", order: 0, color: "#000" },
          { id: "doing", label: "Doing", order: 1, color: "#000" },
          { id: "done", label: "Done", order: 2, color: "#000" },
        ],
        done_status: "done",
        cancelled_status: undefined,
      });

      expect(resolveAutoTransitionStatusId(settings)).toBe("doing");
    });

    it("respects status order rather than array order when resolving the fallback", async () => {
      const { resolveAutoTransitionStatusId } = await import("./tracking.svelte");
      const settings = makeSettings({
        tracking_auto_transition_status_id: undefined,
        statuses: [
          { id: "in-progress", label: "In Progress", order: 3, color: "#000" },
          { id: "done", label: "Done", order: 5, color: "#000" },
          { id: "backlog", label: "Backlog", order: 1, color: "#000" },
          { id: "do", label: "Do", order: 2, color: "#000" },
          { id: "blocked", label: "Blocked", order: 4, color: "#000" },
        ],
        done_status: "done",
        cancelled_status: undefined,
      });

      expect(resolveAutoTransitionStatusId(settings)).toBe("do");
    });
  });

  describe("isOrphaned", () => {
    it("returns true when last_heartbeat_at is null", async () => {
      const { isOrphaned } = await import("./tracking.svelte");
      const entry = makeEntry({ last_heartbeat_at: null });

      expect(isOrphaned(entry, Date.now(), 2 * 60 * 1000)).toBe(true);
    });

    it("returns true when last_heartbeat_at is older than the grace window", async () => {
      const { isOrphaned } = await import("./tracking.svelte");
      const now = Date.parse("2026-06-15T09:10:00+00:00");
      const entry = makeEntry({ last_heartbeat_at: "2026-06-15T09:00:00+00:00" });

      expect(isOrphaned(entry, now, 2 * 60 * 1000)).toBe(true);
    });

    it("returns false when last_heartbeat_at is within the grace window", async () => {
      const { isOrphaned } = await import("./tracking.svelte");
      const now = Date.parse("2026-06-15T09:01:00+00:00");
      const entry = makeEntry({ last_heartbeat_at: "2026-06-15T09:00:00+00:00" });

      expect(isOrphaned(entry, now, 2 * 60 * 1000)).toBe(false);
    });

    it("returns false exactly at the grace boundary (strictly greater-than, not greater-or-equal)", async () => {
      const { isOrphaned } = await import("./tracking.svelte");
      const now = Date.parse("2026-06-15T09:00:00+00:00") + 2 * 60 * 1000;
      const entry = makeEntry({ last_heartbeat_at: "2026-06-15T09:00:00+00:00" });

      expect(isOrphaned(entry, now, 2 * 60 * 1000)).toBe(false);
    });

    it("returns true just past the grace boundary", async () => {
      const { isOrphaned } = await import("./tracking.svelte");
      const now = Date.parse("2026-06-15T09:00:00+00:00") + 2 * 60 * 1000 + 1;
      const entry = makeEntry({ last_heartbeat_at: "2026-06-15T09:00:00+00:00" });

      expect(isOrphaned(entry, now, 2 * 60 * 1000)).toBe(true);
    });
  });
});
