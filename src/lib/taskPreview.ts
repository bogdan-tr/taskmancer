import type { ParsedTaskInput } from "./naturalLanguage";
import { defaultPriorityId } from "./priorities.svelte";
import { resolveDueRule } from "./recurrence";
import {
  resolveDueRelativeDate,
  resolveScheduledRelativeDate,
  scheduledRelativeDateLabel,
} from "./relativeDates";
import { defaultStatusId } from "./statuses.svelte";
import type { PriorityLevel, StatusDefinition, TaskDefaults } from "./types";
import { formatDateISO } from "./weekRange";

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
 * `scheduled` is a display label: either the absolute date typed via a
 * `sch:` quick-add token, or the human-readable label for a resolved
 * relative-date default (e.g. "Tomorrow").
 *
 * `due` is always an absolute `YYYY-MM-DD` date (or "Never"), never a
 * generic label: either the date typed via a `due:` quick-add token, or a
 * default due code *actually resolved* relative to the effective scheduled
 * date above (mirroring `resolve_creation_defaults` in the Rust command
 * layer) — not just the code's static label, which previously caused the
 * preview to mis-render any default-resolved due date as "Due today"
 * (`formatDueDateDisplay` silently treats an unparseable label as "0 days
 * away"). "Never" covers both the `due:na`/`due na` token and a default due
 * code of `"none"`.
 */
export interface TaskPreview {
  project: string;
  priorityId: string;
  statusId: string;
  tags: string[];
  due?: string;
  scheduled?: string;
  /**
   * The same resolution as `scheduled`, but always an absolute `YYYY-MM-DD`
   * date, never a label — for UI that needs a real date to anchor against
   * (e.g. highlighting the right day in a calendar-popup date picker),
   * where `scheduled`'s human-readable label (e.g. "Tomorrow") isn't usable.
   */
  scheduledDate: string;
  estimatedMinutes?: number;
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
  /** "Now", for resolving relative-date defaults. Defaults to `new Date()`. */
  now?: Date;
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
 * - `scheduled`: the quick-add `sch:` token, else the label for the
 *   effective default relative-date code (project overrides global).
 * - `scheduledDate`: the same resolution as `scheduled`, but always resolved
 *   to an absolute date rather than a label — see [`TaskPreview`].
 * - `due`: the quick-add `due:` token, `"Never"` for the `due:na`/`due na`
 *   never-due token, else (if a recurring-due-rule phrase like `due in 5
 *   days`/`due mondays` was used) that rule resolved against the effective
 *   scheduled date, else a default due code of `"none"` as `"Never"`, else
 *   the effective default due code resolved to an absolute date relative to
 *   the *effective scheduled date* above (not "today" — see
 *   [`TaskPreview`]).
 * - `estimatedMinutes`: the quick-add `est`/bare duration token, else the
 *   project's `TaskDefaults.estimated_minutes` if set, else the global
 *   default, mirroring `effective_default_estimated_minutes` in the Rust
 *   command layer.
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
    now = new Date(),
  } = options;

  const project = matchedProjectName ?? parsed.project ?? projectFilter ?? defaultProjectName;
  const priorityId = parsed.priority ?? defaultPriorityId(priorities, globalDefaults.priority);
  const statusId =
    parsed.status ?? effectiveDefaultStatus(statuses, projectBoardDefaultStatus, globalDefaults.status);
  const tags = mergeTags(parsed.tags, effectiveDefaultTags(globalDefaults.tags, projectDefaults?.tags));

  const dueCode = effectiveDefaultCode(globalDefaults.due, projectDefaults?.due);
  const scheduledCode = effectiveDefaultCode(globalDefaults.scheduled, projectDefaults?.scheduled);

  // The scheduled date this task would actually get, mirroring
  // `resolve_creation_defaults`'s `resolved_scheduled` — needed as the
  // anchor for resolving a default due code below, since due defaults
  // resolve relative to scheduled, not to `now`.
  const resolvedScheduledDate =
    parsed.scheduled ??
    (scheduledCode ? resolveScheduledRelativeDate(scheduledCode, now) : undefined) ??
    formatDateISO(now);

  const due =
    parsed.due === "none"
      ? "Never"
      : parsed.due ??
        (parsed.dueRule
          ? resolveDueRule(parsed.dueRule, resolvedScheduledDate) ?? "Never"
          : dueCode === "none"
            ? "Never"
            : dueCode
              ? resolveDueRelativeDate(dueCode, resolvedScheduledDate)
              : undefined);

  const estimatedMinutes = parsed.estimatedMinutes ?? projectDefaults?.estimated_minutes ?? globalDefaults.estimated_minutes;

  return {
    project,
    priorityId,
    statusId,
    tags,
    due,
    scheduled: parsed.scheduled ?? (scheduledCode ? scheduledRelativeDateLabel(scheduledCode) : undefined),
    scheduledDate: resolvedScheduledDate,
    estimatedMinutes,
  };
}
