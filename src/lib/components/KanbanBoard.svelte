<script lang="ts">
  import { onMount } from "svelte";
  import type { DndEvent } from "svelte-dnd-action";
  import {
    createRecurringTask,
    createTask,
    deleteTask,
    ensureOccurrencesUntil,
    finishDay,
    listTasks,
    reorderTask,
    updateTask,
  } from "$lib/api";
  import { isVisibleOnBoard } from "$lib/boardVisibility";
  import AddTaskModal from "$lib/components/AddTaskModal.svelte";
  import CalendarView from "$lib/components/CalendarView.svelte";
  import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
  import KanbanGrid from "$lib/components/KanbanGrid.svelte";
  import WeekView from "$lib/components/WeekView.svelte";
  import { displayState } from "$lib/displaySettings.svelte";
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
  import { projectsState } from "$lib/projects.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import {
    FALLBACK_STATUS_COLOR,
    FALLBACK_STATUSES,
    sortedStatuses,
    statusColor,
    statusLabel,
  } from "$lib/statuses.svelte";
  import { refreshTags } from "$lib/tags.svelte";
  import type { Task } from "$lib/types";
  import { formatDateISO } from "$lib/weekRange";

  interface Props {
    title: string;
    tagline?: string;
    accentColor?: string;
    /** When set, only tasks whose `project` matches this name are shown. */
    projectFilter?: string;
  }

  let { title, tagline, accentColor, projectFilter }: Props = $props();

  /** Spacing between sequential `order` values when a bucket is renumbered after a drag. */
  const ORDER_STEP = 1000;

  let priorities = $derived(settingsState.current?.priorities ?? FALLBACK_PRIORITIES);
  let statuses = $derived(sortedStatuses(settingsState.current?.statuses ?? FALLBACK_STATUSES));

  /** Whether Kanban columns divide tasks into priority groups, from the global display settings. */
  let groupByPriority = $derived(displayState.showPriorityGroups);

  /**
   * The current project, looked up by name (case-insensitively, mirroring
   * `find_project_board` in the Rust command layer) when this board is scoped
   * to a project via `projectFilter`.
   */
  let project = $derived(
    projectFilter
      ? projectsState.items.find((p) => p.name.toLowerCase() === projectFilter.toLowerCase())
      : undefined,
  );

  /**
   * The status ids shown as columns on this board, in display order: the
   * project's configured board subset if it has one, otherwise every status
   * in the global list.
   */
  let boardStatusIds = $derived(
    project && project.board.statuses.length > 0
      ? project.board.statuses
      : statuses.map((status) => status.id),
  );

  /** This project's `board.show_previous_weeks` override if set, else the global default. */
  let showPreviousWeeksColumn = $derived(
    project?.board.show_previous_weeks ?? settingsState.current?.show_previous_weeks_column ?? false,
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
  let finishDayConfirmOpen = $state(false);
  let finishDayMessage = $state("");
  let isFinishingDay = $state(false);

  /** Which view this board shows: the Kanban grid or the calendar week view. */
  let activeView: "board" | "week" | "calendar" = $state("board");

  /**
   * Every task visible on this board (project-filtered, but not subject to
   * Item 3's future-`scheduled` visibility rule) - passed to `WeekView`,
   * which shows scheduled/due bars regardless of whether a task is currently
   * hidden from the Kanban grid.
   */
  let visibleTasks: Task[] = $state([]);

  async function refresh() {
    try {
      const allTasks = await listTasks();
      const visible = projectFilter
        ? allTasks.filter((task) => task.project === projectFilter)
        : allTasks;
      visibleTasks = visible;
      const today = formatDateISO(new Date());
      const boardVisible = visible.filter((task) => isVisibleOnBoard(task, today));
      buckets = groupByStatusAndPriority(boardVisible, priorities, boardStatusIds, groupByPriority);
      recomputeHasOther();
      errorMessage = "";
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : "Failed to load tasks";
    } finally {
      isLoading = false;
    }
  }

  async function handleAddTask(parsed: ParsedTaskInput) {
    try {
      if (parsed.recurrence) {
        const tasks = await createRecurringTask(parsed, parsed.recurrence.frequency, parsed.recurrence.endDate);
        for (const task of tasks) replaceTask(task);
      } else {
        const task = await createTask(parsed);
        replaceTask(task);
      }
      errorMessage = "";
      finishDayMessage = "";
      modalOpen = false;
      void refreshTags();
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : "Failed to create task";
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
      errorMessage = error instanceof Error ? error.message : "Failed to extend recurring tasks";
    }
  }

  /** Removes a task from whichever bucket currently holds it, and from `visibleTasks`. */
  function removeTask(id: string) {
    buckets = removeTaskFromBuckets(buckets, id);
    visibleTasks = visibleTasks.filter((task) => task.id !== id);
    recomputeHasOther();
  }

  /**
   * Upserts a created/edited task into `visibleTasks` and `buckets`. If this
   * board is scoped to a project and the task's `project` doesn't match, it's
   * removed from view entirely instead. A task that's in `visibleTasks` but
   * not currently `isVisibleOnBoard` (Item 3's future-`scheduled` rule) is
   * left out of `buckets` - it stays hidden from the Kanban grid but remains
   * available to the week view.
   */
  function replaceTask(updated: Task) {
    if (projectFilter && updated.project !== projectFilter) {
      removeTask(updated.id);
      return;
    }

    const index = visibleTasks.findIndex((task) => task.id === updated.id);
    visibleTasks =
      index === -1
        ? [...visibleTasks, updated]
        : visibleTasks.map((task) => (task.id === updated.id ? updated : task));

    const withoutUpdated = removeTaskFromBuckets(buckets, updated.id);
    buckets = isVisibleOnBoard(updated, formatDateISO(new Date()))
      ? insertTaskIntoBuckets(withoutUpdated, updated, priorities, groupByPriority)
      : withoutUpdated;
    recomputeHasOther();
  }

  async function handleUpdate(task: Task) {
    try {
      const updated = await updateTask(task);
      replaceTask(updated);
      errorMessage = "";
      finishDayMessage = "";
      void refreshTags();
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : "Failed to update task";
    }
  }

  async function handleDelete(id: string) {
    try {
      await deleteTask(id);
      removeTask(id);
      errorMessage = "";
      finishDayMessage = "";
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : "Failed to delete task";
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
      errorMessage = error instanceof Error ? error.message : "Failed to finish day";
    } finally {
      isFinishingDay = false;
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
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : "Failed to reorder tasks";
      await refresh();
    }
  }

  /**
   * Opens the add-task modal with Ctrl+T, from anywhere on the page.
   * Skipped while focus is in an editable field (e.g. a task's title edit
   * input) so the shortcut doesn't yank the user out of an in-progress edit.
   */
  function handleGlobalKeydown(event: KeyboardEvent) {
    const target = event.target as HTMLElement | null;
    if (target?.matches("input, textarea, [contenteditable='true']")) return;

    if (event.ctrlKey && event.key.toLowerCase() === "t") {
      event.preventDefault();
      modalOpen = true;
    }
  }

  onMount(() => {
    window.addEventListener("keydown", handleGlobalKeydown);
    return () => window.removeEventListener("keydown", handleGlobalKeydown);
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
      {#if project}
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
      <button
        type="button"
        class="icon-button add-task-button"
        onclick={() => (modalOpen = true)}
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

  <AddTaskModal
    open={modalOpen}
    onClose={() => (modalOpen = false)}
    onSubmit={handleAddTask}
    {projectFilter}
    {errorMessage}
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

  {#if isLoading}
    <p class="loading">Loading tasks…</p>
  {:else if activeView === "week"}
    <WeekView
      tasks={visibleTasks}
      onUpdate={handleUpdate}
      {showPreviousWeeksColumn}
      onEnsureOccurrences={ensureOccurrencesThrough}
    />
  {:else if activeView === "calendar"}
    <CalendarView tasks={visibleTasks} onUpdate={handleUpdate} onEnsureOccurrences={ensureOccurrencesThrough} />
  {:else}
    <KanbanGrid
      {boardColumns}
      {groupByPriority}
      onConsider={handleConsider}
      onFinalize={handleFinalize}
      onUpdate={handleUpdate}
      onDelete={handleDelete}
    />
  {/if}
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
