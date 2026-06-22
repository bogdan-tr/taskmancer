import type { ProjectBoard } from "./types";

/** Returns `true` if `a` and `b` describe the same board configuration. */
export function boardsEqual(a: ProjectBoard, b: ProjectBoard): boolean {
  return (
    a.statuses.length === b.statuses.length &&
    a.statuses.every((id, index) => id === b.statuses[index]) &&
    a.default_status === b.default_status
  );
}

/**
 * Returns the status ids shown as columns on a board, in display order:
 * the nearest customized board in `boardChain` (a project's own board,
 * then its ancestors' boards, nearest-first — see
 * `crate::project_tree::self_and_ancestors`'s frontend mirror,
 * `selfAndAncestors`) if any has a non-empty `statuses` list, otherwise
 * every id in `allStatusIds` (the global status list, in its configured
 * order).
 */
export function effectiveBoardStatuses(boardChain: ProjectBoard[], allStatusIds: string[]): string[] {
  const customized = boardChain.find((board) => board.statuses.length > 0);
  return customized ? customized.statuses : allStatusIds;
}
