import { DEFAULT_PROJECT_COLOR, type Project } from "./types";

/** The project's stored color, or `DEFAULT_PROJECT_COLOR` for tasks with no project or an unrecognized one. */
export function resolveProjectColor(projectName: string | undefined, projects: Project[]): string {
  if (!projectName) return DEFAULT_PROJECT_COLOR;
  return projects.find((project) => project.name === projectName)?.color ?? DEFAULT_PROJECT_COLOR;
}
