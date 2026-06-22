import { allSubtasksDone, subtasksOf } from "./subtasks";
import type { Task } from "./types";

/**
 * Compares every task with a subtask container against the previous run's
 * "all done" set, to find transitions worth surfacing the all-done popup
 * for. A task only ever appears in `newlyDone` the run its non-cancelled
 * subtasks *become* fully done — not on every run where they happen to
 * still be done — which is what gives the popup its "won't auto-reshow
 * for the same completion" behavior described in the Subtasks design
 * spec: the caller just needs to keep feeding back `stillAllDone` as the
 * next call's `previouslyAllDone`, with no separate dismissal flag. A
 * subtask later un-done and re-done is a fresh transition (the task drops
 * out of `stillAllDone` in between), so it fires again correctly.
 *
 * `stillAllDone` starts from `previouslyAllDone` rather than empty, and a
 * task's membership is only ever changed when `tasks` actually contains
 * at least one of its subtasks. Two distinct cases are folded into that
 * one guard, both for the same underlying reason — this function can't
 * tell "genuinely no subtasks visible right now" apart from "the caller's
 * batch just doesn't have them yet":
 * - A task missing from `tasks` entirely (e.g. `KanbanBoard` is showing a
 *   different project's rolled-up list) keeps whatever membership it
 *   already had, rather than dropping out and spuriously re-triggering
 *   the popup once it's back in view with nothing about its actual
 *   completion having changed.
 * - A task *present* in `tasks` but whose subtasks aren't (yet) — e.g.
 *   right after a page reload, where the parent's own global cache can
 *   resolve before the current board's own task fetch finishes — would
 *   otherwise make `allSubtasksDone` see zero subtasks and conclude "not
 *   all done", incorrectly clearing real membership based on transiently
 *   incomplete data, which would then look like a brand new completion
 *   the very next tick once the real subtasks arrive.
 */
export function newlyAllDoneTaskIds(
  previouslyAllDone: Set<string>,
  tasks: Task[],
  doneStatusId: string | undefined,
  cancelledStatusId: string | undefined,
  today: string,
): { newlyDone: string[]; stillAllDone: Set<string> } {
  const stillAllDone = new Set(previouslyAllDone);
  const newlyDone: string[] = [];

  for (const task of tasks) {
    if (!task.subtask_project_id) continue;
    if (subtasksOf(task, tasks).length === 0) continue;

    if (allSubtasksDone(task, tasks, doneStatusId, cancelledStatusId, today)) {
      stillAllDone.add(task.id);
      if (!previouslyAllDone.has(task.id)) {
        newlyDone.push(task.id);
      }
    } else {
      stillAllDone.delete(task.id);
    }
  }

  return { newlyDone, stillAllDone };
}
