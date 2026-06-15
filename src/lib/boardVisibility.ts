import type { Task } from "./types";

/**
 * Whether `task` should appear on a Kanban board on `today` (a "YYYY-MM-DD"
 * date string). A task with a `scheduled` date strictly after `today` is
 * hidden from Kanban boards until that date arrives - it remains visible in
 * the week view regardless. Tasks with no `scheduled` date, or one on or
 * before `today`, are always visible.
 */
export function isVisibleOnBoard(task: Task, today: string): boolean {
  if (!task.scheduled) return true;
  return task.scheduled <= today;
}
