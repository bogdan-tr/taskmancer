import { DEFAULT_PROJECT_COLOR, type Project } from "./types";

/** The project's stored color, or `DEFAULT_PROJECT_COLOR` for tasks with no project or an unrecognized one. */
export function resolveProjectColor(projectName: string | undefined, projects: Project[]): string {
  if (!projectName) return DEFAULT_PROJECT_COLOR;
  return projects.find((project) => project.name === projectName)?.color ?? DEFAULT_PROJECT_COLOR;
}

/**
 * The project's `board.card_lightness` override, or `globalLightness` for
 * tasks with no project, an unrecognized project, or a project that hasn't
 * overridden it. `?? globalLightness` (not `||`) so an override of exactly
 * `0` is respected rather than treated as unset.
 */
export function resolveCardLightness(
  projectName: string | undefined,
  projects: Project[],
  globalLightness: number,
): number {
  if (!projectName) return globalLightness;
  return projects.find((project) => project.name === projectName)?.board.card_lightness ?? globalLightness;
}

/** Same as `resolveCardLightness`, but for `board.bar_lightness` (week/calendar-view bars). */
export function resolveBarLightness(
  projectName: string | undefined,
  projects: Project[],
  globalLightness: number,
): number {
  if (!projectName) return globalLightness;
  return projects.find((project) => project.name === projectName)?.board.bar_lightness ?? globalLightness;
}
