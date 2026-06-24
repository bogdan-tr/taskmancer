import { describe, expect, it } from "vitest";
import { newlyAllDoneTaskIds } from "./subtaskCompletion";
import type { Task } from "./types";

const DONE = "done";
const CANCELLED = "cancelled";
const TODAY = "2026-06-22";

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: crypto.randomUUID(),
    title: "Task",
    status: "backlog",
    tags: [],
    priority: "medium",
    order: 1,
    created: "2026-06-11T00:00:00+00:00",
    depends_on: [],
    tracked_minutes: 0,
    hidden: false,
    notes: "",
    ...overrides,
  };
}

describe("newlyAllDoneTaskIds", () => {
  it("reports a task as newly done the first time all its subtasks finish", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const sub1 = makeTask({ id: "sub1", project_id: "container", status: DONE });
    const sub2 = makeTask({ id: "sub2", project_id: "container", status: DONE });

    const result = newlyAllDoneTaskIds(new Set(), [parent, sub1, sub2], DONE, CANCELLED, TODAY);

    expect(result.newlyDone).toEqual(["parent"]);
    expect(result.stillAllDone).toEqual(new Set(["parent"]));
  });

  it("does not report a task again on a later run where it's still all done", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const sub1 = makeTask({ id: "sub1", project_id: "container", status: DONE });
    const tasks = [parent, sub1];

    const result = newlyAllDoneTaskIds(new Set(["parent"]), tasks, DONE, CANCELLED, TODAY);

    expect(result.newlyDone).toEqual([]);
    expect(result.stillAllDone).toEqual(new Set(["parent"]));
  });

  it("fires again after a subtask is un-done and then re-done (a fresh transition)", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const sub1 = makeTask({ id: "sub1", project_id: "container", status: "backlog" });

    const undone = newlyAllDoneTaskIds(new Set(["parent"]), [parent, sub1], DONE, CANCELLED, TODAY);
    expect(undone.newlyDone).toEqual([]);
    expect(undone.stillAllDone).toEqual(new Set());

    const redone = newlyAllDoneTaskIds(undone.stillAllDone, [parent, { ...sub1, status: DONE }], DONE, CANCELLED, TODAY);
    expect(redone.newlyDone).toEqual(["parent"]);
  });

  it("ignores cancelled subtasks for both the done count and the denominator", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const sub1 = makeTask({ id: "sub1", project_id: "container", status: DONE });
    const sub2 = makeTask({ id: "sub2", project_id: "container", status: CANCELLED });

    const result = newlyAllDoneTaskIds(new Set(), [parent, sub1, sub2], DONE, CANCELLED, TODAY);

    expect(result.newlyDone).toEqual(["parent"]);
  });

  it("never reports a task with no container, or a container with zero non-cancelled subtasks", () => {
    const plainTask = makeTask({ id: "plain" });
    const ownerOfEmptyContainer = makeTask({ id: "owner", subtask_project_id: "container" });
    const onlyCancelledSubtask = makeTask({ id: "sub", project_id: "container", status: CANCELLED });

    const result = newlyAllDoneTaskIds(
      new Set(),
      [plainTask, ownerOfEmptyContainer, onlyCancelledSubtask],
      DONE,
      CANCELLED,
      TODAY,
    );

    expect(result.newlyDone).toEqual([]);
    expect(result.stillAllDone).toEqual(new Set());
  });

  it("preserves a task's membership when it's absent from the current batch, without re-triggering once it reappears", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const sub1 = makeTask({ id: "sub1", project_id: "container", status: DONE });

    // A different project's board doesn't include this task or its subtask at all.
    const whileNavigatedAway = newlyAllDoneTaskIds(new Set(["parent"]), [], DONE, CANCELLED, TODAY);
    expect(whileNavigatedAway.newlyDone).toEqual([]);
    expect(whileNavigatedAway.stillAllDone).toEqual(new Set(["parent"]));

    const backInView = newlyAllDoneTaskIds(whileNavigatedAway.stillAllDone, [parent, sub1], DONE, CANCELLED, TODAY);
    expect(backInView.newlyDone).toEqual([]);
  });

  it("preserves membership when the parent is present but its subtasks haven't loaded into this batch yet (a load-order race, not navigating away)", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const sub1 = makeTask({ id: "sub1", project_id: "container", status: DONE });

    // Simulates the moment right after a reload where the parent's global
    // cache (`tasksState`) has resolved but the current board's own task
    // fetch hasn't yet — `parent` itself is in the batch, but `sub1` isn't.
    const duringLoadRace = newlyAllDoneTaskIds(new Set(["parent"]), [parent], DONE, CANCELLED, TODAY);
    expect(duringLoadRace.newlyDone).toEqual([]);
    expect(duringLoadRace.stillAllDone).toEqual(new Set(["parent"]));

    // Once the real subtasks arrive, it must not look like a fresh completion.
    const fullyLoaded = newlyAllDoneTaskIds(duringLoadRace.stillAllDone, [parent, sub1], DONE, CANCELLED, TODAY);
    expect(fullyLoaded.newlyDone).toEqual([]);
  });

  it("handles multiple tasks completing in the same run", () => {
    const parentA = makeTask({ id: "parentA", subtask_project_id: "containerA" });
    const subA = makeTask({ id: "subA", project_id: "containerA", status: DONE });
    const parentB = makeTask({ id: "parentB", subtask_project_id: "containerB" });
    const subB = makeTask({ id: "subB", project_id: "containerB", status: DONE });

    const result = newlyAllDoneTaskIds(new Set(), [parentA, subA, parentB, subB], DONE, CANCELLED, TODAY);

    expect(new Set(result.newlyDone)).toEqual(new Set(["parentA", "parentB"]));
  });

  it("fires once today's occurrence of a recurring subtask is done, ignoring un-done future occurrences pre-generated by its series", () => {
    const parent = makeTask({ id: "parent", subtask_project_id: "container" });
    const todayOccurrence = makeTask({
      id: "today",
      project_id: "container",
      series_id: "series-1",
      status: DONE,
      scheduled: TODAY,
    });
    const futureOccurrences = Array.from({ length: 60 }, (_, i) => {
      const month = i < 28 ? "07" : "08";
      const day = String((i % 28) + 1).padStart(2, "0");
      return makeTask({
        id: `future-${i}`,
        project_id: "container",
        series_id: "series-1",
        status: "backlog",
        scheduled: `2026-${month}-${day}`,
      });
    });

    const result = newlyAllDoneTaskIds(
      new Set(),
      [parent, todayOccurrence, ...futureOccurrences],
      DONE,
      CANCELLED,
      TODAY,
    );

    expect(result.newlyDone).toEqual(["parent"]);
  });
});
