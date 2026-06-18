export interface Task {
  id: string;
  title: string;
  /** The id of a user-defined `StatusDefinition` (see `Settings.statuses`). */
  status: string;
  project?: string;
  tags: string[];
  /** The id of a user-defined `PriorityLevel` (see `Settings.priorities`). */
  priority: string;
  due?: string;
  scheduled?: string;
  order: number;
  created: string;
  depends_on: string[];
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
 */
export interface TaskDefaults {
  tags: string[];
  priority?: string;
  status?: string;
  due?: string;
  scheduled?: string;
}

export interface Project {
  id: string;
  name: string;
  color: string;
  order: number;
  created: string;
  board: ProjectBoard;
  defaults: TaskDefaults;
}

/** Matches `DEFAULT_PROJECT_COLOR` in `src-tauri/src/project.rs`. */
export const DEFAULT_PROJECT_COLOR = "#3b82f6";

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
 * `default_project` names the project a new task is filed under when no
 * project was specified (and no project-scoped board supplied one); the
 * backend never creates or saves a task with an empty/missing project.
 */
export interface Settings {
  priorities: PriorityLevel[];
  statuses: StatusDefinition[];
  defaults: TaskDefaults;
  done_status: string;
  cancelled_status?: string;
  default_project: string;
  /** Global default for whether Week view shows a "previous weeks" column. See `ProjectBoard.show_previous_weeks`. */
  show_previous_weeks_column: boolean;
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
