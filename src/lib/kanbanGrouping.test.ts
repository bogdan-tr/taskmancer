import { describe, expect, test } from "vitest";
import {
  bucketsHaveTasks,
  comparePriorityThenOrder,
  groupByStatusAndPriority,
  insertTaskIntoBuckets,
  OTHER_BUCKET_LABEL,
  removeTaskFromBuckets,
  renumberBucket,
  sortBucketTasks,
  type StatusBuckets,
} from "./kanbanGrouping";
import { FALLBACK_PRIORITY_COLOR } from "./priorities.svelte";
import { FALLBACK_STATUSES } from "./statuses.svelte";
import type { PriorityLevel, Task } from "./types";

const PRIORITIES: PriorityLevel[] = [
  { id: "medium", label: "Medium", color: "oklch(58% 0.13 70)", rank: 2 },
  { id: "high", label: "High", color: "oklch(54% 0.2 350)", rank: 1 },
  { id: "low", label: "Low", color: "oklch(58% 0.14 155)", rank: 3 },
];

const STATUS_IDS = FALLBACK_STATUSES.map((status) => status.id);

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: "t1",
    title: "Task",
    status: "backlog",
    tags: [],
    priority: "medium",
    order: 0,
    created: "2026-06-10T00:00:00.000Z",
    depends_on: [],
    tracked_minutes: 0,
    hidden: false,
    notes: "",
    ...overrides,
  };
}

function buildBuckets(tasks: Task[], groupByPriority = true): StatusBuckets {
  return groupByStatusAndPriority(tasks, PRIORITIES, STATUS_IDS, groupByPriority);
}

describe("groupByStatusAndPriority", () => {
  describe("grouped (groupByPriority = true)", () => {
    test("creates one bucket per priority level (ordered by rank) plus a trailing Other bucket, for every status and for other", () => {
      const result = groupByStatusAndPriority([], PRIORITIES, STATUS_IDS, true);

      for (const buckets of [...STATUS_IDS.map((status) => result.byStatus[status]), result.other]) {
        expect(buckets.map((bucket) => bucket.priorityId)).toEqual(["high", "medium", "low", undefined]);
        expect(buckets.map((bucket) => bucket.label)).toEqual([
          "High",
          "Medium",
          "Low",
          OTHER_BUCKET_LABEL,
        ]);
        expect(buckets.at(-1)?.color).toBe(FALLBACK_PRIORITY_COLOR);
      }
    });

    test("places each task into the bucket matching its status and priority", () => {
      const tasks = [
        makeTask({ id: "a", status: "backlog", priority: "high", order: 0 }),
        makeTask({ id: "b", status: "do", priority: "low", order: 0 }),
      ];

      const result = groupByStatusAndPriority(tasks, PRIORITIES, STATUS_IDS, true);

      expect(
        result.byStatus.backlog.find((bucket) => bucket.priorityId === "high")?.tasks.map((t) => t.id),
      ).toEqual(["a"]);
      expect(
        result.byStatus.do.find((bucket) => bucket.priorityId === "low")?.tasks.map((t) => t.id),
      ).toEqual(["b"]);
    });

    test("places tasks with an unrecognized priority into the trailing Other priority bucket", () => {
      const tasks = [makeTask({ id: "a", status: "backlog", priority: "urgent", order: 0 })];

      const result = groupByStatusAndPriority(tasks, PRIORITIES, STATUS_IDS, true);

      const otherBucket = result.byStatus.backlog.find((bucket) => bucket.priorityId === undefined);
      expect(otherBucket?.tasks.map((t) => t.id)).toEqual(["a"]);
    });

    test("places tasks with an unrecognized status into other, grouped by priority", () => {
      const tasks = [makeTask({ id: "a", status: "on-hold", priority: "high", order: 0 })];

      const result = groupByStatusAndPriority(tasks, PRIORITIES, STATUS_IDS, true);

      expect(result.byStatus.backlog.flatMap((bucket) => bucket.tasks.map((t) => t.id))).toEqual([]);
      expect(result.other.find((bucket) => bucket.priorityId === "high")?.tasks.map((t) => t.id)).toEqual([
        "a",
      ]);
    });

    test("sorts tasks within each bucket by order ascending", () => {
      const tasks = [
        makeTask({ id: "a", status: "backlog", priority: "high", order: 2000 }),
        makeTask({ id: "b", status: "backlog", priority: "high", order: 500 }),
        makeTask({ id: "c", status: "backlog", priority: "high", order: 1000 }),
      ];

      const result = groupByStatusAndPriority(tasks, PRIORITIES, STATUS_IDS, true);

      expect(
        result.byStatus.backlog.find((bucket) => bucket.priorityId === "high")?.tasks.map((t) => t.id),
      ).toEqual(["b", "c", "a"]);
    });

    test("only produces byStatus entries for the given statuses, with everything else under other", () => {
      const tasks = [makeTask({ id: "a", status: "done", priority: "high", order: 0 })];

      const result = groupByStatusAndPriority(tasks, PRIORITIES, ["backlog", "do"], true);

      expect(Object.keys(result.byStatus)).toEqual(["backlog", "do"]);
      expect(result.other.find((bucket) => bucket.priorityId === "high")?.tasks.map((t) => t.id)).toEqual([
        "a",
      ]);
    });
  });

  describe("ungrouped (groupByPriority = false)", () => {
    test("creates a single, unlabeled bucket for every status and for other", () => {
      const result = groupByStatusAndPriority([], PRIORITIES, STATUS_IDS, false);

      for (const buckets of [...STATUS_IDS.map((status) => result.byStatus[status]), result.other]) {
        expect(buckets).toHaveLength(1);
        expect(buckets[0].priorityId).toBeUndefined();
        expect(buckets[0].label).toBe("");
      }
    });

    test("places all of a status's tasks into its single bucket, regardless of priority", () => {
      const tasks = [
        makeTask({ id: "a", status: "backlog", priority: "high", order: 0 }),
        makeTask({ id: "b", status: "backlog", priority: "low", order: 1000 }),
      ];

      const result = groupByStatusAndPriority(tasks, PRIORITIES, STATUS_IDS, false);

      expect(result.byStatus.backlog[0].tasks.map((t) => t.id)).toEqual(["a", "b"]);
    });

    test("sorts the single bucket by priority rank first, then by order", () => {
      const tasks = [
        makeTask({ id: "low-first", status: "backlog", priority: "low", order: 0 }),
        makeTask({ id: "high-second", status: "backlog", priority: "high", order: 1000 }),
        makeTask({ id: "high-first", status: "backlog", priority: "high", order: 0 }),
      ];

      const result = groupByStatusAndPriority(tasks, PRIORITIES, STATUS_IDS, false);

      expect(result.byStatus.backlog[0].tasks.map((t) => t.id)).toEqual([
        "high-first",
        "high-second",
        "low-first",
      ]);
    });

    test("places tasks with an unrecognized status into a single other bucket", () => {
      const tasks = [makeTask({ id: "a", status: "on-hold", priority: "high", order: 0 })];

      const result = groupByStatusAndPriority(tasks, PRIORITIES, STATUS_IDS, false);

      expect(result.other[0].tasks.map((t) => t.id)).toEqual(["a"]);
    });
  });
});

describe("comparePriorityThenOrder", () => {
  test("sorts a task with a lower rank before one with a higher rank", () => {
    const high = makeTask({ id: "high", priority: "high", order: 1000 });
    const low = makeTask({ id: "low", priority: "low", order: 0 });

    expect(comparePriorityThenOrder(high, low, PRIORITIES)).toBeLessThan(0);
    expect(comparePriorityThenOrder(low, high, PRIORITIES)).toBeGreaterThan(0);
  });

  test("falls back to order when priorities match", () => {
    const first = makeTask({ id: "first", priority: "medium", order: 0 });
    const second = makeTask({ id: "second", priority: "medium", order: 1000 });

    expect(comparePriorityThenOrder(first, second, PRIORITIES)).toBeLessThan(0);
    expect(comparePriorityThenOrder(second, first, PRIORITIES)).toBeGreaterThan(0);
  });

  test("sorts tasks with an unrecognized priority after all recognized priorities", () => {
    const recognized = makeTask({ id: "recognized", priority: "low", order: 9999 });
    const unrecognized = makeTask({ id: "unrecognized", priority: "urgent", order: 0 });

    expect(comparePriorityThenOrder(recognized, unrecognized, PRIORITIES)).toBeLessThan(0);
  });
});

describe("sortBucketTasks", () => {
  const tasks = [
    makeTask({ id: "low-first", priority: "low", order: 0 }),
    makeTask({ id: "high-second", priority: "high", order: 1000 }),
  ];

  test("returns tasks unchanged when groupByPriority is true", () => {
    expect(sortBucketTasks(tasks, PRIORITIES, true)).toEqual(tasks);
  });

  test("re-sorts tasks by priority then order when groupByPriority is false", () => {
    const result = sortBucketTasks(tasks, PRIORITIES, false);

    expect(result.map((t) => t.id)).toEqual(["high-second", "low-first"]);
  });

  test("does not mutate the input array", () => {
    const before = [...tasks];

    sortBucketTasks(tasks, PRIORITIES, false);

    expect(tasks).toEqual(before);
  });
});

describe("renumberBucket", () => {
  test("renumbers tasks sequentially using the given order step", () => {
    const tasks = [
      makeTask({ id: "a", order: 9999, status: "backlog", priority: "low" }),
      makeTask({ id: "b", order: 1, status: "backlog", priority: "low" }),
    ];

    const result = renumberBucket(tasks, "backlog", "high", 1000);

    expect(result.map((task) => task.order)).toEqual([0, 1000]);
  });

  test("sets status on every task to statusId", () => {
    const tasks = [makeTask({ id: "a", status: "backlog", priority: "high" })];

    const result = renumberBucket(tasks, "do", "high", 1000);

    expect(result[0].status).toBe("do");
  });

  test("sets priority to priorityId when a priority level is given", () => {
    const tasks = [makeTask({ id: "a", status: "backlog", priority: "low" })];

    const result = renumberBucket(tasks, "backlog", "high", 1000);

    expect(result[0].priority).toBe("high");
  });

  test("preserves each task's existing priority when priorityId is undefined (the Other priority bucket)", () => {
    const tasks = [makeTask({ id: "a", status: "backlog", priority: "urgent" })];

    const result = renumberBucket(tasks, "do", undefined, 1000);

    expect(result[0].priority).toBe("urgent");
    expect(result[0].status).toBe("do");
  });

  test("preserves each task's existing status when statusId is undefined (the Other status column)", () => {
    const tasks = [makeTask({ id: "a", status: "on-hold", priority: "low" })];

    const result = renumberBucket(tasks, undefined, "high", 1000);

    expect(result[0].status).toBe("on-hold");
    expect(result[0].priority).toBe("high");
  });

  test("does not mutate the input tasks", () => {
    const tasks = [makeTask({ id: "a", order: 9999, status: "backlog", priority: "low" })];
    const before = makeTask({ id: "a", order: 9999, status: "backlog", priority: "low" });

    renumberBucket(tasks, "do", "high", 1000);

    expect(tasks[0]).toEqual(before);
  });
});

describe("removeTaskFromBuckets", () => {
  test("removes the task from whichever status/bucket holds it", () => {
    const tasks = [
      makeTask({ id: "a", status: "backlog", priority: "high", order: 0 }),
      makeTask({ id: "b", status: "do", priority: "medium", order: 0 }),
    ];
    const buckets = buildBuckets(tasks);

    const result = removeTaskFromBuckets(buckets, "a");

    expect(result.byStatus.backlog.flatMap((bucket) => bucket.tasks.map((t) => t.id))).not.toContain("a");
    expect(result.byStatus.do.flatMap((bucket) => bucket.tasks.map((t) => t.id))).toEqual(["b"]);
  });

  test("removes the task from the other bucket", () => {
    const tasks = [makeTask({ id: "a", status: "on-hold", priority: "high", order: 0 })];
    const buckets = buildBuckets(tasks);

    const result = removeTaskFromBuckets(buckets, "a");

    expect(result.other.flatMap((bucket) => bucket.tasks.map((t) => t.id))).not.toContain("a");
  });

  test("leaves all buckets unchanged when no task matches the id", () => {
    const tasks = [makeTask({ id: "a", status: "backlog", priority: "high", order: 0 })];
    const buckets = buildBuckets(tasks);
    const expectedUnchanged = buildBuckets(tasks);

    const result = removeTaskFromBuckets(buckets, "missing");

    expect(result).toEqual(expectedUnchanged);
  });

  test("does not mutate the input buckets", () => {
    const tasks = [makeTask({ id: "a", status: "backlog", priority: "high", order: 0 })];
    const buckets = buildBuckets(tasks);
    const expectedUnchanged = buildBuckets(tasks);

    removeTaskFromBuckets(buckets, "a");

    expect(buckets).toEqual(expectedUnchanged);
  });
});

describe("insertTaskIntoBuckets", () => {
  describe("grouped (groupByPriority = true)", () => {
    test("inserts the task into the bucket matching its status and priority", () => {
      const buckets = buildBuckets([]);
      const task = makeTask({ id: "a", status: "backlog", priority: "medium", order: 0 });

      const result = insertTaskIntoBuckets(buckets, task, PRIORITIES, true);

      const bucket = result.byStatus.backlog.find((b) => b.priorityId === "medium");
      expect(bucket?.tasks.map((t) => t.id)).toEqual(["a"]);
    });

    test("inserts into the trailing Other priority bucket when no priority level matches", () => {
      const buckets = buildBuckets([]);
      const task = makeTask({ id: "a", status: "backlog", priority: "urgent", order: 0 });

      const result = insertTaskIntoBuckets(buckets, task, PRIORITIES, true);

      const otherBucket = result.byStatus.backlog.find((b) => b.priorityId === undefined);
      expect(otherBucket?.tasks.map((t) => t.id)).toEqual(["a"]);
    });

    test("inserts into other when the task's status doesn't match any column", () => {
      const buckets = buildBuckets([]);
      const task = makeTask({ id: "a", status: "on-hold", priority: "high", order: 0 });

      const result = insertTaskIntoBuckets(buckets, task, PRIORITIES, true);

      const bucket = result.other.find((b) => b.priorityId === "high");
      expect(bucket?.tasks.map((t) => t.id)).toEqual(["a"]);
    });

    test("keeps the bucket's tasks sorted by order after insertion", () => {
      const existing = makeTask({ id: "a", status: "backlog", priority: "high", order: 1000 });
      const buckets = buildBuckets([existing]);
      const inserted = makeTask({ id: "b", status: "backlog", priority: "high", order: 500 });

      const result = insertTaskIntoBuckets(buckets, inserted, PRIORITIES, true);

      const bucket = result.byStatus.backlog.find((b) => b.priorityId === "high");
      expect(bucket?.tasks.map((t) => t.id)).toEqual(["b", "a"]);
    });

    test("leaves buckets for other statuses untouched", () => {
      const buckets = buildBuckets([]);
      const task = makeTask({ id: "a", status: "backlog", priority: "medium", order: 0 });

      const result = insertTaskIntoBuckets(buckets, task, PRIORITIES, true);

      expect(result.byStatus.do).toEqual(buckets.byStatus.do);
      expect(result.byStatus["in-progress"]).toEqual(buckets.byStatus["in-progress"]);
    });

    test("does not mutate the input buckets", () => {
      const buckets = buildBuckets([]);
      const expectedUnchanged = buildBuckets([]);
      const task = makeTask({ id: "a", status: "backlog", priority: "medium", order: 0 });

      insertTaskIntoBuckets(buckets, task, PRIORITIES, true);

      expect(buckets).toEqual(expectedUnchanged);
    });
  });

  describe("ungrouped (groupByPriority = false)", () => {
    test("inserts the task into its status's single bucket, sorted by priority then order", () => {
      const existing = makeTask({ id: "a", status: "backlog", priority: "low", order: 0 });
      const buckets = buildBuckets([existing], false);
      const inserted = makeTask({ id: "b", status: "backlog", priority: "high", order: 1000 });

      const result = insertTaskIntoBuckets(buckets, inserted, PRIORITIES, false);

      expect(result.byStatus.backlog[0].tasks.map((t) => t.id)).toEqual(["b", "a"]);
    });

    test("inserts into other's single bucket when the task's status doesn't match any column", () => {
      const buckets = buildBuckets([], false);
      const task = makeTask({ id: "a", status: "on-hold", priority: "high", order: 0 });

      const result = insertTaskIntoBuckets(buckets, task, PRIORITIES, false);

      expect(result.other[0].tasks.map((t) => t.id)).toEqual(["a"]);
    });

    test("does not mutate the input buckets", () => {
      const buckets = buildBuckets([], false);
      const expectedUnchanged = buildBuckets([], false);
      const task = makeTask({ id: "a", status: "backlog", priority: "medium", order: 0 });

      insertTaskIntoBuckets(buckets, task, PRIORITIES, false);

      expect(buckets).toEqual(expectedUnchanged);
    });
  });
});

describe("bucketsHaveTasks", () => {
  test("returns false when every bucket is empty", () => {
    const result = buildBuckets([]);

    expect(bucketsHaveTasks(result.other)).toBe(false);
  });

  test("returns true when at least one bucket has a task", () => {
    const tasks = [makeTask({ id: "a", status: "on-hold", priority: "high", order: 0 })];
    const result = buildBuckets(tasks);

    expect(bucketsHaveTasks(result.other)).toBe(true);
  });
});
