<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { onMount } from "svelte";
  import type { DndEvent } from "svelte-dnd-action";
  import {
    createRecurringTask,
    createTask,
    deleteSeriesOccurrence,
    deleteSubtaskContainer,
    deleteTask,
    ensureOccurrencesUntil,
    finishDay,
    listTasks,
    removeRecurrence,
    reorderTask,
    updateSeriesOccurrence,
    updateSeriesRecurrence,
    updateTask,
  } from "$lib/api";
  import { isVisibleOnBoard } from "$lib/boardVisibility";
  import AddTaskModal from "$lib/components/AddTaskModal.svelte";
  import AllSubtasksDoneDialog from "$lib/components/AllSubtasksDoneDialog.svelte";
  import CalendarView from "$lib/components/CalendarView.svelte";
  import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
  import DashboardView from "$lib/components/DashboardView.svelte";
  import ProjectDashboardView from "$lib/components/ProjectDashboardView.svelte";
  import KanbanGrid from "$lib/components/KanbanGrid.svelte";
  import GlobalStatusBar from "$lib/components/GlobalStatusBar.svelte";
  import ProjectStatusLine from "$lib/components/ProjectStatusLine.svelte";
  import TaskEditDialog from "$lib/components/TaskEditDialog.svelte";
  import WeekView from "$lib/components/WeekView.svelte";
  import { vimState } from "$lib/vim.svelte";
  import { displayState } from "$lib/displaySettings.svelte";
  import { getErrorMessage } from "$lib/errors";
  import { formatFinishDayResult } from "$lib/finishDay";
  import {
    bucketsHaveTasks,
    groupByStatusAndPriority,
    insertTaskIntoBuckets,
    OTHER_BUCKET_LABEL,
    removeTaskFromBuckets,
    renumberBucket,
    sortBucketTasks,
    type BoardColumn,
    type PriorityBucket,
    type StatusBuckets,
  } from "$lib/kanbanGrouping";
  import type { ParsedTaskInput } from "$lib/naturalLanguage";
  import { FALLBACK_PRIORITIES } from "$lib/priorities.svelte";
  import { effectiveBoardStatuses } from "$lib/projectBoardSettings";
  import { projectsState, refreshProjects } from "$lib/projects.svelte";
  import { childrenOf, descendantsOf, selfAndAncestors } from "$lib/projectTree";
  import { isExpanded, toggleExpanded } from "$lib/projectTree.svelte";
  import type { Project } from "$lib/types";
  import type { DueRule, RecurrenceFrequency, SeriesEditScope } from "$lib/recurrence";
  import { settingsState } from "$lib/settings.svelte";
  import {
    FALLBACK_STATUS_COLOR,
    FALLBACK_STATUSES,
    sortedStatuses,
    statusColor,
    statusLabel,
  } from "$lib/statuses.svelte";
  import { refreshProjectStatusStats } from "$lib/statusLine.svelte";
  import {
    allDoneQueueState,
    checkAllDoneTransitions,
    dequeueAllDone,
    dismissAllDonePermanently,
  } from "$lib/subtaskCompletionQueue.svelte";
  import { containerOwner, subtasksOf } from "$lib/subtasks";
  import { refreshTasks, removeCachedTask, tasksState, upsertCachedTask } from "$lib/tasks.svelte";
  import { isHiddenAsSubtask } from "$lib/subtaskVisibility";
  import { refreshTags } from "$lib/tags.svelte";
  import {
    isTaskActive,
    liveDisplaySecondsFor,
    startProjectTracking,
    startTaskTracking,
    stopProjectTracking,
    stopTaskTracking,
  } from "$lib/tracking.svelte";
  import type { Task } from "$lib/types";
  import { formatDateISO } from "$lib/weekRange";
  import { formatHms } from "$lib/liveTimer";

  interface Props {
    title: string;
    tagline?: string;
    accentColor?: string;
    /** When set, only tasks whose `project_id` matches this id are shown. */
    projectFilter?: string;
  }

  let { title, tagline, accentColor, projectFilter }: Props = $props();

  /** Spacing between sequential `order` values when a bucket is renumbered after a drag. */
  const ORDER_STEP = 1000;

  /** Returns all projects in sidebar display order (depth-first tree), excluding subtask containers. */
  /** Depth-first visible project order, respecting sidebar expand/collapse state. */
  function sidebarProjectOrder(projects: Project[]): Project[] {
    const excluded = new Set(
      projects.filter((p) => containerOwner(p.id, tasksState.items) !== undefined).map((p) => p.id),
    );
    const result: Project[] = [];
    function traverse(parentId: string | undefined) {
      for (const child of childrenOf(projects, parentId)) {
        if (excluded.has(child.id)) continue;
        result.push(child);
        if (isExpanded(child.id)) traverse(child.id);
      }
    }
    traverse(undefined);
    return result;
  }

  let priorities = $derived(settingsState.current?.priorities ?? FALLBACK_PRIORITIES);
  let statuses = $derived(sortedStatuses(settingsState.current?.statuses ?? FALLBACK_STATUSES));

  /** Whether Kanban columns divide tasks into priority groups, from the global display settings. */
  let groupByPriority = $derived(displayState.showPriorityGroups);

  /**
   * The current project, looked up by id, when this board is scoped to a
   * project via `projectFilter`.
   */
  let project = $derived(
    projectFilter ? projectsState.items.find((p) => p.id === projectFilter) : undefined,
  );

  /** True when the current project is a subtask container (navigated to via a task's own subtask board). Subtask containers should not expose project settings. */
  let isSubtaskContainer = $derived(
    !!projectFilter && tasksState.items.some((t) => t.subtask_project_id === projectFilter),
  );

  /** The full ancestor chain for the current project (own board first, then ancestors' boards, nearest-first), or empty when this board isn't project-scoped. */
  let projectChain = $derived(project ? selfAndAncestors(projectsState.items, project.id) : []);

  /** The nearest `board.show_subproject_tasks` override in `projectChain`, else the global default — opt-in, defaulting to `false`, per the Subtasks-feedback round that introduced this setting. */
  let showSubprojectTasks = $derived(
    projectChain.find((p) => p.board.show_subproject_tasks !== undefined)?.board.show_subproject_tasks ??
      settingsState.current?.show_subproject_tasks_default ??
      false,
  );

  /**
   * `projectFilter` plus every one of its descendant subprojects' ids
   * worth rolling up — a "real" descendant subproject only counts when
   * `showSubprojectTasks` is enabled for the viewed project (rollup is
   * opt-in, see `showSubprojectTasks`'s own doc comment), but a hidden
   * subtask container is *always* included regardless of that toggle: the
   * glued nested-row feature on `TaskCard` needs its owning task's
   * subtasks present in `visibleTasks` everywhere that task's own card
   * renders, and a subtask container is never a "subproject" from the
   * user's perspective in the first place, just this feature's internal
   * plumbing.
   */
  let rollupProjectIds = $derived.by(() => {
    if (!projectFilter) return [];
    const descendants = descendantsOf(projectsState.items, projectFilter);
    const included = showSubprojectTasks
      ? descendants
      : descendants.filter((p) => {
          const owner = containerOwner(p.id, tasksState.items);
          return owner !== undefined && owner.project_id === projectFilter;
        });
    return [projectFilter, ...included.map((p) => p.id)];
  });

  /**
   * The status ids shown as columns on this board, in display order: the
   * nearest customized board in `projectChain` if any has one, otherwise
   * every status in the global list.
   */
  let boardStatusIds = $derived(
    effectiveBoardStatuses(
      projectChain.map((p) => p.board),
      statuses.map((status) => status.id),
    ),
  );

  /** The nearest `board.show_previous_weeks` override in `projectChain`, else the global default. */
  let showPreviousWeeksColumn = $derived(
    projectChain.find((p) => p.board.show_previous_weeks !== undefined)?.board.show_previous_weeks ??
      settingsState.current?.show_previous_weeks_column ??
      false,
  );

  let buckets: StatusBuckets = $state(
    groupByStatusAndPriority(
      [],
      FALLBACK_PRIORITIES,
      FALLBACK_STATUSES.map((status) => status.id),
      displayState.showPriorityGroups,
    ),
  );

  /**
   * Whether the trailing "Other" status column should be shown - only when it
   * has at least one task whose `status` falls outside this board's
   * configured statuses. Tracked separately from `buckets.other` (rather than
   * derived from it directly) so that `handleConsider`'s live drag preview -
   * which can transiently empty `buckets.other` while dragging its last task
   * out - doesn't make the column (and its drop target) disappear mid-drag;
   * it's recomputed once the drag settles in `handleFinalize`.
   */
  let hasOtherTasks = $state(false);

  /** Syncs `hasOtherTasks` with the current contents of `buckets.other`. */
  function recomputeHasOther() {
    hasOtherTasks = bucketsHaveTasks(buckets.other);
  }

  let boardColumns: BoardColumn[] = $derived([
    ...boardStatusIds.map((id) => ({
      id,
      label: statusLabel(statuses, id),
      color: statusColor(statuses, id),
      buckets: buckets.byStatus[id] ?? [],
    })),
    ...(hasOtherTasks
      ? [{ id: undefined, label: OTHER_BUCKET_LABEL, color: FALLBACK_STATUS_COLOR, buckets: buckets.other }]
      : []),
  ]);

  let isLoading = $state(true);
  let errorMessage = $state("");
  let modalOpen = $state(false);
  /** Seeds `AddTaskModal`'s `parentTaskId` — set by `openCreateSubtask`, `undefined` for the ordinary "+ Add task" entry point. */
  let subtaskParentIdForModal: string | undefined = $state(undefined);
  let finishDayConfirmOpen = $state(false);
  let finishDayMessage = $state("");
  let isFinishingDay = $state(false);
  let vimDeleteOpen = $state(false);
  let vimDeletePendingId = $state<string | null>(null);

  /** Which view this board shows: the Kanban grid or the calendar week view. */
  let activeView: "board" | "week" | "calendar" | "dashboard" = $state("board");

  /** The task opened for editing via the vim `e` key. `undefined` when the dialog is closed. */
  let vimEditTask: Task | undefined = $state(undefined);

  /**
   * Every task visible on this board (project-filtered, but not subject to
   * Item 3's future-`scheduled` visibility rule) - passed to `WeekView`,
   * which shows scheduled/due bars regardless of whether a task is currently
   * hidden from the Kanban grid.
   */
  let visibleTasks: Task[] = $state([]);

  /**
   * `visibleTasks` minus any subtask that shouldn't render as a standalone
   * bar here — passed to `WeekView`/`CalendarView` instead of `visibleTasks`
   * directly, mirroring the equivalent exclusion `boardVisible` (inside
   * `refresh`/`replaceTask`) applies to the Kanban grid's own buckets. See
   * `isHiddenAsSubtask`'s own doc comment for the exact rule. Also excludes
   * any `task.hidden` task (e.g. a project's auto-generated time-tracking
   * anchor, see `Task.hidden`'s own doc comment) — never a real, user-facing
   * task, so it must never render as a bar here either.
   */
  let weekCalendarTasks = $derived(
    visibleTasks.filter((task) => !task.hidden && !isHiddenAsSubtask(task, visibleTasks, projectFilter)),
  );

  /** All `{ date, taskIds }` entries across every dated task — the full global list. */
  let weekDays = $derived(
    (() => {
      const map = new Map<string, string[]>();
      for (const t of weekCalendarTasks) {
        const date = t.due?.slice(0, 10) ?? t.scheduled?.slice(0, 10);
        if (!date) continue;
        if (!map.has(date)) map.set(date, []);
        map.get(date)!.push(t.id);
      }
      return [...map.entries()]
        .sort(([a], [b]) => a.localeCompare(b))
        .map(([date, taskIds]) => ({ date, taskIds }));
    })(),
  );

  /** Date strings currently visible in WeekView — set via onDateStringsChange callback. */
  let weekViewDateStrings = $state<string[]>([]);
  /** Date strings currently visible in CalendarView — set via onDateStringsChange callback. */
  let calendarViewDateStrings = $state<string[]>([]);

  /** weekDays filtered to only dates visible in the current WeekView. */
  let weekViewVisibleDays = $derived(
    weekViewDateStrings.length > 0
      ? weekDays.filter((d) => weekViewDateStrings.includes(d.date))
      : [],
  );
  /** weekDays filtered to only dates visible in the current CalendarView. */
  let calendarViewVisibleDays = $derived(
    calendarViewDateStrings.length > 0
      ? weekDays.filter((d) => calendarViewDateStrings.includes(d.date))
      : [],
  );

  /** Opens the add-task modal, optionally pre-marking it as creating a subtask of the given task (the "Create Subtask" entry points). */
  function openAddTaskModal(parentTaskId?: string) {
    subtaskParentIdForModal = parentTaskId;
    modalOpen = true;
  }

  function openCreateSubtask(task: Task) {
    openAddTaskModal(task.id);
  }

  /**
   * The task that owns the subtask container currently being viewed, if
   * `projectFilter` names one — i.e. this board *is* a subtask's own
   * board, not the parent's. Looked up against the global `tasksState`
   * (kept fresh via `refreshTags`/`refreshTasks`), not `visibleTasks`,
   * since on this exact board `visibleTasks` is just the subtasks
   * themselves — the owner task lives in a different project entirely
   * and is never part of it.
   */
  let containerOwnerTask = $derived(
    projectFilter ? containerOwner(projectFilter, tasksState.items) : undefined,
  );

  /**
   * Whichever queued task id is up next for the all-done popup — only
   * ever resolves while actively viewing *that exact task's* own subtask
   * board (not merely "some board with a queued task"), since
   * `checkAllDoneTransitions` below only ever queues an id while this
   * same condition already held. The explicit id match here is a second,
   * defensive guarantee of "move it to just the subtask view": if two
   * different parents both have queued popups, navigating to parent A's
   * board must never show parent B's dialog.
   */
  let allDoneDialogTask = $derived(
    allDoneQueueState.items.length > 0 && containerOwnerTask?.id === allDoneQueueState.items[0]
      ? containerOwnerTask
      : undefined,
  );

  /**
   * Re-checks for an all-subtasks-done transition, but only while
   * actively viewing the subtask container's own board (`containerOwnerTask`
   * is defined) — previously this ran unconditionally against
   * `visibleTasks` on *every* board, which meant the popup could fire
   * while viewing the parent's own project or the global "All Tasks" view
   * (wherever the parent task happened to be loaded), never the subtask
   * board itself. `visibleTasks` here is exactly the owner's own
   * subtasks (this board's entire content), so `[containerOwnerTask,
   * ...visibleTasks]` gives `checkAllDoneTransitions` everything it needs
   * to evaluate just this one task. The actual "already shown"/"never
   * ask again" tracking lives in `subtaskCompletionQueue.svelte.ts`'s
   * persisted module-level state, not here — see its own doc comment.
   */
  $effect(() => {
    if (!containerOwnerTask) return;
    checkAllDoneTransitions(
      [containerOwnerTask, ...visibleTasks],
      settingsState.current?.done_status,
      settingsState.current?.cancelled_status,
      formatDateISO(new Date()),
    );
  });

  /**
   * Marks the subtask board's owning parent task done — shared by the
   * all-done dialog's "Mark parent done" action and the persistent header
   * button that's always available on a subtask board regardless of
   * whether the dialog happens to be open. Only dequeues when responding
   * to an actually-open dialog; the header button can fire with nothing
   * queued at all, and must not silently drop some unrelated queued
   * popup if one exists.
   *
   * Once done, the subtask container's job is finished — it's disbanded
   * (subtasks moved back into the parent's own project, container
   * removed; see `deleteSubtaskContainer`) and the user is sent back to
   * the parent task's own board, since this one is about to stop
   * existing. Navigation only happens on success: if disbanding the
   * container fails, staying put with the error shown is more useful
   * than leaving the user on a board that's silently out of sync with
   * the backend.
   */
  async function handleMarkParentDone() {
    const task = allDoneDialogTask ?? containerOwnerTask;
    if (allDoneDialogTask) dequeueAllDone();
    const doneStatusId = settingsState.current?.done_status;
    if (!task || !doneStatusId) return;
    await handleUpdate({ ...task, status: doneStatusId });
    try {
      await deleteSubtaskContainer(task.id);
      await refreshProjects();
      await refreshTasks();
      errorMessage = "";
      await goto(task.project_id ? `/projects/${task.project_id}` : "/");
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to remove the subtask list");
    }
  }

  /** Deletes every current subtask of the popup's task — the backend auto-clears the now-empty container and the task's own `subtask_project_id` once the last one is gone (see `delete_task`'s cascade-cleanup), so no separate "delete container" step is needed here. */
  async function handleDeleteSubtaskList() {
    const task = allDoneDialogTask ?? containerOwnerTask;
    if (allDoneDialogTask) dequeueAllDone();
    if (!task) return;
    try {
      for (const subtask of subtasksOf(task, visibleTasks)) {
        await deleteTask(subtask.id);
        removeTask(subtask.id);
        removeCachedTask(subtask.id);
      }
      await refresh();
      errorMessage = "";
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to delete subtasks");
    }
  }

  function handleDismissAllDonePermanently() {
    if (allDoneDialogTask) dismissAllDonePermanently(allDoneDialogTask.id);
  }

  /**
   * Re-fetches this board's project-status-line stats, when this board is
   * scoped to a project — a no-op on the unfiltered "all tasks" view, which
   * has no single project to show stats for. Called explicitly after every
   * action that can change a stat's underlying numbers (task create/update/
   * delete, recurrence/finish-day bulk operations via `refresh()` itself,
   * and starting/stopping tracking) rather than reactively recomputing on
   * every unrelated state change — `get_project_status_stats` does real file
   * I/O and a SQLite query, so each call is a deliberate, explicit refresh
   * point, mirroring this codebase's existing `refreshTasks`/
   * `refreshActiveSessions` "explicit refresh after specific actions"
   * convention rather than wiring up a reactive `$effect`.
   */
  async function refreshStatusLine() {
    if (!projectFilter) return;
    await refreshProjectStatusStats(projectFilter, displayState.weekStartsOn);
  }

  async function refresh() {
    try {
      const allTasks = await listTasks();
      // Doubles as a `refreshTasks()` for the global cache — this fetch
      // already has the exact same data, so there's no reason for every
      // board load/reload to leave `tasksState` stale until some unrelated
      // `refreshTags` call happens to also fire.
      tasksState.items = allTasks;
      const visible = projectFilter
        ? allTasks.filter((task) => task.project_id !== undefined && rollupProjectIds.includes(task.project_id))
        : allTasks;
      visibleTasks = visible;
      const today = formatDateISO(new Date());
      const boardVisible = visible.filter(
        (task) =>
          !task.hidden && isVisibleOnBoard(task, today) && !isHiddenAsSubtask(task, visible, projectFilter),
      );
      buckets = groupByStatusAndPriority(boardVisible, priorities, boardStatusIds, groupByPriority);
      recomputeHasOther();
      errorMessage = "";
      void refreshStatusLine();
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to load tasks");
    } finally {
      isLoading = false;
    }
  }

  async function handleAddTask(parsed: ParsedTaskInput) {
    try {
      if (parsed.recurrence) {
        const tasks = await createRecurringTask(
          parsed,
          parsed.recurrence.frequency,
          parsed.recurrence.endDate,
          parsed.dueRule,
          parsed.projectId,
        );
        for (const task of tasks) replaceTask(task);
      } else {
        const task = await createTask(parsed, parsed.projectId);
        replaceTask(task);
      }
      // Creating a subtask (via the "Create Subtask" buttons' `parentTaskId`,
      // or a typed `sub <name>` token) also mutates its *parent* task
      // server-side (`ensureSubtaskContainer` sets `subtask_project_id`) and
      // creates a brand new container `Project`, but neither the parent's
      // locally-cached copy in `visibleTasks` nor the new container's entry
      // in the global `projectsState` (used by the container's own board
      // page to look itself up) sees that — `replaceTask` above only
      // patches the newly-created subtask itself. Without this, the new
      // subtask would render as a standalone card (nothing in the
      // locally-cached parent yet identifies it as a subtask) and clicking
      // through to the container's board would 404 ("Project not found"),
      // until some unrelated reload happened to refresh both caches. A full
      // refresh of both is the simplest fix, mirroring `confirmFinishDay`'s
      // identical reasoning for a server-side change that's impractical to
      // patch precisely client-side.
      if (subtaskParentIdForModal !== undefined || parsed.subtaskParentName !== undefined) {
        await Promise.all([refresh(), refreshProjects()]);
      }
      errorMessage = "";
      finishDayMessage = "";
      modalOpen = false;
      void refreshTags();
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to create task");
    }
  }

  /**
   * Extends every recurring series with at least one currently-loaded task
   * so it has occurrences generated through `through` — the scroll-
   * triggered lookahead, called from `WeekView`/`CalendarView` whenever the
   * visible range's end date changes. Each call is independently
   * idempotent on the backend (a `through` at or before what's already
   * generated is a no-op, and a series with recurrence removed is also a
   * no-op), so this doesn't need its own "have we already asked for this
   * date" cache.
   *
   * Series discovery relies entirely on `series_id`s already present in
   * `visibleTasks` — there's no separate "list all series" command yet. A
   * series whose *only* loaded occurrences are currently filtered out of
   * `visibleTasks` (e.g. a different project filter) silently stops being
   * extended. Acceptable today since `visibleTasks` is the full
   * project-filtered task list, not a windowed/paginated one, but would
   * need a real series list if loading ever becomes more selective.
   */
  async function ensureOccurrencesThrough(through: string) {
    const seriesIds = new Set(visibleTasks.map((task) => task.series_id).filter((id): id is string => !!id));
    if (seriesIds.size === 0) return;

    try {
      const results = await Promise.all(
        [...seriesIds].map((seriesId) => ensureOccurrencesUntil(seriesId, through)),
      );
      for (const tasks of results) {
        for (const task of tasks) replaceTask(task);
      }
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to extend recurring tasks");
    }
  }

  /** Removes a task from whichever bucket currently holds it, and from `visibleTasks`. */
  function removeTask(id: string) {
    buckets = removeTaskFromBuckets(buckets, id);
    visibleTasks = visibleTasks.filter((task) => task.id !== id);
    recomputeHasOther();
  }

  /**
   * Upserts a created/edited task into `visibleTasks` and `buckets`, and
   * always into the global `tasksState` cache too (`upsertCachedTask`) —
   * every other view's subtask-relationship lookups read that cache, not
   * this board's own `visibleTasks`, so without this a status/attribute
   * change here would only become visible elsewhere once some unrelated
   * `refreshTags`/`refreshTasks` call happened to fire and resolve. If this
   * board is scoped to a project and the task's `project` doesn't match,
   * it's removed from *this board's own view* — `removeTask` below — but
   * the task still exists, so the global cache keeps the just-upserted
   * copy regardless. A task that's in `visibleTasks` but not currently
   * `isVisibleOnBoard` (Item 3's future-`scheduled` rule) is left out of
   * `buckets` - it stays hidden from the Kanban grid but remains available
   * to the week view.
   */
  function replaceTask(updated: Task) {
    upsertCachedTask(updated);

    if (projectFilter && (updated.project_id === undefined || !rollupProjectIds.includes(updated.project_id))) {
      removeTask(updated.id);
      return;
    }

    const index = visibleTasks.findIndex((task) => task.id === updated.id);
    visibleTasks =
      index === -1
        ? [...visibleTasks, updated]
        : visibleTasks.map((task) => (task.id === updated.id ? updated : task));

    const withoutUpdated = removeTaskFromBuckets(buckets, updated.id);
    buckets =
      !updated.hidden &&
      isVisibleOnBoard(updated, formatDateISO(new Date())) &&
      !isHiddenAsSubtask(updated, visibleTasks, projectFilter)
        ? insertTaskIntoBuckets(withoutUpdated, updated, priorities, groupByPriority)
        : withoutUpdated;
    recomputeHasOther();
    void refreshStatusLine();
  }

  async function handleUpdate(task: Task, scope?: SeriesEditScope) {
    try {
      if (scope) {
        const updated = await updateSeriesOccurrence(task, scope);
        for (const t of updated) replaceTask(t);
      } else {
        const updated = await updateTask(task);
        replaceTask(updated);
      }
      errorMessage = "";
      finishDayMessage = "";
      void refreshTags();
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to update task");
    }
  }

  /** Toggles the timer for a task via vim `s` — mirrors TaskCard's `handleTrackingToggle`. */
  async function handleVimTimerToggle(taskId: string) {
    const task = visibleTasks.find((t) => t.id === taskId);
    if (!task) return;
    try {
      if (isTaskActive(taskId)) {
        const trackedMinutes = await stopTaskTracking(taskId);
        upsertCachedTask({ ...task, tracked_minutes: trackedMinutes });
      } else {
        const autoTransitioned = await startTaskTracking(taskId);
        if (autoTransitioned) await handleUpdate(autoTransitioned);
      }
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to update tracking");
    }
  }

  /** Changes the status of one or more tasks via vim status shortcuts. */
  async function handleVimStatusChange(taskIds: string[], statusId: string) {
    for (const taskId of taskIds) {
      const task = visibleTasks.find((t) => t.id === taskId);
      if (task) await handleUpdate({ ...task, status: statusId });
    }
  }

  /** Deletes a task via vim `D`, showing a ConfirmDialog instead of window.confirm. */
  function handleVimDeleteTask(taskId: string) {
    vimDeletePendingId = taskId;
    vimDeleteOpen = true;
  }

  /** Moves a batch of selected tasks one calendar day left or right (week/calendar view visual mode). */
  async function handleVimMoveTasksToAdjacentDay(taskIds: string[], direction: "left" | "right") {
    const delta = direction === "left" ? -1 : 1;
    const updates: Task[] = [];
    for (const taskId of taskIds) {
      const task = visibleTasks.find((t) => t.id === taskId);
      if (!task) continue;
      const dateStr = task.scheduled ?? task.due;
      if (!dateStr) continue;
      const d = new Date(dateStr + "T00:00:00");
      d.setDate(d.getDate() + delta);
      const newDate = d.toISOString().slice(0, 10);
      updates.push(task.scheduled ? { ...task, scheduled: newDate } : { ...task, due: newDate });
    }
    await Promise.all(updates.map((t) => handleUpdate(t)));
  }

  /** Moves a batch of selected tasks one status column left or right. */
  async function handleVimMoveTasksToAdjacentStatus(taskIds: string[], direction: "left" | "right") {
    const focusedTask = visibleTasks.find((t) => t.id === vimState.focusedTaskId);
    if (!focusedTask) return;
    const currentColIdx = boardColumns.findIndex((col) => col.id === focusedTask.status);
    if (currentColIdx === -1) return;
    const targetColIdx = direction === "left" ? currentColIdx - 1 : currentColIdx + 1;
    if (targetColIdx < 0 || targetColIdx >= boardColumns.length) return;
    const targetStatusId = boardColumns[targetColIdx].id;
    if (!targetStatusId) return; // "Other" column has no fixed status
    const updates = taskIds
      .map((id) => visibleTasks.find((t) => t.id === id))
      .filter((t): t is Task => t !== undefined)
      .map((t) => ({ ...t, status: targetStatusId }));
    await Promise.all(updates.map((t) => handleUpdate(t)));
  }

  /**
   * `scope: "future"` can delete an unbounded number of other occurrences
   * at once, so rather than track which ids those were, a full `refresh()`
   * reloads the task list afterward — mirroring `confirmFinishDay`'s own
   * bulk-operation pattern below. `scope: "this"`/no scope only ever
   * deletes the one task, so `removeTask` stays precise there.
   */
  async function handleDelete(id: string, scope?: SeriesEditScope) {
    try {
      if (scope) {
        await deleteSeriesOccurrence(id, scope);
        if (scope === "future") {
          await refresh();
        } else {
          removeTask(id);
          removeCachedTask(id);
          void refreshStatusLine();
        }
      } else {
        await deleteTask(id);
        removeTask(id);
        removeCachedTask(id);
        void refreshStatusLine();
      }
      errorMessage = "";
      finishDayMessage = "";
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to delete task");
    }
  }

  async function handleRemoveRecurrence(id: string) {
    try {
      await removeRecurrence(id);
      errorMessage = "";
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to remove recurrence");
    }
  }

  /**
   * Applies a recurrence-pattern edit (frequency/due rule/end date) from
   * `TaskEditDialog`'s recurrence builder. Unlike a `scope: "future"`
   * edit/delete (where precise local patching is possible — every
   * affected task id is known), this deletes and regenerates an unknown
   * set of tasks server-side, so a full `refresh()` is the only option,
   * the same way `confirmFinishDay`'s archive operation already needs one.
   */
  async function handleUpdateRecurrence(
    seriesId: string,
    cutoff: string,
    frequency: RecurrenceFrequency,
    dueRule: DueRule,
    endDate: string | undefined,
  ) {
    try {
      await updateSeriesRecurrence(seriesId, cutoff, frequency, dueRule, endDate);
      errorMessage = "";
      await refresh();
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to update recurrence");
    }
  }

  /** Archives every done/cancelled task across all projects, after the user confirms. */
  async function confirmFinishDay() {
    finishDayConfirmOpen = false;
    finishDayMessage = "";
    isFinishingDay = true;
    try {
      const result = await finishDay();
      finishDayMessage = formatFinishDayResult(result.archived_count);
      errorMessage = "";
      await refresh();
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to finish day");
    } finally {
      isFinishingDay = false;
    }
  }

  /**
   * `true` while this project's hidden tracker task has a currently-active
   * session. Before the first press, `project.tracking_task_id` doesn't
   * exist yet — `isTaskActive(undefined)` would never match any real
   * session's `task_id`, but the guard is explicit here for clarity since
   * the prop is genuinely absent rather than just an unmatched id.
   */
  const isProjectTracking = $derived(
    project?.tracking_task_id !== undefined && isTaskActive(project.tracking_task_id),
  );

  /** The hidden tracker task itself, for `liveDisplaySecondsFor`'s ticker — needs the task's own `tracked_minutes`, not just its id. */
  const projectTrackingTask = $derived(
    project?.tracking_task_id !== undefined
      ? tasksState.items.find((t) => t.id === project.tracking_task_id)
      : undefined,
  );

  /** Set while a project-tracking toggle call is in flight, so a second click can't fire an overlapping request. */
  let isProjectTrackingPending = $state(false);

  /**
   * Toggles tracking for the whole project currently being viewed. Only
   * meaningful when `project` is defined (no project-level tracking on the
   * unfiltered "all tasks" view) — gated in the template by only rendering
   * the button when `project` exists. The very first press has no
   * `tracking_task_id` yet; `start_project_tracking` lazily creates the
   * hidden tracker task server-side, so nothing special is needed here
   * beyond calling it.
   *
   * `stopProjectTracking` (the store function) already does a full
   * `refreshTasks()` internally before resolving, which re-fetches the
   * hidden tracker task with its corrected `tracked_minutes` already
   * written — so there's nothing further to apply to the cache here; doing
   * so manually on top would just be a second, redundant write racing the
   * same data.
   */
  async function handleProjectTrackingToggle() {
    if (!project) return;
    isProjectTrackingPending = true;
    errorMessage = "";
    try {
      if (isProjectTracking && project.tracking_task_id) {
        await stopProjectTracking(project.id);
      } else {
        await startProjectTracking(project.id);
      }
      // `total_time_tracked` only changes on stop/start, not while a timer
      // ticks live — see `statusLine.svelte`'s own doc comment on why this
      // bar isn't a per-second ticker. Refreshing here, rather than from a
      // reactive watch on `isProjectTracking`, keeps the same "explicit
      // refresh after a specific action" shape as every other call site.
      void refreshStatusLine();
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to update project tracking");
    } finally {
      isProjectTrackingPending = false;
    }
  }

  /** Returns the priority buckets for the column with the given status id, or `buckets.other` for the trailing "Other" column. */
  function bucketsForColumn(statusId: string | undefined): PriorityBucket[] {
    return statusId !== undefined ? buckets.byStatus[statusId] : buckets.other;
  }

  /**
   * Live preview while a drag is in progress; persistence happens on finalize.
   * Deliberately left unmapped (a task dragged across buckets keeps its
   * original `status`/`priority` here) so `handleFinalize` can diff against
   * it to detect cross-bucket moves that need persisting. `statusId` is
   * `undefined` for the trailing "Other" status column.
   */
  function handleConsider(
    statusId: string | undefined,
    bucketIndex: number,
    event: CustomEvent<DndEvent<Task>>,
  ) {
    bucketsForColumn(statusId)[bucketIndex].tasks = event.detail.items;
  }

  /**
   * Renumbers the dropped-into bucket sequentially (`index * ORDER_STEP`),
   * applying the bucket's status/priority to every task in it, and persists
   * `order`/`status`/`priority` only for tasks whose values actually changed -
   * i.e. the dragged task itself, plus any siblings whose position shifted.
   * `statusId` is `undefined` for the "Other" status column, in which case
   * each task's existing `status` is preserved (it has no single column to be
   * assigned to); likewise the bucket's `priorityId` is `undefined` for the
   * "Other" priority bucket and for the single bucket used when
   * `groupByPriority` is off, in which case `priority` is left untouched.
   * When `groupByPriority` is off, `sortBucketTasks` re-sorts the bucket by
   * priority rank (then `order`) afterwards, so a dropped task visually snaps
   * back into its own priority tier instead of staying wherever it landed.
   */
  async function handleFinalize(
    statusId: string | undefined,
    bucketIndex: number,
    event: CustomEvent<DndEvent<Task>>,
  ) {
    const target = bucketsForColumn(statusId);
    const bucket = target[bucketIndex];
    const before = event.detail.items;
    const after = renumberBucket(before, statusId, bucket.priorityId, ORDER_STEP);
    target[bucketIndex].tasks = sortBucketTasks(after, priorities, groupByPriority);
    recomputeHasOther();

    const changed = after.filter(
      (task, index) =>
        task.order !== before[index].order ||
        task.status !== before[index].status ||
        task.priority !== before[index].priority,
    );
    if (changed.length === 0) return;

    visibleTasks = visibleTasks.map((task) => changed.find((c) => c.id === task.id) ?? task);

    try {
      await Promise.all(
        changed.map((task) => reorderTask(task.id, task.order, task.status, bucket.priorityId)),
      );
      errorMessage = "";
      finishDayMessage = "";
      void refreshStatusLine();
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to reorder tasks");
      await refresh();
    }
  }

  /**
   * Opens the add-task modal with Ctrl+T (always works, even in vim mode — it
   * uses a modifier key so there is no conflict with single-letter vim bindings).
   * Also delegates to `vimState.handleKeydown` for all vim navigation: Escape
   * activates/deactivates vim mode; hjkl/G/gg/v/vv/Space navigate and select;
   * e/s/n/D/d/i/b/c trigger task actions and status changes.
   */
  function handleGlobalKeydown(event: KeyboardEvent) {
    const target = event.target as HTMLElement | null;
    const inEditField = target?.matches("input, textarea, [contenteditable='true']") ?? false;

    // While the vim delete confirm dialog is open, intercept keyboard so board vim keys
    // don't fire. h/l move focus between Cancel/Delete buttons; Escape closes the dialog.
    if (vimDeleteOpen) {
      if (event.key === "Escape") {
        event.preventDefault();
        vimDeleteOpen = false;
        vimDeletePendingId = null;
      } else if (event.key === "h" || event.key === "ArrowLeft") {
        event.preventDefault();
        const dialog = document.querySelector("dialog[open]");
        const buttons = dialog?.querySelectorAll("button");
        if (buttons?.length) (buttons[0] as HTMLButtonElement).focus();
      } else if (event.key === "l" || event.key === "ArrowRight") {
        event.preventDefault();
        const dialog = document.querySelector("dialog[open]");
        const buttons = dialog?.querySelectorAll("button");
        if (buttons?.length) (buttons[buttons.length - 1] as HTMLButtonElement).focus();
      }
      return;
    }

    // Ctrl+T: open add-task modal. Skip if typing in an input to avoid conflict.
    if (event.ctrlKey && event.key.toLowerCase() === "t") {
      if (!inEditField) {
        event.preventDefault();
        openAddTaskModal();
      }
      return;
    }

    // Vim handler: called for all non-Ctrl+T keys. The handler internally
    // checks `suspended` state (skips non-Escape when an input has focus)
    // and `vimEnabled` (returns false when vim is off).
    const consumed = vimState.handleKeydown(event, {
      boardColumns,
      activeView,
      vimEnabled: settingsState.current?.vim?.enabled ?? false,
      userBindings: settingsState.current?.vim?.keybindings ?? [],
      statusBindings: settingsState.current?.vim?.status_keybindings ?? [],
      statuses,
      sidebarItems: [
        { route: "/" },
        { route: "/dashboard" },
        ...sidebarProjectOrder(projectsState.items).map((p) => ({ route: `/projects/${p.id}` })),
      ],
      currentPageRoute: page.url.pathname,
      currentProjectId: projectFilter ?? null,
      editDialogOpen: vimEditTask !== undefined,
      onEditTask: (id) => {
        vimEditTask = visibleTasks.find((t) => t.id === id);
      },
      onToggleTimer: (id) => {
        void handleVimTimerToggle(id);
      },
      onNewTask: () => openAddTaskModal(),
      onDeleteTask: (id) => {
        handleVimDeleteTask(id);
      },
      onChangeStatus: (ids, statusId) => {
        void handleVimStatusChange(ids, statusId);
      },
      onMoveTasksToAdjacentStatus: (ids, dir) => {
        void handleVimMoveTasksToAdjacentStatus(ids, dir);
      },
      onMoveTasksToAdjacentDay: (ids, dir) => {
        void handleVimMoveTasksToAdjacentDay(ids, dir);
      },
      onToggleSidebarExpand: (projectId) => toggleExpanded(projectId),
      weekDays:
        activeView === "week"
          ? weekViewVisibleDays
          : activeView === "calendar"
            ? calendarViewVisibleDays
            : weekDays,
    });
    if (consumed) {
      event.preventDefault();
      event.stopPropagation();
    }
  }

  /** Suspends vim navigation while the user types in an input, textarea, or select. */
  function handleFocusIn(e: FocusEvent) {
    const target = e.target as HTMLElement | null;
    if (target?.matches("input, textarea, select, [contenteditable='true']")) vimState.suspend();
  }

  /** Resumes vim navigation when focus leaves an editable field or select. */
  function handleFocusOut(e: FocusEvent) {
    const target = e.target as HTMLElement | null;
    if (target?.matches("input, textarea, select, [contenteditable='true']")) vimState.resume();
  }

  onMount(() => {
    window.addEventListener("keydown", handleGlobalKeydown);
    window.addEventListener("focusin", handleFocusIn);
    window.addEventListener("focusout", handleFocusOut);

    // vim:set-tab is dispatched by vimState when ArrowLeft/ArrowRight are pressed
    function handleVimSetTab(e: Event) {
      const tab = (e as CustomEvent<string>).detail;
      if (tab === "board" || tab === "week" || tab === "calendar" || tab === "dashboard") {
        activeView = tab;
      }
    }
    document.addEventListener("vim:set-tab", handleVimSetTab);

    return () => {
      window.removeEventListener("keydown", handleGlobalKeydown);
      window.removeEventListener("focusin", handleFocusIn);
      window.removeEventListener("focusout", handleFocusOut);
      document.removeEventListener("vim:set-tab", handleVimSetTab);
    };
  });

  /**
   * Re-fetches whenever `projectFilter` changes (needed because navigating
   * between two `/projects/[id]` routes reuses this component instance
   * rather than remounting it), `priorities` changes (settings finish loading
   * after mount, or the user edits priority levels), `boardStatusIds` changes
   * (settings or the project's board configuration load or change), or
   * `groupByPriority` changes (the user flips the global display-settings
   * toggle) - all of these require regrouping `buckets` from scratch. Each
   * dependency must be read synchronously here (not just inside `refresh()`,
   * which is async) for Svelte's effect tracking to pick it up.
   */
  $effect(() => {
    void projectFilter;
    void priorities;
    void boardStatusIds;
    void groupByPriority;
    void refresh();
  });

  // Auto-focus: select the first board task when vim is active and either no task is focused
  // or the focused task is no longer visible (e.g. after switching projects).
  $effect(() => {
    if (vimState.active && activeView === "board") {
      const focusedId = vimState.focusedTaskId;
      const cols = boardColumns;
      const isVisible =
        focusedId !== null &&
        cols.some((col) => col.buckets.some((b) => b.tasks.some((t) => t.id === focusedId)));
      if (!isVisible) {
        vimState.setInitialFocus(cols);
      }
    }
  });

  // Auto-select first task in week/calendar view when vim activates, view switches, or period changes.
  // Uses only the dates currently visible in the child component so the cursor is always on-screen.
  // The guard `visibleDays.length > 0` also waits for the child's onDateStringsChange to fire on mount.
  $effect(() => {
    if (vimState.active && (activeView === "week" || activeView === "calendar")) {
      const visibleDays = activeView === "week" ? weekViewVisibleDays : calendarViewVisibleDays;
      if (visibleDays.length === 0) return;
      const focusedId = vimState.weekFocusedTaskId;
      const isVisible = focusedId !== null && visibleDays.some((d) => d.taskIds.includes(focusedId));
      if (!isVisible) {
        vimState.setWeekInitialFocus(visibleDays);
      }
    }
  });
</script>

<main class="page">
  <header class="page-header">
    <div class="brand">
      <h1 class="brand-mark">
        {#if accentColor}
          <span class="color-dot" style="background: {accentColor}" aria-hidden="true"></span>
        {/if}
        {title}
      </h1>
      {#if tagline}
        <p class="brand-tagline">{tagline}</p>
      {/if}
    </div>
    <div class="view-tabs" role="tablist" aria-label="Board view">
      <button
        type="button"
        class="view-tab"
        class:active={activeView === "board"}
        role="tab"
        aria-selected={activeView === "board"}
        onclick={() => (activeView = "board")}
      >
        Board
      </button>
      <button
        type="button"
        class="view-tab"
        class:active={activeView === "week"}
        role="tab"
        aria-selected={activeView === "week"}
        onclick={() => (activeView = "week")}
      >
        Week
      </button>
      <button
        type="button"
        class="view-tab"
        class:active={activeView === "calendar"}
        role="tab"
        aria-selected={activeView === "calendar"}
        onclick={() => (activeView = "calendar")}
      >
        Calendar
      </button>
      <button
        type="button"
        class="view-tab"
        class:active={activeView === "dashboard"}
        role="tab"
        aria-selected={activeView === "dashboard"}
        onclick={() => (activeView = "dashboard")}
      >
        Dashboard
      </button>
    </div>
    <div class="header-actions">
      {#if !projectFilter}
        <button
          type="button"
          class="finish-day-button"
          onclick={() => (finishDayConfirmOpen = true)}
          disabled={isFinishingDay}
        >
          {isFinishingDay ? "Finishing day…" : "Finish day"}
        </button>
      {/if}
      {#if containerOwnerTask}
        <button type="button" class="mark-parent-done-button" onclick={handleMarkParentDone}>
          Mark parent done
        </button>
      {/if}
      {#if project}
        <div class="project-tracking">
          {#if isProjectTracking && projectTrackingTask}
            <span class="project-tracking-ticker" title="Currently tracking this project">
              {formatHms(
                liveDisplaySecondsFor(
                  projectTrackingTask,
                  settingsState.current?.card_tracked_time_display ?? "total",
                ) ?? 0,
              )}
            </span>
          {/if}
          <button
            type="button"
            class="icon-button project-tracking-button"
            class:tracking-active={isProjectTracking}
            onclick={handleProjectTrackingToggle}
            disabled={isProjectTrackingPending}
            aria-label={isProjectTracking ? "Stop tracking project" : "Start tracking project"}
            title={isProjectTracking ? "Stop tracking project" : "Start tracking project"}
          >
            {#if isProjectTracking}
              <svg viewBox="0 0 16 16" width="16" height="16" fill="currentColor" aria-hidden="true">
                <rect x="3" y="2" width="3.5" height="12" rx="0.75" />
                <rect x="9.5" y="2" width="3.5" height="12" rx="0.75" />
              </svg>
            {:else}
              <svg viewBox="0 0 16 16" width="16" height="16" fill="currentColor" aria-hidden="true">
                <path d="M4 2.5v11l9.5-5.5z" />
              </svg>
            {/if}
          </button>
        </div>
        {#if !isSubtaskContainer}
        <a
          class="icon-button settings-link"
          href="/projects/{project.id}/settings"
          aria-label="Board settings"
          title="Board settings"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
          >
            <circle cx="12" cy="12" r="3" />
            <path
              d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
            />
          </svg>
        </a>
        {/if}
      {/if}
      <button
        type="button"
        class="icon-button add-task-button"
        onclick={() => openAddTaskModal()}
        aria-label="Add task"
        title="Add task (Ctrl+T)"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <line x1="12" y1="5" x2="12" y2="19" />
          <line x1="5" y1="12" x2="19" y2="12" />
        </svg>
      </button>
    </div>
  </header>

  {#if project}
    <ProjectStatusLine
      projectId={project.id}
      statusBarEnabledOverride={project.board.status_bar_enabled_override}
    />
  {:else if !projectFilter}
    <GlobalStatusBar />
  {/if}

  <AddTaskModal
    open={modalOpen}
    onClose={() => (modalOpen = false)}
    onSubmit={handleAddTask}
    {projectFilter}
    {errorMessage}
    allTasks={visibleTasks}
    parentTaskId={subtaskParentIdForModal}
  />

  {#if errorMessage && !modalOpen}
    <p class="error" role="alert">{errorMessage}</p>
  {/if}

  {#if finishDayMessage && !errorMessage}
    <p class="success" role="status">{finishDayMessage}</p>
  {/if}

  <ConfirmDialog
    open={finishDayConfirmOpen}
    title="Finish day?"
    message="Archive all completed and cancelled tasks across every project? They'll be moved out of the board but kept in an archive."
    confirmLabel="Finish day"
    onConfirm={confirmFinishDay}
    onCancel={() => (finishDayConfirmOpen = false)}
  />

  <ConfirmDialog
    open={vimDeleteOpen}
    title="Delete task?"
    message="This will permanently delete the task. This cannot be undone."
    confirmLabel="Delete"
    onConfirm={() => {
      vimDeleteOpen = false;
      if (vimDeletePendingId) {
        void handleDelete(vimDeletePendingId);
        vimDeletePendingId = null;
      }
    }}
    onCancel={() => {
      vimDeleteOpen = false;
      vimDeletePendingId = null;
    }}
  />

  <AllSubtasksDoneDialog
    open={allDoneDialogTask !== undefined}
    task={allDoneDialogTask}
    onMarkDone={handleMarkParentDone}
    onDeleteSubtasks={handleDeleteSubtaskList}
    onDismiss={dequeueAllDone}
    onDismissPermanently={handleDismissAllDonePermanently}
  />

  {#if isLoading}
    <p class="loading">Loading tasks…</p>
  {:else if activeView === "week"}
    <WeekView
      tasks={weekCalendarTasks}
      allTasks={tasksState.items}
      onUpdate={handleUpdate}
      onDelete={handleDelete}
      onRemoveRecurrence={handleRemoveRecurrence}
      onUpdateRecurrence={handleUpdateRecurrence}
      {showPreviousWeeksColumn}
      onEnsureOccurrences={ensureOccurrencesThrough}
      onCreateSubtask={openCreateSubtask}
      onDateStringsChange={(ds) => { weekViewDateStrings = ds; }}
    />
  {:else if activeView === "calendar"}
    <CalendarView
      tasks={weekCalendarTasks}
      allTasks={tasksState.items}
      onUpdate={handleUpdate}
      onDelete={handleDelete}
      onRemoveRecurrence={handleRemoveRecurrence}
      onUpdateRecurrence={handleUpdateRecurrence}
      onEnsureOccurrences={ensureOccurrencesThrough}
      onCreateSubtask={openCreateSubtask}
      onDateStringsChange={(ds) => { calendarViewDateStrings = ds; }}
    />
  {:else if activeView === "dashboard"}
    {#if projectFilter && project}
      <ProjectDashboardView
        projectId={projectFilter}
        projectColor={project.color}
        projectName={project.name}
      />
    {:else}
      <DashboardView projectId={null} />
    {/if}
  {:else}
    <div class="board-view-wrapper">
      <KanbanGrid
        {boardColumns}
        {groupByPriority}
        onConsider={handleConsider}
        onFinalize={handleFinalize}
        onUpdate={handleUpdate}
        onDelete={handleDelete}
        onRemoveRecurrence={handleRemoveRecurrence}
        allTasks={tasksState.items}
        onCreateSubtask={openCreateSubtask}
      />
    </div>
  {/if}

  <TaskEditDialog
    open={vimEditTask !== undefined}
    task={vimEditTask}
    onSave={(task, scope) => {
      void handleUpdate(task, scope);
      vimEditTask = undefined;
    }}
    onDelete={(id, scope) => {
      void handleDelete(id, scope);
      vimEditTask = undefined;
    }}
    onRemoveRecurrence={(id) => {
      void handleRemoveRecurrence(id);
      vimEditTask = undefined;
    }}
    onUpdateRecurrence={handleUpdateRecurrence}
    onCancel={() => (vimEditTask = undefined)}
    allTasks={tasksState.items}
    onCreateSubtask={openCreateSubtask}
  />
</main>

<style>
  .page {
    max-width: min(var(--board-width, 1200px), 95vw);
    margin: 0 auto;
    padding: var(--space-xl) var(--space-lg) var(--space-2xl);
  }

  .page-header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-lg);
    padding-bottom: var(--space-lg);
    margin-bottom: var(--space-xl);
    border-bottom: 1px solid var(--color-border);
  }

  /* Positioned wrapper for the board view so VimModeIndicator can use position: absolute */
  .board-view-wrapper {
    position: relative;
  }

  .brand {
    display: flex;
    flex-direction: column;
    gap: var(--space-3xs);
  }

  .brand-mark {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    margin: 0;
    font-size: var(--text-display);
    font-weight: 600;
    letter-spacing: var(--tracking-tight);
  }

  .color-dot {
    width: 0.75rem;
    height: 0.75rem;
    border-radius: var(--radius-pill);
    flex-shrink: 0;
  }

  .brand-tagline {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--color-ink-muted);
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
  }

  .view-tabs {
    display: inline-flex;
    gap: var(--space-4xs);
    padding: var(--space-4xs);
    border-radius: var(--radius-pill);
    background: var(--color-canvas);
    border: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .view-tab {
    padding: var(--space-2xs) var(--space-md);
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-ink-muted);
    font-weight: 600;
    font-size: var(--text-sm);
    cursor: pointer;
    transition:
      background var(--duration-fast) var(--ease-out-expo),
      color var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo);
  }

  .view-tab:hover {
    color: var(--color-ink);
  }

  .view-tab.active {
    background: var(--color-surface-raised);
    color: var(--color-ink);
    box-shadow: var(--shadow-sm);
  }

  .view-tab:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    flex-shrink: 0;
  }

  .icon-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2.5rem;
    height: 2.5rem;
    flex-shrink: 0;
    border-radius: var(--radius-md);
    border: none;
    cursor: pointer;
    box-shadow: var(--shadow-sm);
    transition:
      background var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo),
      transform var(--duration-fast) var(--ease-out-expo);
  }

  .icon-button:active {
    transform: translateY(0);
  }

  .add-task-button {
    background: var(--color-accent);
    color: var(--color-accent-ink);
  }

  .add-task-button:hover {
    background: var(--color-accent-hover);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .project-tracking {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    flex-shrink: 0;
  }

  .project-tracking-ticker {
    font-size: var(--text-sm);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    color: var(--color-accent);
  }

  .project-tracking-button {
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink-muted);
  }

  .project-tracking-button:hover {
    background: var(--color-canvas);
    color: var(--color-ink);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  /* While running, stays visibly "lit" in the accent color rather than only
     highlighting on hover — mirrors TaskCard's `.tracking-icon-button.tracking-active`. */
  .project-tracking-button.tracking-active {
    border-color: color-mix(in oklch, var(--color-accent) 50%, var(--color-border));
    background: var(--color-accent-soft);
    color: var(--color-accent);
  }

  .settings-link {
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink-muted);
    text-decoration: none;
  }

  .settings-link:hover {
    background: var(--color-canvas);
    color: var(--color-ink);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .finish-day-button {
    height: 2.5rem;
    padding: 0 var(--space-md);
    flex-shrink: 0;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    color: var(--color-ink-muted);
    font-weight: 600;
    font-size: var(--text-sm);
    cursor: pointer;
    box-shadow: var(--shadow-sm);
    transition:
      background var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo),
      transform var(--duration-fast) var(--ease-out-expo);
  }

  .finish-day-button:hover:not(:disabled) {
    background: var(--color-canvas);
    color: var(--color-ink);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .finish-day-button:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  .mark-parent-done-button {
    height: 2.5rem;
    padding: 0 var(--space-md);
    flex-shrink: 0;
    border: 1px solid color-mix(in oklch, var(--color-accent) 45%, transparent);
    border-radius: var(--radius-md);
    background: var(--color-accent-soft);
    color: var(--color-accent);
    font-weight: 600;
    font-size: var(--text-sm);
    cursor: pointer;
    box-shadow: var(--shadow-sm);
    transition:
      background var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo),
      transform var(--duration-fast) var(--ease-out-expo);
  }

  .mark-parent-done-button:hover {
    background: var(--color-accent);
    color: var(--color-accent-ink);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .error {
    margin: 0 0 var(--space-md);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    background: var(--color-danger-soft);
    color: var(--color-danger);
    font-weight: 600;
    font-size: var(--text-sm);
  }

  .success {
    margin: 0 0 var(--space-md);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    background: var(--color-accent-soft);
    color: var(--color-accent);
    font-weight: 600;
    font-size: var(--text-sm);
  }

  .loading {
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
  }
</style>
