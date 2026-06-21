import { descendantsOf } from "./projectTree";
import type { Project, ProjectTaskStrategy, Task } from "./types";

/**
 * Returns `true` if `projectId` is the configured default project - mirrors
 * `ensure_not_default_project` in `src-tauri/src/commands.rs`. The default
 * project can never be deleted.
 */
export function isDefaultProject(projectId: string, defaultProjectId: string): boolean {
  return projectId === defaultProjectId;
}

/**
 * Returns the tasks currently filed under `projectId` - mirrors
 * `tasks_for_projects` (singular form) in `src-tauri/src/commands.rs`.
 */
export function tasksForProject(tasks: Task[], projectId: string): Task[] {
  return tasks.filter((task) => task.project_id === projectId);
}

/**
 * Returns the tasks currently filed under any id in `projectIds` - mirrors
 * `tasks_for_projects` in `src-tauri/src/commands.rs`, used when previewing
 * or performing a cascading delete across a project and its descendants.
 */
export function tasksForProjects(tasks: Task[], projectIds: string[]): Task[] {
  return tasks.filter((task) => task.project_id !== undefined && projectIds.includes(task.project_id));
}

/**
 * Projects that can be picked as a reassignment target: every project
 * other than the one being deleted and its own descendants — both are
 * about to be deleted too, so reassigning into either would just create
 * tasks with nowhere to go once the delete completes.
 */
export function reassignTargets(projects: Project[], excludeProjectId: string): Project[] {
  const excludedIds = new Set([
    excludeProjectId,
    ...descendantsOf(projects, excludeProjectId).map((p) => p.id),
  ]);
  return projects.filter((project) => !excludedIds.has(project.id));
}

export type DeleteStrategyKind = "reassign" | "archive" | "delete";

/** Builds the `task_strategy` payload for `deleteProject` from the dialog's selection. */
export function buildTaskStrategy(
  kind: DeleteStrategyKind,
  targetProjectId: string,
): ProjectTaskStrategy {
  switch (kind) {
    case "reassign":
      return { type: "reassign", target_project_id: targetProjectId };
    case "archive":
      return { type: "archive" };
    case "delete":
      return { type: "delete" };
  }
}
