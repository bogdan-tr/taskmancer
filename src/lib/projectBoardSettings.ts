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
 * `board.statuses` if the board has been customized, otherwise every id in
 * `allStatusIds` (the global status list, in its configured order).
 */
export function effectiveBoardStatuses(board: ProjectBoard, allStatusIds: string[]): string[] {
  return board.statuses.length > 0 ? board.statuses : allStatusIds;
}
