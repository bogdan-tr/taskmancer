import type { Project, ProjectTaskStrategy, Task } from "./types";

/**
 * Returns `true` if `projectName` is the configured default project
 * (case-insensitive, ignoring surrounding whitespace) - mirrors
 * `ensure_not_default_project` in `src-tauri/src/commands.rs`. The default
 * project can never be deleted.
 */
export function isDefaultProject(projectName: string, defaultProjectName: string): boolean {
  return projectName.trim().toLowerCase() === defaultProjectName.trim().toLowerCase();
}

/**
 * Returns the tasks currently filed under `projectName` (matched
 * case-insensitively) - mirrors `tasks_for_project` in
 * `src-tauri/src/commands.rs`.
 */
export function tasksForProject(tasks: Task[], projectName: string): Task[] {
  const target = projectName.toLowerCase();
  return tasks.filter((task) => (task.project ?? "").toLowerCase() === target);
}

/** Projects that can be picked as a reassignment target: every project other than the one being deleted. */
export function reassignTargets(projects: Project[], excludeProjectId: string): Project[] {
  return projects.filter((project) => project.id !== excludeProjectId);
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
