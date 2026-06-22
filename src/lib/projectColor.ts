import type { InkMode } from "./colorPresets";
import { selfAndAncestors } from "./projectTree";
import { DEFAULT_PROJECT_COLOR, type Project } from "./types";

/** The project's stored color, or `DEFAULT_PROJECT_COLOR` for tasks with no project or an unrecognized one. Color itself is never inherited — only board overrides are — so this still checks just the one project, not its ancestors. */
export function resolveProjectColor(projectId: string | undefined, projects: Project[]): string {
  if (!projectId) return DEFAULT_PROJECT_COLOR;
  return projects.find((project) => project.id === projectId)?.color ?? DEFAULT_PROJECT_COLOR;
}

/**
 * The nearest `board.card_lightness` override in `projectId`'s ancestor
 * chain (the project itself, then its ancestors nearest-first — see
 * `selfAndAncestors`), or `globalLightness` if none of them have one.
 * `?? globalLightness` (not `||`) so an override of exactly `0` is
 * respected rather than treated as unset.
 */
export function resolveCardLightness(
  projectId: string | undefined,
  projects: Project[],
  globalLightness: number,
): number {
  if (!projectId) return globalLightness;
  const chain = selfAndAncestors(projects, projectId);
  return chain.find((p) => p.board.card_lightness !== undefined)?.board.card_lightness ?? globalLightness;
}

/** Same as `resolveCardLightness`, but for `board.bar_lightness` (week/calendar-view bars). */
export function resolveBarLightness(
  projectId: string | undefined,
  projects: Project[],
  globalLightness: number,
): number {
  if (!projectId) return globalLightness;
  const chain = selfAndAncestors(projects, projectId);
  return chain.find((p) => p.board.bar_lightness !== undefined)?.board.bar_lightness ?? globalLightness;
}

/** Same as `resolveCardLightness`, but for `board.ink_mode` (color-coded card/bar text). */
export function resolveInkMode(
  projectId: string | undefined,
  projects: Project[],
  globalInkMode: InkMode,
): InkMode {
  if (!projectId) return globalInkMode;
  const chain = selfAndAncestors(projects, projectId);
  return chain.find((p) => p.board.ink_mode !== undefined)?.board.ink_mode ?? globalInkMode;
}
