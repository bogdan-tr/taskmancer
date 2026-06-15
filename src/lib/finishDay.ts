/** Formats the result of `finishDay` for display, e.g. "3 tasks archived". */
export function formatFinishDayResult(archivedCount: number): string {
  const noun = archivedCount === 1 ? "task" : "tasks";
  return `${archivedCount} ${noun} archived`;
}
