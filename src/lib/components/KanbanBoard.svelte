<script lang="ts">
  import { onMount } from "svelte";
  import { dndzone, type DndEvent } from "svelte-dnd-action";
  import { createTask, deleteTask, listTasks, reorderTask, updateTask } from "$lib/api";
  import AddTaskModal from "$lib/components/AddTaskModal.svelte";
  import TaskCard from "$lib/components/TaskCard.svelte";
  import { displayState } from "$lib/displaySettings.svelte";
  import {
    bucketsHaveTasks,
    groupByStatusAndPriority,
    insertTaskIntoBuckets,
    OTHER_BUCKET_LABEL,
    removeTaskFromBuckets,
    renumberBucket,
    sortBucketTasks,
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
  const FLIP_DURATION_MS = 150;

  /**
   * A Kanban column: either a configured status (`id` set) or the trailing
   * "Other" status column (`id` undefined) for tasks whose status falls
   * outside the board's configured statuses.
   */
  interface BoardColumn {
    id: string | undefined;
    label: string;
    color: string;
    buckets: PriorityBucket[];
  }

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

  async function refresh() {
    try {
      const allTasks = await listTasks();
      const visible = projectFilter
        ? allTasks.filter((task) => task.project === projectFilter)
        : allTasks;
      buckets = groupByStatusAndPriority(visible, priorities, boardStatusIds, groupByPriority);
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
      const task = await createTask(parsed);
      if (!projectFilter || task.project === projectFilter) {
        buckets = insertTaskIntoBuckets(buckets, task, priorities, groupByPriority);
        recomputeHasOther();
      }
      errorMessage = "";
      modalOpen = false;
      void refreshTags();
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : "Failed to create task";
    }
  }

  /** Removes a task from whichever bucket currently holds it. */
  function removeTask(id: string) {
    buckets = removeTaskFromBuckets(buckets, id);
    recomputeHasOther();
  }

  /**
   * Moves a task to the bucket matching its (possibly new) status and
   * priority. If this board is scoped to a project and the edit moved the
   * task to a different project, it's removed from view instead.
   */
  function replaceTask(updated: Task) {
    if (projectFilter && updated.project !== projectFilter) {
      removeTask(updated.id);
      return;
    }
    buckets = insertTaskIntoBuckets(
      removeTaskFromBuckets(buckets, updated.id),
      updated,
      priorities,
      groupByPriority,
    );
    recomputeHasOther();
  }

  async function handleUpdate(task: Task) {
    try {
      const updated = await updateTask(task);
      replaceTask(updated);
      errorMessage = "";
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
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : "Failed to delete task";
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

    try {
      await Promise.all(
        changed.map((task) => reorderTask(task.id, task.order, task.status, bucket.priorityId)),
      );
      errorMessage = "";
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
    <div class="header-actions">
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

  {#if isLoading}
    <p class="loading">Loading tasks…</p>
  {:else}
    <div class="board">
      {#each boardColumns as column (column.id)}
        <section class="column" style="--status-accent: {column.color}">
          <h2>{column.label}</h2>
          {#each column.buckets as bucket, bucketIndex (bucket.priorityId ?? "other")}
            <div class="priority-group">
              {#if groupByPriority}
                <p class="priority-group-label" style="--priority-color: {bucket.color}">
                  {bucket.label}
                </p>
              {/if}
              <ul
                use:dndzone={{
                  items: bucket.tasks,
                  flipDurationMs: FLIP_DURATION_MS,
                  zoneItemTabIndex: -1,
                  // Empty object overrides (not merges with) the library default
                  // outline: 'rgba(255, 255, 102, 0.7) solid 2px' — disables it.
                  dropTargetStyle: {},
                }}
                onconsider={(event) => handleConsider(column.id, bucketIndex, event)}
                onfinalize={(event) => handleFinalize(column.id, bucketIndex, event)}
              >
                {#each bucket.tasks as task (task.id)}
                  <TaskCard {task} onUpdate={handleUpdate} onDelete={handleDelete} />
                {/each}
              </ul>
            </div>
          {/each}
        </section>
      {/each}
    </div>
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

  .error {
    margin: 0 0 var(--space-md);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    background: var(--color-danger-soft);
    color: var(--color-danger);
    font-weight: 600;
    font-size: var(--text-sm);
  }

  .loading {
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
  }

  .board {
    display: flex;
    align-items: flex-start;
    gap: var(--space-lg);
    overflow-x: auto;
    padding-bottom: var(--space-sm);
  }

  .column {
    position: relative;
    display: flex;
    flex-direction: column;
    flex: 0 0 var(--column-width, 240px);
    width: var(--column-width, 240px);
    gap: var(--space-sm);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-md);
    min-height: 200px;
  }

  .column::before {
    content: "";
    position: absolute;
    top: 0;
    left: var(--space-md);
    right: var(--space-md);
    height: 3px;
    border-radius: 0 0 var(--radius-pill) var(--radius-pill);
    background: var(--status-accent, var(--color-border-strong));
  }

  .column h2 {
    margin: 0;
    padding-top: var(--space-2xs);
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
  }

  .priority-group {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
  }

  .priority-group-label {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    margin: 0;
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-faint);
  }

  .priority-group-label::before {
    content: "";
    width: 0.5rem;
    height: 0.5rem;
    border-radius: var(--radius-pill);
    background: var(--priority-color, var(--color-border-strong));
    flex-shrink: 0;
  }

  .column ul {
    list-style: none;
    margin: 0;
    padding: 0;
    /* Tall enough that a dragged card's center can land inside this rect
       even when the bucket is empty (svelte-dnd-action computes drop
       targets via element-center-inside-collection-rect). */
    min-height: 2.5rem;
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .column ul:empty {
    border: 1px dashed var(--color-border-strong);
    border-radius: var(--radius-md);
  }
</style>
