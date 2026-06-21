import { invoke } from "@tauri-apps/api/core";
import type { ParsedTaskInput } from "./naturalLanguage";
import type { DueRule, RecurrenceFrequency, SeriesEditScope } from "./recurrence";
import type {
  DeleteProjectResult,
  FinishDayResult,
  Project,
  ProjectTaskStrategy,
  Series,
  Settings,
  Task,
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
