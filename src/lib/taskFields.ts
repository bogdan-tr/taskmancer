import type { Task } from "./types";

/** Splits a comma-separated tag string into a normalized list of tags. */
export function parseTags(value: string): string[] {
  return value
    .split(",")
    .map((tag) => tag.trim())
    .filter((tag) => tag.length > 0);
}

/** Joins a list of tags into a comma-separated string for editing. */
export function formatTags(tags: string[]): string {
  return tags.join(", ");
}

/** Converts an empty/whitespace-only string to `undefined` for optional fields. */
export function emptyToUndefined(value: string): string | undefined {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : undefined;
}

const ISO_DATE_PATTERN = /^\d{4}-\d{2}-\d{2}$/;

/** Returns true if `value` is empty/whitespace-only or matches `YYYY-MM-DD`. */
export function isValidOptionalDate(value: string): boolean {
  return value.trim().length === 0 || ISO_DATE_PATTERN.test(value.trim());
}

/** Returns true if `a` and `b` contain the same tags, ignoring order. */
function sameTags(a: string[], b: string[]): boolean {
  if (a.length !== b.length) return false;
  const sortedA = [...a].sort();
  const sortedB = [...b].sort();
  return sortedA.every((tag, index) => tag === sortedB[index]);
}

/**
 * Whether any field a recurring task's series shares across all its
 * occurrences (title, project, priority, tags, estimated time, notes)
 * differs between `original` and `edited` — used to decide whether saving
 * an edit to a recurring task needs to ask "just this task vs. this and
 * future", since per-occurrence-only fields (status, due, scheduled) never
 * need that prompt, the same way dragging a task to reschedule it never
 * does.
 */
export function seriesSharedFieldsChanged(original: Task, edited: Task): boolean {
  return (
    original.title !== edited.title ||
    original.project_id !== edited.project_id ||
    original.priority !== edited.priority ||
    original.estimated_minutes !== edited.estimated_minutes ||
    original.notes !== edited.notes ||
    !sameTags(original.tags, edited.tags)
  );
}

/**
 * Whether any field the Task Detail Panel can edit (title, project, tags,
 * priority, status, due, scheduled, estimated time, notes) differs between
 * `original` and `edited`. Drives the panel's auto-save-on-blur skip: when
 * nothing changed, no save (and no "Saving…/Saved" flicker) is triggered.
 * Tags are compared order-insensitively, matching `seriesSharedFieldsChanged`.
 */
export function taskEditableFieldsChanged(original: Task, edited: Task): boolean {
  return (
    original.title !== edited.title ||
    original.project_id !== edited.project_id ||
    original.priority !== edited.priority ||
    original.status !== edited.status ||
    original.due !== edited.due ||
    original.scheduled !== edited.scheduled ||
    original.estimated_minutes !== edited.estimated_minutes ||
    original.notes !== edited.notes ||
    !sameTags(original.tags, edited.tags)
  );
}
