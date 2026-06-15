import { FALLBACK_PRIORITY_COLOR, sortedPriorities } from "./priorities.svelte";
import type { PriorityLevel, Task } from "./types";

/** A group of same-status, same-priority tasks for Kanban column rendering. */
export interface PriorityBucket {
  /**
   * The priority level id for this bucket, or `undefined` for the trailing
   * "Other" bucket holding tasks whose `priority` doesn't match any
   * currently-defined level (or, in ungrouped mode, for the single bucket
   * holding every task in the column).
   */
  priorityId: string | undefined;
  label: string;
  color: string;
  tasks: Task[];
}

export const OTHER_BUCKET_LABEL = "Other";

/**
 * The full grouping of tasks for Kanban board rendering: `byStatus` holds one
 * entry per id in the board's configured `statuses`, and `other` holds tasks
 * whose `status` doesn't match any of those ids (e.g. after a status was
 * removed from a project's board or from the global status list), for
 * rendering in a trailing "Other" status column.
 */
export interface StatusBuckets {
  byStatus: Record<string, PriorityBucket[]>;
  other: PriorityBucket[];
}

/**
 * A Kanban column: either a configured status (`id` set) or the trailing
 * "Other" status column (`id` undefined) for tasks whose status falls
 * outside the board's configured statuses.
 */
export interface BoardColumn {
  id: string | undefined;
  label: string;
  color: string;
  buckets: PriorityBucket[];
}

/** Returns whether any bucket in `buckets` contains at least one task. */
export function bucketsHaveTasks(buckets: PriorityBucket[]): boolean {
  return buckets.some((bucket) => bucket.tasks.length > 0);
}

/**
 * Compares two tasks for the "ungrouped" Kanban ordering: by priority rank
 * ascending (rank 1 first), then by `order` ascending. Tasks whose `priority`
 * doesn't match any defined level sort after all recognized priorities.
 */
export function comparePriorityThenOrder(a: Task, b: Task, priorities: PriorityLevel[]): number {
  const rankA = priorities.find((level) => level.id === a.priority)?.rank ?? Number.POSITIVE_INFINITY;
  const rankB = priorities.find((level) => level.id === b.priority)?.rank ?? Number.POSITIVE_INFINITY;
  if (rankA !== rankB) return rankA - rankB;
  return a.order - b.order;
}

/**
 * Groups `tasks` by priority: buckets for currently-defined priority levels
 * come first (ordered by `rank`, rank 1 first), followed by a trailing
 * "Other" bucket for tasks whose `priority` doesn't match any defined level.
 * Always returns `priorities.length + 1` buckets in this fixed order, so
 * callers can render stable drop targets even when a bucket is empty. Tasks
 * within each bucket are sorted by `order` ascending.
 */
function buildPriorityBuckets(tasks: Task[], priorities: PriorityLevel[]): PriorityBucket[] {
  const levels = sortedPriorities(priorities);
  const buckets: PriorityBucket[] = [
    ...levels.map((level) => ({
      priorityId: level.id,
      label: level.label,
      color: level.color,
      tasks: [] as Task[],
    })),
    {
      priorityId: undefined,
      label: OTHER_BUCKET_LABEL,
      color: FALLBACK_PRIORITY_COLOR,
      tasks: [] as Task[],
    },
  ];

  for (const task of tasks) {
    const bucket = buckets.find((b) => b.priorityId === task.priority) ?? buckets[buckets.length - 1];
    bucket.tasks.push(task);
  }

  for (const bucket of buckets) {
    bucket.tasks.sort((a, b) => a.order - b.order);
  }

  return buckets;
}

/**
 * Returns a single bucket holding every task in `tasks`, sorted by priority
 * rank then `order` (see [`comparePriorityThenOrder`]) - used for "ungrouped"
 * Kanban columns where priority isn't visually divided but still determines
 * sort order.
 */
function buildUngroupedBucket(tasks: Task[], priorities: PriorityLevel[]): PriorityBucket[] {
  return [
    {
      priorityId: undefined,
      label: "",
      color: FALLBACK_PRIORITY_COLOR,
      tasks: [...tasks].sort((a, b) => comparePriorityThenOrder(a, b, priorities)),
    },
  ];
}

/**
 * Builds the priority bucket(s) for one status's tasks: `priorities.length +
 * 1` priority-divided buckets (see [`buildPriorityBuckets`]) when
 * `groupByPriority` is true, or a single bucket pre-sorted by priority then
 * order (see [`buildUngroupedBucket`]) when false.
 */
function buildBuckets(tasks: Task[], priorities: PriorityLevel[], groupByPriority: boolean): PriorityBucket[] {
  return groupByPriority ? buildPriorityBuckets(tasks, priorities) : buildUngroupedBucket(tasks, priorities);
}

/**
 * Groups `tasks` by `status`, then by priority within each status.
 *
 * `statuses` is the ordered list of status ids for the board being rendered
 * (e.g. a project's configured subset, or the global status list). Tasks
 * whose `status` doesn't match any id in `statuses` are grouped the same way
 * under `other`, for rendering in a trailing "Other" status column.
 *
 * `groupByPriority` selects between priority-divided buckets and a single
 * priority-sorted bucket per status - see [`buildBuckets`].
 */
export function groupByStatusAndPriority(
  tasks: Task[],
  priorities: PriorityLevel[],
  statuses: string[],
  groupByPriority: boolean,
): StatusBuckets {
  const byStatus: Record<string, PriorityBucket[]> = {};
  for (const status of statuses) {
    byStatus[status] = buildBuckets(
      tasks.filter((task) => task.status === status),
      priorities,
      groupByPriority,
    );
  }

  const other = buildBuckets(
    tasks.filter((task) => !statuses.includes(task.status)),
    priorities,
    groupByPriority,
  );

  return { byStatus, other };
}

/**
 * Returns a copy of `tasks` with `order` reassigned sequentially
 * (`index * orderStep`). `statusId` is set on every task unless `undefined`
 * (the "Other" status column has no single status to assign, so each task's
 * existing `status` is preserved); `priority` is set to `priorityId` unless
 * `priorityId` is `undefined` (the "Other" priority bucket, and every bucket
 * in ungrouped mode, has no priority level to assign, so each task's existing
 * `priority` is preserved).
 */
export function renumberBucket(
  tasks: Task[],
  statusId: string | undefined,
  priorityId: string | undefined,
  orderStep: number,
): Task[] {
  return tasks.map((task, index) => ({
    ...task,
    order: index * orderStep,
    status: statusId ?? task.status,
    priority: priorityId ?? task.priority,
  }));
}

/**
 * Returns `tasks` for display after [`renumberBucket`]: in grouped mode
 * (`groupByPriority` true) `tasks` are already in the dropped order within a
 * single-priority bucket and are returned as-is; in ungrouped mode `tasks`
 * are re-sorted by priority rank then `order` (see
 * [`comparePriorityThenOrder`]) so a task dropped among different-priority
 * neighbors "snaps" back into its own priority tier.
 */
export function sortBucketTasks(tasks: Task[], priorities: PriorityLevel[], groupByPriority: boolean): Task[] {
  return groupByPriority ? tasks : [...tasks].sort((a, b) => comparePriorityThenOrder(a, b, priorities));
}

/** Returns a copy of `buckets` with every task matching `taskId` removed. */
function removeFromPriorityBuckets(buckets: PriorityBucket[], taskId: string): PriorityBucket[] {
  return buckets.map((bucket) => ({
    ...bucket,
    tasks: bucket.tasks.filter((task) => task.id !== taskId),
  }));
}

/** Returns a copy of `buckets` with every task matching `taskId` removed. */
export function removeTaskFromBuckets(buckets: StatusBuckets, taskId: string): StatusBuckets {
  const byStatus: Record<string, PriorityBucket[]> = {};
  for (const [status, statusBuckets] of Object.entries(buckets.byStatus)) {
    byStatus[status] = removeFromPriorityBuckets(statusBuckets, taskId);
  }

  return { byStatus, other: removeFromPriorityBuckets(buckets.other, taskId) };
}

/**
 * Returns a copy of `buckets` with `task` inserted: in grouped mode, into the
 * bucket matching `task.priority` (falling back to the trailing "Other"
 * priority bucket if no level matches), keeping that bucket sorted by
 * `order`; in ungrouped mode, into the column's single bucket, keeping it
 * sorted by priority then `order` (see [`comparePriorityThenOrder`]).
 */
function insertIntoPriorityBuckets(
  buckets: PriorityBucket[],
  task: Task,
  priorities: PriorityLevel[],
  groupByPriority: boolean,
): PriorityBucket[] {
  if (!groupByPriority) {
    return [
      {
        ...buckets[0],
        tasks: [...buckets[0].tasks, task].sort((a, b) => comparePriorityThenOrder(a, b, priorities)),
      },
    ];
  }

  const matchIndex = buckets.findIndex((bucket) => bucket.priorityId === task.priority);
  const targetIndex = matchIndex >= 0 ? matchIndex : buckets.length - 1;

  return buckets.map((bucket, index) => {
    if (index !== targetIndex) return bucket;
    return { ...bucket, tasks: [...bucket.tasks, task].sort((a, b) => a.order - b.order) };
  });
}

/**
 * Returns a copy of `buckets` with `task` inserted into the status column
 * matching `task.status` (falling back to `other` if no column matches), then
 * into that column's bucket(s) per [`insertIntoPriorityBuckets`].
 *
 * Does not remove `task` from any other bucket - callers replacing an
 * existing task should call [`removeTaskFromBuckets`] first.
 */
export function insertTaskIntoBuckets(
  buckets: StatusBuckets,
  task: Task,
  priorities: PriorityLevel[],
  groupByPriority: boolean,
): StatusBuckets {
  if (task.status in buckets.byStatus) {
    return {
      ...buckets,
      byStatus: {
        ...buckets.byStatus,
        [task.status]: insertIntoPriorityBuckets(buckets.byStatus[task.status], task, priorities, groupByPriority),
      },
    };
  }

  return { ...buckets, other: insertIntoPriorityBuckets(buckets.other, task, priorities, groupByPriority) };
}
