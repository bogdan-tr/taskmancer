import type { Project } from "./types";

/** Returns the direct children of `parentId` (or every top-level project, if `parentId` is `undefined`), in the order they appear in `projects`. */
export function childrenOf(projects: Project[], parentId: string | undefined): Project[] {
  return projects.filter((p) => p.parent_id === parentId);
}

/** Returns the ancestors of the project identified by `id`, nearest-first, ending at the root. Empty if `id` doesn't exist in `projects` or names a top-level project. */
export function ancestorsOf(projects: Project[], id: string): Project[] {
  const result: Project[] = [];
  let currentId = projects.find((p) => p.id === id)?.parent_id;

  while (currentId !== undefined) {
    const ancestor = projects.find((p) => p.id === currentId);
    if (!ancestor) break;
    result.push(ancestor);
    currentId = ancestor.parent_id;
  }

  return result;
}

/** Returns the project identified by `id` (if found) followed by its ancestors, nearest-first — the full settings-resolution chain for that project, ending at the root. Empty if `id` doesn't exist in `projects`. */
export function selfAndAncestors(projects: Project[], id: string): Project[] {
  const self = projects.find((p) => p.id === id);
  if (!self) return [];
  return [self, ...ancestorsOf(projects, id)];
}

/** Returns the full ancestor path for the project identified by `id`, root-first and joined with "/" (e.g. "Work/Client A/Phase 1") — unambiguous even for same-named subprojects under different parents, unlike a bare leaf name. Empty string if `id` doesn't exist in `projects`. */
export function projectPath(projects: Project[], id: string): string {
  const self = projects.find((p) => p.id === id);
  if (!self) return "";
  const ancestorsRootFirst = ancestorsOf(projects, id).reverse();
  return [...ancestorsRootFirst, self].map((p) => p.name).join("/");
}

/**
 * Formats root-first path `segments` (e.g. from `projectPath`, split back
 * apart) as a literal `+Project` quick-add token's text — the inverse of
 * `parsePathSegments` in `naturalLanguage.ts` — quoting any segment that
 * contains whitespace so the result round-trips back through the parser
 * unambiguously (e.g. `["Work", "Client A"]` becomes `Work/"Client A"`).
 */
export function formatProjectPathToken(segments: string[]): string {
  return segments.map((segment) => (/\s/.test(segment) ? `"${segment}"` : segment)).join("/");
}

/**
 * Resolves a root-first ancestor path (e.g. `["Work", "Client A"]`, the
 * counterpart to `projectPath`'s string form) to the project it names: at
 * each level, the first child (or, for the first segment, the first
 * top-level project) whose name matches case-insensitively — mirroring
 * `find_project`'s existing case-insensitivity. Returns `undefined` as soon
 * as any level has no match (an invalid/typo'd path doesn't fall back to
 * matching the last segment as a bare name elsewhere in the tree — that
 * would be more confusing than just not matching), or if `segments` is
 * empty.
 */
export function findProjectByPath(projects: Project[], segments: string[]): Project | undefined {
  if (segments.length === 0) return undefined;

  let parentId: string | undefined = undefined;
  let match: Project | undefined;

  for (const segment of segments) {
    const lowerSegment = segment.toLowerCase();
    match = childrenOf(projects, parentId).find((p) => p.name.toLowerCase() === lowerSegment);
    if (!match) return undefined;
    parentId = match.id;
  }

  return match;
}

/** Returns every transitive descendant of the project identified by `id` (children, grandchildren, ...). */
export function descendantsOf(projects: Project[], id: string): Project[] {
  const result: Project[] = [];
  const frontier: string[] = [id];

  while (frontier.length > 0) {
    const currentId = frontier.pop();
    if (currentId === undefined) break;
    for (const child of childrenOf(projects, currentId)) {
      result.push(child);
      frontier.push(child.id);
    }
  }

  return result;
}

/** Returns `true` if making `newParentId` the parent of `movingId` would create a cycle — i.e. `newParentId` is `movingId` itself, or is one of `movingId`'s current descendants. */
export function wouldCreateCycle(projects: Project[], movingId: string, newParentId: string): boolean {
  if (movingId === newParentId) return true;
  return descendantsOf(projects, movingId).some((p) => p.id === newParentId);
}

export interface ProjectOrderUpdate {
  id: string;
  parent_id: string | undefined;
  order: number;
}

/**
 * Computes the project updates needed to persist `zoneItems` (a project
 * tree zone's current children, in display order, after a drag-and-drop
 * reorder/reparent) as the children of `zoneParentId` (`undefined` for the
 * top-level zone). Returns one `ProjectOrderUpdate` per item whose
 * `parent_id` and/or `order` actually needs to change — an item already
 * correctly parented and numbered isn't included, so callers only persist
 * what actually changed.
 *
 * `allProjects` (the full, current tree) is used for a cycle check: if any
 * item's `parent_id` would need to change to `zoneParentId`, but
 * `zoneParentId` is that item's own descendant (see `wouldCreateCycle`),
 * the whole result is `rejected: true` and `updates` is empty — callers
 * should treat a rejected drop as entirely invalid and re-sync from the
 * server rather than partially applying it.
 */
export function computeZoneOrderUpdates(
  allProjects: Project[],
  zoneParentId: string | undefined,
  zoneItems: Project[],
  orderStep = 1000,
): { updates: ProjectOrderUpdate[]; rejected: boolean } {
  const updates: ProjectOrderUpdate[] = [];

  for (const [index, item] of zoneItems.entries()) {
    const needsReparent = item.parent_id !== zoneParentId;
    if (needsReparent && zoneParentId !== undefined && wouldCreateCycle(allProjects, item.id, zoneParentId)) {
      return { updates: [], rejected: true };
    }

    const newOrder = (index + 1) * orderStep;
    if (needsReparent || item.order !== newOrder) {
      updates.push({ id: item.id, parent_id: zoneParentId, order: newOrder });
    }
  }

  return { updates, rejected: false };
}
