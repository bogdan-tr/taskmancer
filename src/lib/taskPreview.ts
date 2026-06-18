import type { ParsedTaskInput } from "./naturalLanguage";
import { defaultPriorityId } from "./priorities.svelte";
import { dueRelativeDateLabel, scheduledRelativeDateLabel } from "./relativeDates";
import { defaultStatusId } from "./statuses.svelte";
import type { PriorityLevel, StatusDefinition, TaskDefaults } from "./types";

/**
 * Resolves the default tags a new task should get, mirroring
 * `effective_default_tags` in the Rust command layer: the project's default
 * tags if it has any configured, otherwise the global default tags.
 */
export function effectiveDefaultTags(global: string[], project?: string[]): string[] {
  return project && project.length > 0 ? project : global;
}

/**
 * Resolves a default `due`/`scheduled` relative-date code, mirroring
 * `effective_default_code` in the Rust command layer: the project's code if
 * set, otherwise the global code.
 */
export function effectiveDefaultCode(global?: string, project?: string): string | undefined {
  return project ?? global;
}

/**
 * Combines explicitly-requested tags with default tags, appending any
 * default tag not already present, mirroring `merge_tags` in the Rust
 * command layer.
 */
export function mergeTags(explicit: string[], defaults: string[]): string[] {
  const merged = [...explicit];
  for (const tag of defaults) {
    if (!merged.includes(tag)) {
      merged.push(tag);
    }
  }
  return merged;
}

/**
 * Resolves the status a new task should get when none was explicitly
 * requested, mirroring `resolve_default_status` in the Rust command layer:
 * the project board's `default_status` if it names a currently-defined
 * status, otherwise `defaultStatusId`'s own fallback chain (the global
 * default if valid, else the lowest-`order` status, else `"backlog"`).
 * Each candidate is validated independently — an invalid board default
 * falls through to the global default rather than skipping straight to
 * the lowest-order fallback.
 */
export function effectiveDefaultStatus(
  statuses: StatusDefinition[],
  projectBoardDefault: string | undefined,
  globalDefault: string | undefined,
): string {
  if (projectBoardDefault && statuses.some((status) => status.id === projectBoardDefault)) {
    return projectBoardDefault;
  }
  return defaultStatusId(statuses, globalDefault);
}

/**
 * The effective project, priority, tags, due, and scheduled a new task will
 * be created with, for display in `AddTaskModal` before the task is saved.
 *
 * `due`/`scheduled` are display labels: either the absolute date typed via a
 * `due:`/`sch:` quick-add token, "Never" for the `due:na`/`due na`
 * never-due token, or the human-readable label for a resolved relative-date
 * default (e.g. "Tomorrow"). The backend resolves relative-date defaults to
 * an absolute date (or no date, for "none") at save time.
 */
export interface TaskPreview {
  project: string;
  priorityId: string;
  statusId: string;
  tags: string[];
  due?: string;
  scheduled?: string;
}

export interface ResolveTaskPreviewOptions {
  parsed: ParsedTaskInput;
  /** The project of the board the add-task modal was opened from, if any. */
  projectFilter?: string;
  /** `Settings.default_project`, used when no project was specified or implied. */
  defaultProjectName: string;
  globalDefaults: TaskDefaults;
  projectDefaults?: TaskDefaults;
  /**
   * The canonical name of the project matched (case-insensitively, mirroring
   * `find_project` in the Rust command layer) against `projectDefaults`'
   * source project, if any. Used so the preview shows the project's stored
   * casing rather than whatever casing the user typed via `+project`.
   */
  matchedProjectName?: string;
  priorities: PriorityLevel[];
  statuses: StatusDefinition[];
  /** The matched project's `board.default_status`, if any. */
  projectBoardDefaultStatus?: string;
}

/**
 * Resolves the `TaskPreview` for the current quick-add input, mirroring the
 * defaults-resolution pipeline in `commands.rs` (`resolve_default_priority`,
 * `resolve_default_status`, `effective_default_tags`, `effective_default_code`,
 * `merge_tags`):
 *
 * - `project`: the matched project's canonical name if one was found, else
 *   the `+Project` quick-add token, else the board's project filter, else
 *   `defaultProjectName`.
 * - `priorityId`: the `!priority` quick-add token, else the resolved default
 *   priority.
 * - `statusId`: the `@status` quick-add token, else the resolved default
 *   status (project board default, else global default, else lowest order).
 * - `tags`: quick-add `#tag` tokens merged with the effective default tags
 *   (project defaults override global defaults when non-empty).
 * - `due`/`scheduled`: the quick-add `due:`/`sch:` token (`"Never"` for the
 *   `due:na`/`due na` never-due token), else the label for the effective
 *   default relative-date code (project overrides global).
 */
export function resolveTaskPreview(options: ResolveTaskPreviewOptions): TaskPreview {
  const {
    parsed,
    projectFilter,
    defaultProjectName,
    globalDefaults,
    projectDefaults,
    matchedProjectName,
    priorities,
    statuses,
    projectBoardDefaultStatus,
  } = options;

  const project = matchedProjectName ?? parsed.project ?? projectFilter ?? defaultProjectName;
  const priorityId = parsed.priority ?? defaultPriorityId(priorities, globalDefaults.priority);
  const statusId =
    parsed.status ?? effectiveDefaultStatus(statuses, projectBoardDefaultStatus, globalDefaults.status);
  const tags = mergeTags(parsed.tags, effectiveDefaultTags(globalDefaults.tags, projectDefaults?.tags));

  const dueCode = effectiveDefaultCode(globalDefaults.due, projectDefaults?.due);
  const scheduledCode = effectiveDefaultCode(globalDefaults.scheduled, projectDefaults?.scheduled);

  const due =
    parsed.due === "none" ? "Never" : parsed.due ?? (dueCode ? dueRelativeDateLabel(dueCode) : undefined);

  return {
    project,
    priorityId,
    statusId,
    tags,
    due,
    scheduled: parsed.scheduled ?? (scheduledCode ? scheduledRelativeDateLabel(scheduledCode) : undefined),
  };
}
