/**
 * Central catalog of every dashboard widget's user-facing metadata: title,
 * date-range behavior, and the plain-language definitions shown in the ⓘ
 * popover. EVERY widget (global + project + future ones) must be registered
 * here — the shared `WidgetHeader` component reads this catalog, and the
 * widget skill mandates registration as part of adding a widget.
 *
 * Definition-text conventions (keep these consistent):
 * - "Completed" always means the Done status — cancelled is never completed.
 * - Say explicitly whether a recurring task counts once ("counts once") or
 *   per finished occurrence ("each finished occurrence counts").
 * - Archived tasks are included everywhere; only call it out where users
 *   would otherwise be surprised.
 * - Subtasks count toward the project of their parent task.
 */

import type { DashboardDateRange } from "$lib/api";
import type { DashboardWidget } from "$lib/types";

export type WidgetTypeId = DashboardWidget["widget_type"];

/** The one shared label set for the dashboard date-range pickers. */
export const DATE_RANGE_LABELS: Record<DashboardDateRange, string> = {
  last_7_days: "Last 7 days",
  last_30_days: "Last 30 days",
  this_month: "This month",
  last_3_months: "Last 3 months",
  all_time: "All time",
};

/** How a widget relates to the dashboard's date-range picker. */
export type WidgetRangeMode =
  /** Follows the dashboard picker — badge shows the selected range. */
  | { kind: "picker" }
  /** Deliberately ignores the picker; always all of history. */
  | { kind: "all_time" }
  /** Deliberately ignores the picker; a snapshot of right now. */
  | { kind: "now" }
  /** Has its own inherent timeline (badge shows a custom label). */
  | { kind: "own"; label: string };

export interface WidgetMeta {
  /** Display title, rendered by WidgetHeader. */
  title: string;
  /** One or two sentences: what the widget shows. */
  what: string;
  /** How the numbers are computed — the standardized definitions. */
  how: string;
  range: WidgetRangeMode;
}

const COMPLETED_OCCURRENCE =
  "Completed = the Done status (cancelled tasks are counted separately, never as completed). " +
  "Each finished occurrence of a recurring task counts.";

const COMPLETED_ENTITY =
  "Completed = the Done status; cancelled is excluded. A recurring task counts once.";

export const WIDGET_CATALOG: Record<WidgetTypeId, WidgetMeta> = {
  // ── Global dashboard ───────────────────────────────────────────────────────
  completion_overview: {
    title: "Completion Overview",
    what: "Tasks you finished (green, right) vs cancelled (amber, left) per top-level project.",
    how:
      COMPLETED_OCCURRENCE +
      " Subproject and subtask work rolls up into its top-level project. Archived tasks are included. " +
      "Ranged views count tasks by their completion date; tasks finished before completion " +
      "timestamps existed only appear in All time.",
    range: { kind: "picker" },
  },
  project_scale: {
    title: "Project Time & Scale",
    what: "How big each project is: estimated time, tracked time, and task count, per top-level project.",
    how:
      "Tracked time comes from the time log inside the selected range. Task count and estimates use tasks " +
      "whose due/scheduled/created date falls in the range (a recurring task counts once). " +
      "Subprojects and subtasks roll up. Archived tasks are included.",
    range: { kind: "picker" },
  },
  status_by_project: {
    title: "Status by Project",
    what: "Each project's tasks broken down by status, as a proportional bar.",
    how:
      "A recurring task counts once. Ranged views select tasks by due → scheduled → created date; " +
      "All time shows the board's current view (future-scheduled tasks excluded). " +
      "Subprojects and subtasks roll up into their top-level project.",
    range: { kind: "picker" },
  },
  project_health: {
    title: "Project Health",
    what: "Every project's health tier with tasks due today, tomorrow, and estimated work left.",
    how:
      "The tier is evaluated with the same rules and the same task pool as the project's status-bar badge " +
      "(your per-project rule overrides apply). DUE counts open tasks whose due date is today/tomorrow. " +
      "LEFT sums estimate minus tracked for open scheduled tasks.",
    range: { kind: "now" },
  },
  productivity: {
    title: "Productivity",
    what: "Tracked time per day, stacked by project, with the overall trend.",
    how:
      "Time comes from the tracking log, split by calendar day inside the selected range. " +
      "Subproject, subtask, and project-timer time all count toward their project.",
    range: { kind: "picker" },
  },

  // ── Project dashboard ──────────────────────────────────────────────────────
  p_scoreboard: {
    title: "Scoreboard",
    what: "The project's four key numbers: done, left, time tracked, and estimated time remaining.",
    how:
      "DONE and TRACKED follow the selected range (" +
      COMPLETED_OCCURRENCE.toLowerCase() +
      ") LEFT and REMAINING are the current state: open tasks (a series counts once) and " +
      "estimate-minus-tracked for open scheduled tasks. Subtasks and archived tasks are included.",
    range: { kind: "picker" },
  },
  p_health_pulse: {
    title: "Health Pulse",
    what: "The project's current health tier, plus open tasks due today and tomorrow.",
    how:
      "Same rules and task pool as the status-bar badge (your per-project overrides apply). " +
      "Due counts include only open (not done/cancelled) tasks.",
    range: { kind: "now" },
  },
  p_velocity: {
    title: "Velocity",
    what: "How many tasks you finish per week, the trend vs the previous month, and what's due next week.",
    how:
      COMPLETED_OCCURRENCE +
      " Averaged over the last 4 weeks; the arrow compares against the 4 weeks before that. " +
      "“Due next wk” counts open tasks due or scheduled in the coming 7 days.",
    range: { kind: "own", label: "LAST 4 WKS" },
  },
  p_completion_dial: {
    title: "Progress",
    what: "How much of the project is done — by task count, and weighted by estimated time.",
    how:
      COMPLETED_ENTITY +
      " Count progress = done ÷ (done + open). Time-weighted progress = estimated minutes of done tasks ÷ " +
      "all estimated minutes. Subtasks and archived tasks are included.",
    range: { kind: "now" },
  },
  p_fuel_gauge: {
    title: "Work Remaining",
    what: "How much estimated work is left in the tank, out of everything estimated.",
    how:
      "Remaining = estimate minus tracked time, summed over open scheduled tasks. " +
      "The total is every estimated task (a series counts once). A full tank of to-do glows red " +
      "and cools to green as you burn it down — near-empty is the goal.",
    range: { kind: "now" },
  },
  p_effort_balance: {
    title: "Estimate vs Actual",
    what: "Total time you planned vs total time you actually tracked.",
    how:
      "Estimated sums every task's estimate (a series counts once). Tracked sums all logged time — " +
      "every occurrence, subtasks, and the project timer. All-time by design: estimates aren't tied to dates.",
    range: { kind: "all_time" },
  },
  p_weekly_rhythm: {
    title: "Weekly Rhythm",
    what: "Which weekdays you actually work on this project.",
    how:
      "Average tracked hours per weekday, from the time log inside the selected range. " +
      "Today's bar is highlighted.",
    range: { kind: "picker" },
  },
  p_time_donut: {
    title: "Time Breakdown",
    what: "Where this project's tracked time goes: per subproject, or per tag if there are no subprojects.",
    how:
      "Each slice is a direct subproject (including its own subtree); time logged directly on this project " +
      "gets its own slice, so slices always sum to the center total. Ranged views read the time log.",
    range: { kind: "picker" },
  },
  p_status_radial: {
    title: "Status Radial",
    what: "The project's open and finished tasks by status, as radial wedges.",
    how:
      "A recurring task counts once. Archived tasks are included, so Done grows over the project's life. " +
      "Wedge angle is proportional to task count.",
    range: { kind: "now" },
  },
  p_due_timeline: {
    title: "Upcoming & Overdue",
    what: "What's overdue and what's coming: today, tomorrow, this week, next week, later.",
    how:
      "Buckets count open (not done/cancelled) tasks by due date; every occurrence of a recurring task " +
      "is its own entry. Overdue = due date in the past and still open. The project deadline shows as a marker.",
    range: { kind: "now" },
  },
  p_burndown: {
    title: "Burndown",
    what: "Estimated hours of work remaining over time, against the ideal pace to your deadline.",
    how:
      "One-off tasks with estimates only (recurring tasks regenerate forever, so they can't burn down). " +
      "The line drops when estimated tasks are completed. Start = first task created; " +
      "end = project deadline, or the latest due date without one.",
    range: { kind: "own", label: "PROJECT LIFE" },
  },
  p_completion_trend: {
    title: "Completion Trend",
    what: "Tasks finished per week — is the pace speeding up or slowing down?",
    how:
      COMPLETED_OCCURRENCE +
      " Weeks start on Monday; the current (partial) week is marked. The number of weeks follows " +
      "the selected range (12 for All time).",
    range: { kind: "picker" },
  },
  p_subproject_tree: {
    title: "Project Map",
    what: "The whole project as a treemap — every subproject's size, progress, and health in one picture.",
    how:
      "Tile area = task count (a series counts once; subtasks roll up); nesting shows the subproject " +
      "hierarchy, and each project's own tasks appear as a dashed “Direct” tile. The bottom bar is " +
      "completion; the corner dot is the health tier, evaluated with the same rules as the status bar.",
    range: { kind: "now" },
  },
  p_subproject_bars: {
    title: "Subproject Progress",
    what: "Every subproject's completion, sorted by how urgently it needs attention.",
    how:
      COMPLETED_ENTITY +
      " Progress = done ÷ all tasks of that subproject. Grandchildren are indented and slightly faded; " +
      "fully done subprojects are dimmed.",
    range: { kind: "now" },
  },
  p_subproject_sunburst: {
    title: "Subproject Comparison",
    what: "How tracked time (or task count) is distributed across the subproject hierarchy.",
    how:
      "Inner ring = direct subprojects, outer ring = their children. Slice size = tracked time " +
      "(or task count, per the widget's setting). Subtask time rolls up; containers are not subprojects.",
    range: { kind: "all_time" },
  },
};

/** Resolves the badge text for a widget: pickers show the currently selected
 *  range, the rest are fixed. */
export function rangeBadgeText(meta: WidgetMeta, pickerRange?: DashboardDateRange): string {
  switch (meta.range.kind) {
    case "picker":
      return (pickerRange ? DATE_RANGE_LABELS[pickerRange] : "range").toUpperCase();
    case "all_time":
      return "ALL TIME";
    case "now":
      return "NOW";
    case "own":
      return meta.range.label;
  }
}
