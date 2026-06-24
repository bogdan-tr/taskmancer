import {
  getActiveSessions,
  heartbeat,
  startProjectTracking as apiStartProjectTracking,
  startTracking,
  stopProjectTracking as apiStopProjectTracking,
  stopTracking,
  updateTask,
} from "./api";
import { settingsState } from "./settings.svelte";
import { refreshTasks, tasksState, upsertCachedTask } from "./tasks.svelte";
import type { Settings, Task, TimeEntry } from "./types";

/** How often the live `nowMs` ticker advances while any session is active. */
const TICK_INTERVAL_MS = 1_000;
/** How often each active session's `task_id` is heartbeat-ed while running — see the time-tracking-engine spec's "Heartbeat" section. */
const HEARTBEAT_INTERVAL_MS = 30_000;

/**
 * How stale `last_heartbeat_at` must be before a still-active session
 * (`get_active_sessions` returned it with `ended_at === null`) is treated as
 * orphaned on launch — see `isOrphaned`. Two minutes is long enough that an
 * ordinary quick app relaunch (close and reopen within a normal few-second
 * gap, well under one heartbeat interval) never false-positives, while still
 * being short enough to catch a real force-quit/crash/power-loss within a
 * reasonable window rather than leaving a falsely-still-running session
 * indefinitely unflagged.
 */
export const ORPHAN_GRACE_MS = 2 * 60 * 1000;

/**
 * Boxed in an object because Svelte 5 forbids exporting a reassigned `$state`
 * binding directly from a module — only its properties may be mutated.
 *
 * `nowMs` is the shared "live clock" tick: a single `setInterval` (started by
 * `initTracking`) updates it once a second while `activeSessions.length > 0`,
 * rather than every visible card running its own interval. `elapsedSecondsFor`
 * reads it to compute each active session's live elapsed time.
 */
export const trackingState = $state<{ activeSessions: TimeEntry[]; nowMs: number }>({
  activeSessions: [],
  nowMs: Date.now(),
});

/**
 * Reloads the full active-sessions list from the backend — the source of
 * truth for "what's running" across every surface (`TaskCard`, the
 * per-project header button, `TaskEditDialog`). On failure the previously
 * loaded sessions are left in place, exactly mirroring `refreshTasks`/
 * `refreshProjects`'s swallow-and-keep-stale shape.
 */
export async function refreshActiveSessions(): Promise<void> {
  try {
    trackingState.activeSessions = await getActiveSessions();
  } catch {
    // Keep the previously loaded sessions.
  }
}

/** `true` when `taskId` has a currently-active tracking session. */
export function isTaskActive(taskId: string): boolean {
  return trackingState.activeSessions.some((entry) => entry.task_id === taskId);
}

/** The active session for `taskId`, or `undefined` if it isn't currently running. */
export function activeSessionFor(taskId: string): TimeEntry | undefined {
  return trackingState.activeSessions.find((entry) => entry.task_id === taskId);
}

/**
 * Seconds elapsed since `taskId`'s active session started, computed live
 * from `trackingState.nowMs` — `undefined` if `taskId` isn't currently
 * active. Floored at 0 as a safety net against clock skew or an
 * unexpectedly-future `started_at`.
 */
export function elapsedSecondsFor(taskId: string): number | undefined {
  const entry = activeSessionFor(taskId);
  if (!entry) return undefined;
  return Math.max(0, Math.floor((trackingState.nowMs - Date.parse(entry.started_at)) / 1000));
}

/**
 * The live "total tracked so far" seconds to display for `task` while its
 * timer is running — `task.tracked_minutes` (every *past, completed*
 * session, in minutes — see `recompute_and_persist_tracked_minutes`, which
 * deliberately excludes the currently-running one) converted to seconds,
 * plus `elapsedSecondsFor`'s count of the current session alone.
 * `undefined` if `task.id` isn't currently active.
 *
 * Without this, the displayed ticker would restart from `0:00` on every
 * resume (`elapsedSecondsFor` alone only ever knows about the session that's
 * running right now), even though the underlying cumulative total was never
 * actually lost — pause-then-resume needs to visibly continue from where it
 * left off, not look like it reset.
 */
export function liveTrackedSecondsFor(task: Pick<Task, "id" | "tracked_minutes">): number | undefined {
  const sessionSeconds = elapsedSecondsFor(task.id);
  if (sessionSeconds === undefined) return undefined;
  return task.tracked_minutes * 60 + sessionSeconds;
}

/**
 * `Settings.card_tracked_time_display`'s two values — see that field's own
 * Rust doc comment for the exact meaning of each.
 */
export type CardTrackedTimeDisplay = "total" | "session";

/**
 * The live ticker seconds to show for `task` while its timer is running,
 * per the configurable `Settings.card_tracked_time_display` setting:
 * `"total"` (the default) shows `liveTrackedSecondsFor` (cumulative,
 * survives pause/resume); `"session"` shows only `elapsedSecondsFor` (just
 * the current session, restarting from `0:00` on every resume — a
 * deliberate choice some users want, e.g. to see "how long have I been at
 * this right now"). `undefined` if `task.id` isn't currently active,
 * regardless of mode.
 *
 * This only affects the *live*, actively-running display. The static chip
 * shown once a task's timer is stopped always shows the lifetime total
 * (`task.tracked_minutes`) either way — there's no "current session" once
 * stopped, so this setting has nothing to apply to at that point.
 */
export function liveDisplaySecondsFor(
  task: Pick<Task, "id" | "tracked_minutes">,
  displayMode: CardTrackedTimeDisplay,
): number | undefined {
  return displayMode === "session" ? elapsedSecondsFor(task.id) : liveTrackedSecondsFor(task);
}

/**
 * Resolves the status id a task should auto-transition to when tracking
 * starts, per the time-tracking-engine spec's "auto-transition status on
 * tracking start" setting:
 *
 * - If `tracking_auto_transition_status_id` is explicitly set, that value
 *   wins outright — no fallback computation runs at all.
 * - Otherwise, falls back to the first status (by `order` ascending) that
 *   isn't the lowest-`order` status (treated as the implicit "backlog"),
 *   the configured `done_status`, or the configured `cancelled_status`.
 *   `undefined` if no such status exists (e.g. only backlog/done are
 *   defined).
 */
export function resolveAutoTransitionStatusId(settings: Settings): string | undefined {
  if (settings.tracking_auto_transition_status_id) return settings.tracking_auto_transition_status_id;

  const ordered = [...settings.statuses].sort((a, b) => a.order - b.order);
  const initialStatusId = ordered[0]?.id;
  return ordered.find(
    (status) =>
      status.id !== initialStatusId &&
      status.id !== settings.done_status &&
      status.id !== settings.cancelled_status,
  )?.id;
}

/**
 * `true` when an active session (`entry.ended_at === null`, per
 * `get_active_sessions`) should be treated as orphaned on launch: either it
 * has never been heartbeat-ed (`last_heartbeat_at === null`) or its last
 * heartbeat is older than `graceMs` relative to `nowMs`. Both `nowMs` and
 * `graceMs` are passed in rather than read from `trackingState`/a module
 * constant so this stays a pure, directly-testable function — the caller
 * (`+layout.svelte`'s `onMount`) is responsible for capturing a fresh
 * `Date.now()` once per check, not reusing `trackingState.nowMs` (which only
 * advances while sessions are active and could itself be stale immediately
 * after launch, before `initTracking`'s tick has run even once).
 */
export function isOrphaned(entry: TimeEntry, nowMs: number, graceMs: number): boolean {
  if (entry.last_heartbeat_at === null) return true;
  return nowMs - Date.parse(entry.last_heartbeat_at) > graceMs;
}

/**
 * Best-effort auto-transitions `taskId` to the configured status after its
 * tracking session has started, per `resolveAutoTransitionStatusId`. Swallows
 * and silently drops any failure (lookup miss or a failed `updateTask` save)
 * rather than throwing — by the time this runs, `start_tracking` has already
 * succeeded, and a failed status change should never be reported as a failed
 * "start tracking" action to the user. Mirrors this codebase's existing
 * swallow-and-keep-stale `catch {}` convention (see `refreshTasks`/
 * `refreshSettings`) rather than introducing a first-ever `console.error`
 * call — there is no logging library wired into this app, and every other
 * background failure in this codebase is silently dropped the same way.
 */
async function applyAutoTransition(taskId: string): Promise<void> {
  const settings = settingsState.current;
  if (!settings || !settings.tracking_auto_transition_enabled) return;

  const targetStatusId = resolveAutoTransitionStatusId(settings);
  if (!targetStatusId) return;

  const task = tasksState.items.find((t) => t.id === taskId);
  if (!task || task.status === targetStatusId) return;

  try {
    const updated = await updateTask({ ...task, status: targetStatusId });
    upsertCachedTask(updated);
  } catch {
    // A failed auto-transition must never block the timer itself from
    // having started successfully — see this function's own doc comment.
  }
}

/**
 * Starts tracking `taskId`, then re-fetches the active-sessions list from
 * the backend — re-fetching this small list is cheap and avoids any risk of
 * a local optimistic update drifting from server truth, unlike the heavier
 * per-task object this codebase's `upsertCachedTask` pattern optimizes for.
 *
 * Afterward, best-effort applies the "auto-transition status on tracking
 * start" setting (see `applyAutoTransition`) — deliberately not applied to
 * `startProjectTracking`'s hidden tracker task, which has no meaningful
 * board-facing status to transition.
 */
export async function startTaskTracking(taskId: string): Promise<void> {
  await startTracking(taskId);
  await refreshActiveSessions();
  await applyAutoTransition(taskId);
}

/**
 * Stops tracking `taskId` and re-fetches the active-sessions list. Returns
 * the new `tracked_minutes` the backend computed, so the caller can
 * `upsertCachedTask` (from `./tasks.svelte`) with the corrected value — this
 * store deliberately doesn't reach into `tasksState` itself; each UI call
 * site does that explicitly, mirroring how `KanbanBoard.svelte` already
 * calls `upsertCachedTask` after every other mutation.
 */
export async function stopTaskTracking(taskId: string): Promise<number> {
  const trackedMinutes = await stopTracking(taskId);
  await refreshActiveSessions();
  return trackedMinutes;
}

/** Starts tracking `projectId` as a whole (lazily creating its hidden tracker task on first call server-side), then re-fetches the active-sessions list. */
export async function startProjectTracking(projectId: string): Promise<void> {
  await apiStartProjectTracking(projectId);
  await refreshActiveSessions();
}

/**
 * Stops tracking `projectId` as a whole, re-fetches the active-sessions
 * list, and re-syncs the global task cache via `refreshTasks` (from
 * `./tasks.svelte`) rather than a manual `upsertCachedTask` — the caller
 * only has `projectId`, not the hidden tracker task's id readily in scope
 * at every call site (`KanbanBoard.svelte`'s `project.tracking_task_id` is
 * one lookup away, but a full re-sync is simpler and still cheap; the
 * hidden task is just one more row in the same `list_tasks` response).
 * Returns the new `tracked_minutes` the backend computed for the hidden
 * tracker task.
 */
export async function stopProjectTracking(projectId: string): Promise<number> {
  const trackedMinutes = await apiStopProjectTracking(projectId);
  await refreshActiveSessions();
  await refreshTasks();
  return trackedMinutes;
}

let heartbeatIntervalStarted = false;

/**
 * Synchronous setup for both the live-clock tick and the heartbeat interval
 * — called once from `+layout.svelte`'s `<script>` top level alongside
 * `initSidebar()`/`initTheme()` etc. Each `setInterval` runs for the
 * lifetime of the app (never torn down, exactly like there's no precedent
 * in this codebase for tearing down a top-level interval — `+layout.svelte`
 * never unmounts) and checks `trackingState.activeSessions.length > 0` on
 * every tick to decide whether to do anything that tick, not whether to
 * keep running.
 *
 * Guarded by `heartbeatIntervalStarted` so calling this more than once
 * *within a single module instance's lifetime* (e.g. a test importing the
 * module and calling this twice) doesn't stack duplicate intervals. This
 * guard does NOT protect against Vite's dev-mode hot-module-reload, which
 * replaces the module instance entirely (resetting this flag and
 * `trackingState` to fresh values) — an HMR edit to this file can leave the
 * previous instance's two intervals running orphaned in the background.
 * Acceptable for now since this only affects the dev server, never a
 * built/shipped app; a real fix would need `import.meta.hot.dispose` to
 * clear the old intervals before the new module instance's `initTracking()`
 * runs.
 */
export function initTracking(): void {
  if (heartbeatIntervalStarted) return;
  heartbeatIntervalStarted = true;

  setInterval(() => {
    if (trackingState.activeSessions.length === 0) return;
    trackingState.nowMs = Date.now();
  }, TICK_INTERVAL_MS);

  setInterval(() => {
    if (trackingState.activeSessions.length === 0) return;
    for (const entry of trackingState.activeSessions) {
      void heartbeat(entry.task_id);
    }
  }, HEARTBEAT_INTERVAL_MS);
}
