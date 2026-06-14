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
 */
export interface ProjectBoard {
  statuses: string[];
  default_status?: string;
}

/**
 * Default task attributes. Used both as the global defaults and as a
 * project's per-field overrides of those global defaults: any field left
 * unset/empty here falls back to the corresponding global value.
 *
 * `priority` and `status` are `string` because they reference the ids of
 * user-defined `PriorityLevel`/`StatusDefinition` entries in `Settings`
 * rather than a fixed built-in union.
 */
export interface TaskDefaults {
  tags: string[];
  priority?: string;
  status?: string;
  due?: string;
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
 */
export interface Settings {
  priorities: PriorityLevel[];
  statuses: StatusDefinition[];
  defaults: TaskDefaults;
}
