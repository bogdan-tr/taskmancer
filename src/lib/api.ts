import { invoke } from "@tauri-apps/api/core";
import type { ParsedTaskInput } from "./naturalLanguage";
import type { Project, Task, TaskStatus } from "./types";

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

export async function updateTaskStatus(id: string, status: TaskStatus): Promise<Task> {
  return invoke<Task>("update_task_status", { id, status });
}

export async function updateTask(task: Task): Promise<Task> {
  return invoke<Task>("update_task", { task });
}

export async function deleteTask(id: string): Promise<void> {
  return invoke<void>("delete_task", { id });
}

export async function reorderTask(id: string, order: number, status: TaskStatus): Promise<Task> {
  return invoke<Task>("reorder_task", { id, order, status });
}

export async function listProjects(): Promise<Project[]> {
  return invoke<Project[]>("list_projects");
}

export async function createProject(name: string, color?: string): Promise<Project> {
  return invoke<Project>("create_project", { name, color });
}
