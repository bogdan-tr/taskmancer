import { invoke } from "@tauri-apps/api/core";
import type { WeekStartsOn } from "./displaySettings.svelte";
import type { ParsedTaskInput } from "./naturalLanguage";
import type { DueRule, RecurrenceFrequency, SeriesEditScope } from "./recurrence";
import type {
  DeleteProjectResult,
  FinishDayResult,
  Project,
  ProjectStatusStats,
  ProjectTaskStrategy,
  Series,
  Settings,
  StatLayout,
  Task,
  TimeEntry,
} from "./types";

export async function listTasks(): Promise<Task[]> {
  return invoke<Task[]>("list_tasks");
}

export async function createTask(input: ParsedTaskInput, projectId?: string): Promise<Task> {
  return invoke<Task>("create_task", {
    title: input.title,
    projectId,
    tags: input.tags.length > 0 ? input.tags : undefined,
    priority: input.priority,
    status: input.status,
    due: input.due,
    scheduled: input.scheduled,
    estimatedMinutes: input.estimatedMinutes,
  });
}

/**
 * Creates a recurring task: a first occurrence plus a `Series`, with
 * occurrences immediately generated for the next 60 days. Returns every
 * task created (the first occurrence, then any generated ones).
 *
 * `dueRule`, if given, is the series' due rule exactly as
 * `resolveSeriesDueRule` derived it from whatever due phrase was typed —
 * see its own doc comment. `undefined` lets the backend apply the
 * configured project/global default, same as every other unset field.
 */
export async function createRecurringTask(
  input: ParsedTaskInput,
  frequency: RecurrenceFrequency,
  endDate: string | undefined,
  dueRule: DueRule | undefined,
  projectId?: string,
): Promise<Task[]> {
  return invoke<Task[]>("create_recurring_task", {
    title: input.title,
    projectId,
    tags: input.tags.length > 0 ? input.tags : undefined,
    priority: input.priority,
    status: input.status,
    due: input.due,
    scheduled: input.scheduled,
    estimatedMinutes: input.estimatedMinutes,
    frequency,
    endDate,
    dueRule,
  });
}

/** Extends a series' generated occurrences up through `through` (`YYYY-MM-DD`). Returns the newly created tasks. */
export async function ensureOccurrencesUntil(seriesId: string, through: string): Promise<Task[]> {
  return invoke<Task[]>("ensure_occurrences_until", { seriesId, through });
}

export async function updateTask(task: Task): Promise<Task> {
  return invoke<Task>("update_task", { task });
}

/** Updates an occurrence of a recurring task; `scope` decides how far the edit reaches (see `SeriesEditScope`). Returns every task changed. */
export async function updateSeriesOccurrence(task: Task, scope: SeriesEditScope): Promise<Task[]> {
  return invoke<Task[]>("update_series_occurrence", { task, scope });
}

export async function deleteTask(id: string): Promise<void> {
  return invoke<void>("delete_task", { id });
}

/** Deletes an occurrence of a recurring task; `scope` decides how far the deletion reaches (see `SeriesEditScope`). */
export async function deleteSeriesOccurrence(taskId: string, scope: SeriesEditScope): Promise<void> {
  return invoke<void>("delete_series_occurrence", { taskId, scope });
}

/** Stops a recurring task's series from generating any further occurrences. Existing occurrences keep their `series_id`. */
export async function removeRecurrence(taskId: string): Promise<void> {
  return invoke<void>("remove_recurrence", { taskId });
}

/** Returns a series' recurrence configuration, to pre-fill the recurrence builder when editing an existing recurring task. */
export async function getSeries(seriesId: string): Promise<Series> {
  return invoke<Series>("get_series", { seriesId });
}

/**
 * Updates an existing recurring task's frequency/due rule/end date —
 * always a whole-series change. Deletes every already-generated
 * occurrence on or after `cutoff` (including the occurrence the edit was
 * made from, if it falls on or after `cutoff` — pass that occurrence's
 * own `scheduled` date) and regenerates a fresh baseline under the new
 * rule; past occurrences are untouched. Returns the newly generated
 * tasks.
 */
export async function updateSeriesRecurrence(
  seriesId: string,
  cutoff: string,
  frequency: RecurrenceFrequency,
  dueRule: DueRule,
  endDate: string | undefined,
): Promise<Task[]> {
  return invoke<Task[]>("update_series_recurrence", {
    seriesId,
    cutoff,
    frequency,
    dueRule,
    endDate,
  });
}

export async function reorderTask(
  id: string,
  order: number,
  status: string,
  priority?: string,
): Promise<Task> {
  return invoke<Task>("reorder_task", { id, order, status, priority });
}

export async function listProjects(): Promise<Project[]> {
  return invoke<Project[]>("list_projects");
}

export async function createProject(name: string, color?: string, parentId?: string): Promise<Project> {
  return invoke<Project>("create_project", { name, color, parentId });
}

export async function updateProject(project: Project): Promise<Project> {
  return invoke<Project>("update_project", { project });
}

export async function deleteProject(
  projectId: string,
  taskStrategy?: ProjectTaskStrategy,
): Promise<DeleteProjectResult> {
  return invoke<DeleteProjectResult>("delete_project", { projectId, taskStrategy });
}

/** Returns `parentTaskId`'s subtask container, creating it on first call. */
export async function ensureSubtaskContainer(parentTaskId: string): Promise<Project> {
  return invoke<Project>("ensure_subtask_container", { parentTaskId });
}

/** Disbands `taskId`'s subtask container, moving its subtasks into `taskId`'s own project and removing the now-unused container. The subtasks themselves are kept, not deleted. Returns the updated task (its `subtask_project_id` cleared). */
export async function deleteSubtaskContainer(taskId: string): Promise<Task> {
  return invoke<Task>("delete_subtask_container", { taskId });
}

export async function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

export async function saveSettings(settings: Settings): Promise<Settings> {
  return invoke<Settings>("save_settings", { settings });
}

/** Returns the number of tasks currently using each priority id. */
export async function countTasksByPriority(): Promise<Record<string, number>> {
  return invoke<Record<string, number>>("count_tasks_by_priority");
}

/** Returns the number of tasks currently using each status id. */
export async function countTasksByStatus(): Promise<Record<string, number>> {
  return invoke<Record<string, number>>("count_tasks_by_status");
}

/** Archives every task whose status is the configured done or cancelled status. */
export async function finishDay(): Promise<FinishDayResult> {
  return invoke<FinishDayResult>("finish_day");
}

/** Starts a new tracking session for `taskId`. A no-op if `taskId` already has an active session. */
export async function startTracking(taskId: string): Promise<void> {
  return invoke<void>("start_tracking", { taskId });
}

/** Ends the active session for `taskId` and recomputes its `tracked_minutes`, returning the new value. A no-op (still returns the current value) if `taskId` has no active session. */
export async function stopTracking(taskId: string): Promise<number> {
  return invoke<number>("stop_tracking", { taskId });
}

/** Returns every currently-active session (`ended_at === null`) across all tasks, for restoring "what's running" UI state on load and for orphan detection. */
export async function getActiveSessions(): Promise<TimeEntry[]> {
  return invoke<TimeEntry[]>("get_active_sessions");
}

/** Updates `last_heartbeat_at` on `taskId`'s active session. A no-op if it has none. Called every ~30s by the frontend while any timer runs. */
export async function heartbeat(taskId: string): Promise<void> {
  return invoke<void>("heartbeat", { taskId });
}

/** Resolves an orphaned session detected on launch: `"resume"` leaves it untouched, `"discard"` ends it at its last known heartbeat. */
export async function resolveOrphanedSession(entryId: string, action: "resume" | "discard"): Promise<void> {
  return invoke<void>("resolve_orphaned_session", { entryId, action });
}

/** Adds a completed manual time entry for `taskId` spanning `[startedAt, endedAt]` (RFC3339 strings). */
export async function addManualTimeEntry(taskId: string, startedAt: string, endedAt: string): Promise<void> {
  return invoke<void>("add_manual_time_entry", { taskId, startedAt, endedAt });
}

/** Overwrites an existing time entry's `started_at`/`ended_at` by id, for manual correction. */
export async function updateTimeEntry(entryId: string, startedAt: string, endedAt: string): Promise<void> {
  return invoke<void>("update_time_entry", { entryId, startedAt, endedAt });
}

/** Deletes a time entry by id. */
export async function deleteTimeEntry(entryId: string): Promise<void> {
  return invoke<void>("delete_time_entry", { entryId });
}

/** Returns every time entry (active or completed) for `taskId`. */
export async function listTimeEntries(taskId: string): Promise<TimeEntry[]> {
  return invoke<TimeEntry[]>("list_time_entries", { taskId });
}

/** Starts tracking `projectId` as a whole, lazily creating its hidden tracker task on first call. */
export async function startProjectTracking(projectId: string): Promise<void> {
  return invoke<void>("start_project_tracking", { projectId });
}

/** Stops tracking `projectId` as a whole and recomputes its hidden tracker task's `tracked_minutes`, returning the new value. */
export async function stopProjectTracking(projectId: string): Promise<number> {
  return invoke<number>("stop_project_tracking", { projectId });
}

/** Returns every project-status-line stat for `projectId`, computed as of "now". */
export async function getProjectStatusStats(
  projectId: string,
  weekStartsOn: WeekStartsOn,
): Promise<ProjectStatusStats> {
  return invoke<ProjectStatusStats>("get_project_status_stats", { projectId, weekStartsOn });
}

/** Returns every saved `StatLayout`, in storage order. */
export async function listStatusLayouts(): Promise<StatLayout[]> {
  return invoke<StatLayout[]>("list_status_layouts");
}

/** Creates a new status-line `StatLayout` named `name` with `statIds`, and returns it. */
export async function createStatusLayout(name: string, statIds: string[]): Promise<StatLayout> {
  return invoke<StatLayout>("create_status_layout", { name, statIds });
}

/** Updates an existing `StatLayout` in place — every project (or the global default) pointing at `layout.id` sees the change immediately. */
export async function updateStatusLayout(layout: StatLayout): Promise<StatLayout> {
  return invoke<StatLayout>("update_status_layout", { layout });
}

/** Forks `layoutId` into a brand-new `StatLayout` named `newName`, with a freshly generated id and the same `stat_ids`. */
export async function duplicateStatusLayout(layoutId: string, newName: string): Promise<StatLayout> {
  return invoke<StatLayout>("duplicate_status_layout", { layoutId, newName });
}

/** Permanently deletes `layoutId`. Rejects if any project or the global default still references it. */
export async function deleteStatusLayout(layoutId: string): Promise<void> {
  return invoke<void>("delete_status_layout", { layoutId });
}
