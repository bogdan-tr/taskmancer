import type { Project, StatusDefinition } from "./types";

/** Returns `true` if `a` and `b` contain the same statuses in the same order. */
export function statusesEqual(a: StatusDefinition[], b: StatusDefinition[]): boolean {
  if (a.length !== b.length) return false;

  return a.every((status, index) => {
    const other = b[index];
    return (
      status.id === other.id &&
      status.label === other.label &&
      status.color === other.color &&
      status.order === other.order
    );
  });
}

/** Returns a copy of `statuses` with `order` set to each status's 1-based position. */
export function renumber(statuses: StatusDefinition[]): StatusDefinition[] {
  return statuses.map((status, index) => ({ ...status, order: index + 1 }));
}

/**
 * Returns `base` if it isn't already used by `existingIds`, otherwise
 * `${base}-2`, `${base}-3`, etc. until an unused id is found.
 */
export function uniqueId(existingIds: string[], base: string): string {
  if (!existingIds.includes(base)) return base;

  let suffix = 2;
  while (existingIds.includes(`${base}-${suffix}`)) suffix++;
  return `${base}-${suffix}`;
}

/**
 * Returns the names of projects whose board configuration references
 * `statusId`, either in their configured `board.statuses` column subset or
 * as their `board.default_status`.
 */
export function projectsReferencingStatus(projects: Project[], statusId: string): string[] {
  return projects
    .filter(
      (project) =>
        project.board.statuses.includes(statusId) || project.board.default_status === statusId,
    )
    .map((project) => project.name);
}

/**
 * Returns a human-readable reason `status` can't be deleted, or `undefined`
 * if deletion is allowed. `referencingProjects` is the result of
 * [`projectsReferencingStatus`] for this status.
 */
export function deleteBlockReason(
  status: StatusDefinition,
  statusCount: number,
  defaultStatusId: string | undefined,
  taskCounts: Record<string, number>,
  referencingProjects: string[],
): string | undefined {
  if (statusCount <= 1) {
    return "At least one status is required";
  }

  if (status.id === defaultStatusId) {
    return "This is the default status and can't be deleted";
  }

  const count = taskCounts[status.id] ?? 0;
  if (count > 0) {
    const verb = count === 1 ? "uses" : "use";
    return `${count} task${count === 1 ? "" : "s"} ${verb} this status — reassign them first`;
  }

  if (referencingProjects.length > 0) {
    const projectList = referencingProjects.map((name) => `"${name}"`).join(", ");
    return referencingProjects.length === 1
      ? `Used by project ${projectList} — update its board first`
      : `Used by projects ${projectList} — update their boards first`;
  }

  return undefined;
}
