import { isSubtask } from "./subtasks";
import type { Task } from "./types";

/**
 * Whether `task` should be excluded from standalone rendering on a Board,
 * Week, or Calendar view — i.e. never shown as its own column card or bar
 * there. Only ever `true` for an actual subtask; a non-subtask is always
 * shown normally.
 *
 * Per the Subtasks design spec, "In the parent task's own board... subtasks
 * appear as a nested row glued to the parent — not as independent
 * draggable cards there": a subtask is unconditionally excluded from
 * standalone display everywhere except the one place its own container
 * project is being viewed directly (`viewedProjectId === task.project_id`)
 * — that board's entire purpose is to show its subtasks as ordinary,
 * independently-draggable cards. The `showSubtasks` setting doesn't change
 * *this* exclusion at all; it only governs whether `TaskCard` renders the
 * glued nested row in its place (on) or nothing (off, "vanish entirely") —
 * a subtask must never appear as a standalone card in either case, or it
 * would double up with its own nested row.
 */
export function isHiddenAsSubtask(task: Task, allTasks: Task[], viewedProjectId: string | undefined): boolean {
  return isSubtask(task, allTasks) && task.project_id !== viewedProjectId;
}
