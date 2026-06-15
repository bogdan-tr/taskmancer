import { invoke } from "@tauri-apps/api/core";
import type { ParsedTaskInput } from "./naturalLanguage";
import type {
  DeleteProjectResult,
  FinishDayResult,
  Project,
  ProjectTaskStrategy,
  Settings,
  Task,
} from "./types";

export async function listTasks(): Promise<Task[]> {
  return invoke<Task[]>("list_tasks");
}

export async function createTask(input: ParsedTaskInput): Promise<Task> {
  return invoke<Task>("create_task", {
    title: input.title,
    project: input.project,
    tags: input.tags.length > 0 ? input.tags : undefined,
    priority: input.priority,
    due: input.due,
    scheduled: input.scheduled,
  });
}

export async function updateTask(task: Task): Promise<Task> {
  return invoke<Task>("update_task", { task });
}

export async function deleteTask(id: string): Promise<void> {
  return invoke<void>("delete_task", { id });
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

export async function createProject(name: string, color?: string): Promise<Project> {
  return invoke<Project>("create_project", { name, color });
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
