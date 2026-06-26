import type { InkMode } from "./colorPresets";
import type { DueRule, RecurrenceFrequency } from "./recurrence";
import type { CardTrackedTimeDisplay } from "./tracking.svelte";

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
  /**
   * Per-slot override of `Settings.default_status_tier_rules` for this
   * project's status-line health badge: always exactly 4 entries aligned to
   * `[severe, critical, needs_attention, on_track]` when set, but each slot
   * independently inherits the matching global tier when `null`. `undefined`
   * (the whole field) inherits every tier from the global default. Read-only
   * in Milestone 3 — editing this is Milestone 4's layout/tier-rule UI.
   */
  status_tier_rule_overrides?: (StatusTierRule | null)[];
  /** Overrides `Settings.default_status_line_layout_id` for which `StatLayout` this project's status line renders. `undefined` inherits the global default. */
  status_line_layout_id?: string;
  /** Overrides `Settings.default_dashboard_layout_id` for which dashboard `StatLayout` this project's dashboard renders. `undefined` inherits the global default. */
  dashboard_layout_id?: string;
  /**
   * 3-state status-bar override for this project: `undefined` inherits the
   * global `Settings.status_bar_enabled` value; `true` forces the bar on even
   * when the global default is off; `false` forces it off.
   */
  status_bar_enabled_override?: boolean;
}

/**
 * One status-line health tier's condition set, mirroring `StatusTierRule` in
 * `src-tauri/src/settings.rs` exactly. Every condition the tier has set must
 * match for the tier to match (AND); an unset field is simply skipped for
 * that tier. See `docs/features/project-status-line.md`'s "Status algorithm".
 */
export interface StatusTierRule {
  /** Matches if any of the project's own qualifying tasks has a `due` date `<= today + due_within_days`. Zero or negative catches overdue/due-today. */
  due_within_days?: number;
  /** Matches if any qualifying task's priority has a `rank` at least as severe as (`<=`) this `PriorityLevel.id`'s rank. */
  min_priority?: string;
  /** Matches if the project's own already-computed `estimated_time_left` stat is strictly greater than this many minutes. */
  estimated_time_left_exceeds_minutes?: number;
}

/**
 * A shared layout entity (status lines today, dashboards in Phase 3),
 * mirroring `StatLayout` in `src-tauri/src/layout.rs` exactly. Editing a
 * layout mutates it in place — every project (or the global default)
 * currently pointing at `id` sees the change immediately; `duplicateStatusLayout`
 * forks a new one instead. `kind` is `"status_line"` for every layout this
 * milestone reads/renders; `"dashboard"` is reserved for Phase 3.
 */
export interface StatLayout {
  id: string;
  name: string;
  kind: "status_line" | "dashboard";
  /** Ordered ids of the stats currently shown — see `ProjectStatusStats`' field names for the valid stat ids, plus `"status_badge"`. */
  stat_ids: string[];
  /** Per-widget width override: `"half"` = one column, `"full"` = full row. Absent keys fall back to `"half"`. Only meaningful for `kind === "dashboard"` layouts. */
  widget_widths?: Record<string, "half" | "full">;
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
  /**
   * Whether a card's *live* ticker (while its timer is running) shows the
   * cumulative tracked total (`"total"`, the default) or just the current
   * session's own elapsed time, restarting from `0:00` on every resume
   * (`"session"`). The static chip shown once a timer is stopped always
   * shows the lifetime total either way. See `liveDisplaySecondsFor`.
   */
  card_tracked_time_display: CardTrackedTimeDisplay;
  /** Global tier-rule thresholds for the status-line health badge, exactly 4 entries in `[severe, critical, needs_attention, on_track]` order. See `ProjectBoard.status_tier_rule_overrides` for the per-project per-slot override. */
  default_status_tier_rules: StatusTierRule[];
  /** How many trailing complete weeks the `avg_time_per_week` status-line stat averages over. No per-project override. */
  avg_time_per_week_window: number;
  /** The `StatLayout.id` the status line renders when a project hasn't set `ProjectBoard.status_line_layout_id`. */
  default_status_line_layout_id: string;
  /** The `StatLayout.id` (of `kind === "dashboard"`) rendered when a project hasn't set `ProjectBoard.dashboard_layout_id`. */
  default_dashboard_layout_id: string;
  /** Only `"tiles"` remains after "chips" and "tint" were removed. Kept as a field for future extension but effectively always `"tiles"`. */
  status_bar_style: "tiles";
  /** Global on/off switch for the project status bar. `true` by default. Per-project override via `ProjectBoard.status_bar_enabled_override`. */
  status_bar_enabled: boolean;
  /** When `true`, status-line tiles show a tinted background keyed to the project's current `StatusTier` color. `false` by default. */
  status_bar_tile_tint: boolean;
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

/** The 4 real status-line health tiers plus the implicit `"great"` fallback — see `docs/features/project-status-line.md`'s "Status algorithm". Most-severe-first. */
export type StatusTier = "severe" | "critical" | "needs_attention" | "on_track" | "great";

/**
 * All 6 project-status-line stats for one project, mirroring
 * `ProjectStatusStats` in `src-tauri/src/commands.rs` exactly.
 * `estimated_time_left`/`total_time_tracked` are both in **minutes**;
 * `avg_time_per_week` is in **seconds** (the backend's native unit — see its
 * own Rust doc comment for why this one stat alone stays unconverted).
 * `completion_pct`/`weighted_completion_pct` are fractions in `0.0..=1.0`,
 * `undefined` when there's no meaningful population to divide by (distinct
 * from `0`, which means "a real population, all incomplete"). `effective_layout_id`
 * is the resolved `StatLayout.id` this project's status line should render.
 */
export interface ProjectStatusStats {
  status_tier: StatusTier;
  estimated_time_left: number;
  total_time_tracked: number;
  avg_time_per_week: number;
  completion_pct?: number;
  weighted_completion_pct?: number;
  /** Same calculation as `completion_pct` but restricted to active (non-archived) tasks — "what fraction of tasks on the kanban board are done?". */
  active_completion_pct?: number;
  effective_layout_id: string;
}

/**
 * Global stats for the "All tasks" status bar, mirroring `GlobalStatusStats`
 * in `src-tauri/src/commands.rs` exactly. `tasks_by_status` contains only
 * statuses that have at least one visible (non-hidden, active) task.
 * `time_tracked_today_minutes` and `time_tracked_this_week_minutes` are in
 * **minutes** and cover all tasks in the time database, not filtered by
 * project.
 */
export interface GlobalStatusStats {
  tasks_by_status: [string, number][];
  total_projects: number;
  time_tracked_today_minutes: number;
  time_tracked_this_week_minutes: number;
}

// ── Dashboard API response types ──────────────────────────────────────────────

/** One bar in a "time by project" or "time by tag" chart. */
export interface DashboardTimeEntry {
  label: string;
  minutes: number;
}

/** One row in the "estimated vs. actual" chart. */
export interface DashboardEstVsActual {
  project_name: string;
  estimated_minutes: number;
  actual_minutes: number;
}

/** One series (status label + count) inside a completion-trend week. */
export interface DashboardCompletionSeries {
  label: string;
  count: number;
}

/** One week's data in the completion-trend chart. */
export interface DashboardCompletionWeek {
  week_label: string;
  series: DashboardCompletionSeries[];
}

/** One status bucket in the status-distribution chart. */
export interface DashboardStatusCount {
  status_id: string;
  count: number;
}

/** One bucket (day-of-week or hour-of-day) in the busy-histogram. */
export interface DashboardBucket {
  index: number;
  minutes: number;
}

/** Both axes of the busy-histogram: one entry per day of week, one per hour of day. */
export interface DashboardBusyHistogram {
  days: DashboardBucket[];
  hours: DashboardBucket[];
}
