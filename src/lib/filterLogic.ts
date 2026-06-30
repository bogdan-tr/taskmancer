import type { FilterConfig, DueDateFilter, PriorityLevel, SortConfig, SortKey, StatusDefinition, Task } from "./types";

/** Applies `config` to `tasks` and returns the matching subset.
 *
 *  Each field that has a non-default value is an "active group". When
 *  `config.groupMode === "all"` (the default), every active group must
 *  match (AND). When `"any"`, at least one must match (OR).
 *  Tasks whose `hidden` flag is set are always excluded.
 */
export function applyFilter(tasks: Task[], config: FilterConfig): Task[] {
  const today = getToday();

  return tasks.filter((task) => {
    if (task.hidden) return false;

    interface Check { active: boolean; passes: boolean }
    const groups: Check[] = [
      // Text search
      config.text.trim()
        ? {
            active: true,
            passes: matchesText(task, config.text.trim(), config.titleOnly),
          }
        : { active: false, passes: true },

      // Status
      config.statuses.length > 0
        ? { active: true, passes: config.statuses.includes(task.status) }
        : { active: false, passes: true },

      // Project
      config.projectIds.length > 0
        ? {
            active: true,
            passes:
              task.project_id !== undefined &&
              config.projectIds.includes(task.project_id),
          }
        : { active: false, passes: true },

      // Tags
      config.tags.length > 0
        ? {
            active: true,
            passes:
              config.tagMode === "all"
                ? config.tags.every((t) => task.tags.includes(t))
                : config.tags.some((t) => task.tags.includes(t)),
          }
        : { active: false, passes: true },

      // Due date
      config.dueFilter.type !== "any"
        ? {
            active: true,
            passes: matchesDateFilter(task.due, config.dueFilter, today),
          }
        : { active: false, passes: true },

      // Scheduled date
      config.scheduledFilter.type !== "any"
        ? {
            active: true,
            passes: matchesDateFilter(
              task.scheduled,
              config.scheduledFilter,
              today,
            ),
          }
        : { active: false, passes: true },

      // Priority
      config.priorities.length > 0
        ? { active: true, passes: config.priorities.includes(task.priority) }
        : { active: false, passes: true },

      // Estimate min
      config.estimateMinHours !== null
        ? {
            active: true,
            passes:
              task.estimated_minutes !== undefined &&
              task.estimated_minutes >= config.estimateMinHours * 60,
          }
        : { active: false, passes: true },

      // Estimate max
      config.estimateMaxHours !== null
        ? {
            active: true,
            passes:
              task.estimated_minutes !== undefined &&
              task.estimated_minutes <= config.estimateMaxHours * 60,
          }
        : { active: false, passes: true },

      // Tracked min
      config.trackedMinHours !== null
        ? {
            active: true,
            passes: task.tracked_minutes >= config.trackedMinHours * 60,
          }
        : { active: false, passes: true },

      // Tracked max
      config.trackedMaxHours !== null
        ? {
            active: true,
            passes: task.tracked_minutes <= config.trackedMaxHours * 60,
          }
        : { active: false, passes: true },

      // Has subtasks
      config.hasSubtasks !== "any"
        ? {
            active: true,
            passes:
              config.hasSubtasks === "yes"
                ? task.subtask_project_id !== undefined
                : task.subtask_project_id === undefined,
          }
        : { active: false, passes: true },

      // Is recurring
      config.isRecurring !== "any"
        ? {
            active: true,
            passes:
              config.isRecurring === "yes"
                ? task.series_id !== undefined
                : task.series_id === undefined,
          }
        : { active: false, passes: true },
    ];

    const active = groups.filter((g) => g.active);
    if (active.length === 0) return true;

    return config.groupMode === "all"
      ? active.every((g) => g.passes)
      : active.some((g) => g.passes);
  });
}

/** Returns a stable sorted copy of `tasks` according to `sort`. */
export function applySort(
  tasks: Task[],
  sort: SortConfig,
  priorities: PriorityLevel[],
  statuses: StatusDefinition[],
): Task[] {
  if (sort.levels.length === 0) return [...tasks];

  const priorityRank = new Map(priorities.map((p) => [p.id, p.rank]));
  const statusOrder = new Map(statuses.map((s) => [s.id, s.order]));

  return [...tasks].sort((a, b) => {
    for (const level of sort.levels) {
      const cmp = compareByKey(a, b, level.key, priorityRank, statusOrder);
      if (cmp !== 0) return level.direction === "asc" ? cmp : -cmp;
    }
    return a.order - b.order; // stable tie-break
  });
}

/** Returns `true` if `config` has at least one active (non-default) field. */
export function isFilterActive(config: FilterConfig): boolean {
  return (
    config.text.trim() !== "" ||
    config.statuses.length > 0 ||
    config.projectIds.length > 0 ||
    config.tags.length > 0 ||
    config.dueFilter.type !== "any" ||
    config.scheduledFilter.type !== "any" ||
    config.priorities.length > 0 ||
    config.estimateMinHours !== null ||
    config.estimateMaxHours !== null ||
    config.trackedMinHours !== null ||
    config.trackedMaxHours !== null ||
    config.hasSubtasks !== "any" ||
    config.isRecurring !== "any" ||
    config.isArchived !== "active_only"
  );
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function matchesText(task: Task, text: string, titleOnly: boolean): boolean {
  const q = text.toLowerCase();
  if (task.title.toLowerCase().includes(q)) return true;
  if (!titleOnly && task.notes.toLowerCase().includes(q)) return true;
  return false;
}

function matchesDateFilter(
  dateStr: string | undefined,
  filter: DueDateFilter,
  today: string,
): boolean {
  if (filter.type === "any") return true;
  if (!dateStr) return false;
  const date = dateStr.slice(0, 10);

  switch (filter.type) {
    case "overdue":
      return date < today;
    case "today":
      return date === today;
    case "this_week": {
      const start = weekStart(today);
      const end = weekEnd(today);
      return date >= start && date <= end;
    }
    case "this_month": {
      const prefix = today.slice(0, 7);
      return date.startsWith(prefix);
    }
    case "custom": {
      if (filter.from && date < filter.from) return false;
      if (filter.to && date > filter.to) return false;
      return true;
    }
  }
}

function compareByKey(
  a: Task,
  b: Task,
  key: SortKey,
  priorityRank: Map<string, number>,
  statusOrder: Map<string, number>,
): number {
  switch (key) {
    case "title":
      return a.title.localeCompare(b.title);
    case "created":
      return a.created.localeCompare(b.created);
    case "due":
      return compareNullable(a.due, b.due);
    case "scheduled":
      return compareNullable(a.scheduled, b.scheduled);
    case "priority": {
      const ra = priorityRank.get(a.priority) ?? 999;
      const rb = priorityRank.get(b.priority) ?? 999;
      return ra - rb;
    }
    case "status": {
      const oa = statusOrder.get(a.status) ?? 999;
      const ob = statusOrder.get(b.status) ?? 999;
      return oa - ob;
    }
    case "estimated_time": {
      const ea = a.estimated_minutes ?? 0;
      const eb = b.estimated_minutes ?? 0;
      return ea - eb;
    }
    case "tracked_time":
      return a.tracked_minutes - b.tracked_minutes;
    default:
      return 0;
  }
}

function compareNullable(a: string | undefined, b: string | undefined): number {
  if (!a && !b) return 0;
  if (!a) return 1;
  if (!b) return -1;
  return a.localeCompare(b);
}

function getToday(): string {
  return new Date().toISOString().slice(0, 10);
}

function weekStart(today: string): string {
  const d = new Date(today + "T00:00:00");
  const day = d.getDay(); // 0=Sun
  const diff = day === 0 ? -6 : 1 - day; // Monday-start week
  d.setDate(d.getDate() + diff);
  return d.toISOString().slice(0, 10);
}

function weekEnd(today: string): string {
  const d = new Date(weekStart(today) + "T00:00:00");
  d.setDate(d.getDate() + 6);
  return d.toISOString().slice(0, 10);
}
