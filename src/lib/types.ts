import type { InkMode } from "./colorPresets";
import type { DueRule, RecurrenceFrequency } from "./recurrence";

export interface Task {
  id: string;
  title: string;
  /** The id of a user-defined `StatusDefinition` (see `Settings.statuses`). */
  status: string;
  /** The id of the `Project` this task belongs to. */
  project_id?: string;
  tags: string[];
  /** The id of a user-defined `PriorityLevel` (see `Settings.priorities`). */
  priority: string;
  due?: string;
  scheduled?: string;
  order: number;
  created: string;
  depends_on: string[];
  /** User-editable estimate of how long this task will take, in minutes. `undefined` means no estimate has been set. */
  estimated_minutes?: number;
  /** Total time tracked against this task so far, in minutes. Always present; not user-editable. */
  tracked_minutes: number;
  /** The id of the `Series` this task was generated from, if any. `undefined` for a normal, non-recurring task. */
  series_id?: string;
  /**
   * The id of this task's auto-generated "subtask container" `Project`, if
   * it has ever had a subtask. `undefined` until the first subtask is
   * created, and reset back to `undefined` when the container becomes
   * empty and is cleaned up. The container itself stores no back-pointer
   * to this task — see `subtasks.ts`'s `containerOwner` for the reverse
   * lookup.
   */
  subtask_project_id?: string;
  /**
   * Marks this task as the hidden time-tracking anchor for a `Project` (see
   * `Project.tracking_task_id`) — set once when that project's
   * lazily-created tracker task is generated, and never toggled afterward.
   * `false` for every ordinary task. Hidden tasks are excluded from every
   * kanban/week/calendar view and task picker, but otherwise remain normal
   * addressable tasks.
   */
  hidden: boolean;
  notes: string;
}

/**
 * A project's Kanban board configuration: the subset and order of the
 * global status list (see `Settings.statuses`) shown on this project's
 * board, and the status new tasks in this project get by default. An empty
 * `statuses` list means the project hasn't customized its board and shows
 * the global status list as-is; an unset `default_status` falls back to the
 * global `Settings.defaults.status`.
 *
 * `show_previous_weeks` overrides `Settings.show_previous_weeks_column` for
 * this project's Week view when set; `undefined` inherits the global default.
 */
export interface ProjectBoard {
  statuses: string[];
  default_status?: string;
  show_previous_weeks?: boolean;
  /** Overrides `Settings.card_lightness` for this project's Kanban cards. `undefined` inherits the global default. */
  card_lightness?: number;
  /** Overrides `Settings.bar_lightness` for this project's week/calendar-view bars. `undefined` inherits the global default. */
  bar_lightness?: number;
  /** Overrides `Settings.ink_mode` for this project's color-coded card/bar text. `undefined` inherits the global default. */
  ink_mode?: InkMode;
  /** Overrides `Settings.show_subproject_tasks_default` for whether viewing this project's board/week/calendar rolls up its descendant subprojects' tasks too. `undefined` inherits the global default. */
  show_subproject_tasks?: boolean;
}

/**
 * Default task attributes. Used both as the global defaults and as a
 * project's per-field overrides of those global defaults: any field left
 * unset/empty here falls back to the corresponding global value.
 *
 * `priority` and `status` are `string` because they reference the ids of
 * user-defined `PriorityLevel`/`StatusDefinition` entries in `Settings`
 * rather than a fixed built-in union.
 *
 * `scheduled`, if set, must be one of the option ids in
 * `SCHEDULED_RELATIVE_DATE_OPTIONS` (see `relativeDates.ts`) rather than an
 * absolute date: it's resolved to an absolute date relative to "today" at
 * task-creation time. The global default is always set, since every task
 * must have a scheduled date.
 *
 * `due`, if set, must be one of the option ids in `DUE_RELATIVE_DATE_OPTIONS`
 * (see `relativeDates.ts`) rather than an absolute date: it's resolved to an
 * absolute date relative to the task's *scheduled* date (not "today") at
 * task-creation time. `"none"` means "never due".
 *
 * `estimated_minutes`, if set, seeds `Task.estimated_minutes` for a newly
 * created task that doesn't specify its own estimate.
 */
export interface TaskDefaults {
  tags: string[];
  priority?: string;
  status?: string;
  due?: string;
  scheduled?: string;
  estimated_minutes?: number;
}

export interface Project {
  id: string;
  name: string;
  color: string;
  /** The id of this project's parent, or `undefined` for a top-level project. Nesting is arbitrary depth. */
  parent_id?: string;
  order: number;
  created: string;
  board: ProjectBoard;
  defaults: TaskDefaults;
  /**
   * The id of this project's lazily-created hidden tracker `Task` (see
   * `Task.hidden`), used to "track this project as a whole" rather than any
   * single task within it. `undefined` until the project's own play button
   * is pressed for the first time, and otherwise only ever set once.
   */
  tracking_task_id?: string;
}

/** Matches `DEFAULT_PROJECT_COLOR` in `src-tauri/src/project.rs`. */
export const DEFAULT_PROJECT_COLOR = "#3b82f6";

/**
 * A recurring task's template and rule, mirroring `Series` in
 * `src-tauri/src/series.rs` exactly. Only ever fetched via `getSeries` (to
 * pre-fill the recurrence builder when editing an existing recurring
 * task) — the app otherwise interacts with recurrence entirely through
 * `Task.series_id` and the dedicated series commands, never by reading or
 * writing this shape directly elsewhere.
 */
export interface Series {
  id: string;
  frequency: RecurrenceFrequency;
  anchor_date: string;
  end_date?: string;
  due_rule: DueRule;
  generated_until: string;
  active: boolean;
  title: string;
  project_id?: string;
  priority: string;
  tags: string[];
  estimated_minutes?: number;
  notes: string;
  created: string;
}

/**
 * A user-defined priority level: an id stored in `Task.priority`, a display
 * label, a `color` used to render that priority throughout the UI, and a
 * `rank` used to sort tasks by priority (lower `rank` sorts first / is
 * considered higher priority).
 */
export interface PriorityLevel {
  id: string;
  label: string;
  color: string;
  rank: number;
}

/**
 * A user-defined task status: an id stored in `Task.status`, a display
 * label, `order`, its position in the global status list, and a `color`
 * used to style Kanban columns for this status throughout the UI.
 */
export interface StatusDefinition {
  id: string;
  label: string;
  order: number;
  color: string;
}

/**
 * Global, app-wide settings: the available priority levels, the global list
 * of statuses (from which each project's board is configured), and the
 * global default task attributes.
 *
 * `done_status` and `cancelled_status` mark which entries in `statuses`
 * represent a task being finished or abandoned. Exactly one status is always
 * the done status; the cancelled status is optional and, if set, differs from
 * the done status.
 *
 * `default_project_id` is the id of the project a new task is filed under
 * when no project was specified (and no project-scoped board supplied one);
 * the backend never creates or saves a task with an empty/missing project.
 */
export interface Settings {
  priorities: PriorityLevel[];
  statuses: StatusDefinition[];
  defaults: TaskDefaults;
  done_status: string;
  cancelled_status?: string;
  default_project_id: string;
  /** Global default for whether Week view shows a "previous weeks" column. See `ProjectBoard.show_previous_weeks`. */
  show_previous_weeks_column: boolean;
  /** Global OKLCH lightness for "color code" mode's Kanban card background. See `ProjectBoard.card_lightness`. */
  card_lightness: number;
  /** Global OKLCH lightness for "color code" mode's week/calendar-view bar background. See `ProjectBoard.bar_lightness`. */
  bar_lightness: number;
  /** Global default text-color mode for "color code" mode's card/bar text. See `ProjectBoard.ink_mode`. */
  ink_mode: InkMode;
  /** Global default for whether viewing a project's board/week/calendar rolls up its descendant subprojects' tasks too. See `ProjectBoard.show_subproject_tasks`. */
  show_subproject_tasks_default: boolean;
  /** Whether a task with subtasks' displayed estimated time adds its own `estimated_minutes` on top of its subtasks' total (`true`) or is replaced by that total entirely (`false`, the default). Display-only — never written back to any stored field. See `effectiveEstimatedMinutes`. */
  parent_estimate_includes_own_value: boolean;
  /** How many subtask rows a parent card's nested preview shows before collapsing the rest into a "+N more" line. */
  max_visible_subtasks: number;
  /**
   * Whether starting a task's timer should automatically move it to
   * `tracking_auto_transition_status_id`. Defaults to `false` (no automatic
   * status change).
   */
  tracking_auto_transition_enabled: boolean;
  /**
   * The status a task's timer auto-transitions it to when
   * `tracking_auto_transition_enabled` is `true`. `undefined` when enabled
   * but no status has been chosen yet, in which case the frontend falls
   * back at runtime to the first status in the global status list that
   * isn't backlog/done/cancelled.
   */
  tracking_auto_transition_status_id?: string;
}

/**
 * What to do with a project's existing tasks when the project is deleted
 * (see `deleteProject`). Matches `ProjectTaskStrategy` in
 * `src-tauri/src/commands.rs`.
 */
export type ProjectTaskStrategy =
  | { type: "reassign"; target_project_id: string }
  | { type: "archive" }
  | { type: "delete" };

/** Result of `deleteProject`: how many of the project's tasks were affected. */
export interface DeleteProjectResult {
  affected_tasks: number;
}

/** Result of `finishDay`: how many tasks were archived. */
export interface FinishDayResult {
  archived_count: number;
}

/**
 * A single time-tracking session against a task, mirroring `TimeEntry` in
 * `src-tauri/src/time_storage.rs` exactly. `ended_at: null` means the
 * session is currently running — at most one such row may exist per
 * `task_id` at a time. `last_heartbeat_at` is updated periodically while a
 * session is running (see the time-tracking-engine spec's "Heartbeat"
 * section) and is otherwise `null`.
 */
export interface TimeEntry {
  id: string;
  task_id: string;
  started_at: string;
  ended_at: string | null;
  last_heartbeat_at: string | null;
  created_at: string;
}
