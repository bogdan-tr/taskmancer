<script lang="ts">
  import { onMount } from "svelte";
  import { dndzone, type DndEvent } from "svelte-dnd-action";
  import { createTask, deleteTask, listTasks, reorderTask, updateTask } from "$lib/api";
  import AddTaskModal from "$lib/components/AddTaskModal.svelte";
  import TaskCard from "$lib/components/TaskCard.svelte";
  import type { ParsedTaskInput } from "$lib/naturalLanguage";
  import { refreshTags } from "$lib/tags.svelte";
  import { STATUS_LABELS, TASK_STATUSES, type Task, type TaskStatus } from "$lib/types";

  interface Props {
    title: string;
    tagline?: string;
    accentColor?: string;
    /** When set, only tasks whose `project` matches this name are shown. */
    projectFilter?: string;
  }

  let { title, tagline, accentColor, projectFilter }: Props = $props();

  /** Spacing between sequential `order` values when a column is renumbered after a drag. */
  const ORDER_STEP = 1000;
  const FLIP_DURATION_MS = 150;

  function emptyColumns(): Record<TaskStatus, Task[]> {
    return { backlog: [], do: [], "in-progress": [], blocked: [], done: [] };
  }

  function groupByStatus(allTasks: Task[]): Record<TaskStatus, Task[]> {
    const grouped = emptyColumns();
    for (const task of allTasks) {
      grouped[task.status].push(task);
    }
    return grouped;
  }

  let columns = $state<Record<TaskStatus, Task[]>>(emptyColumns());
  let isLoading = $state(true);
  let errorMessage = $state("");
  let modalOpen = $state(false);

  /** Pre-fills the add-task input with `+ProjectName ` when scoped to a project. */
  let addTaskPrefill = $derived(projectFilter ? `+${projectFilter} ` : "");

  async function refresh() {
    try {
      const allTasks = await listTasks();
      const visible = projectFilter
        ? allTasks.filter((task) => task.project === projectFilter)
        : allTasks;
      columns = groupByStatus(visible);
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
        columns[task.status] = [...columns[task.status], task];
      }
      errorMessage = "";
      modalOpen = false;
      void refreshTags();
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : "Failed to create task";
    }
  }

  /** Removes a task from whichever column currently holds it. */
  function removeTask(id: string) {
    for (const status of TASK_STATUSES) {
      columns[status] = columns[status].filter((task) => task.id !== id);
    }
  }

  /**
   * Replaces a task wherever it currently lives across all columns. If this
   * board is scoped to a project and the edit moved the task to a different
   * project, it's removed from view instead.
   */
  function replaceTask(updated: Task) {
    if (projectFilter && updated.project !== projectFilter) {
      removeTask(updated.id);
      return;
    }
    for (const status of TASK_STATUSES) {
      columns[status] = columns[status].map((task) => (task.id === updated.id ? updated : task));
    }
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

  /**
   * Live preview while a drag is in progress; persistence happens on finalize.
   * Deliberately left unmapped (a task dragged across columns keeps its
   * original `status` here) so `handleFinalize` can diff against it to
   * detect cross-column moves that need persisting.
   */
  function handleConsider(status: TaskStatus, event: CustomEvent<DndEvent<Task>>) {
    columns[status] = event.detail.items;
  }

  /**
   * Renumbers the dropped-into column sequentially (`index * ORDER_STEP`) and
   * persists `order`/`status` only for tasks whose values actually changed -
   * i.e. the dragged task itself, plus any siblings whose position shifted.
   */
  async function handleFinalize(status: TaskStatus, event: CustomEvent<DndEvent<Task>>) {
    const before = event.detail.items;
    const after = before.map((task, index) => ({
      ...task,
      order: index * ORDER_STEP,
      status,
    }));
    columns[status] = after;

    const changed = after.filter(
      (task, index) => task.order !== before[index].order || task.status !== before[index].status,
    );
    if (changed.length === 0) return;

    try {
      await Promise.all(changed.map((task) => reorderTask(task.id, task.order, status)));
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
   * Re-fetches whenever `projectFilter` changes. Needed because navigating
   * between two `/projects/[id]` routes reuses this component instance
   * rather than remounting it.
   */
  $effect(() => {
    void projectFilter;
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
      <button
        type="button"
        class="add-task-button"
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
    initialInput={addTaskPrefill}
    {errorMessage}
  />

  {#if errorMessage && !modalOpen}
    <p class="error" role="alert">{errorMessage}</p>
  {/if}

  {#if isLoading}
    <p class="loading">Loading tasks…</p>
  {:else}
    <div class="board">
      {#each TASK_STATUSES as status (status)}
        <section class="column" style="--status-accent: var(--color-status-{status})">
          <h2>{STATUS_LABELS[status]}</h2>
          <ul
            use:dndzone={{
              items: columns[status],
              flipDurationMs: FLIP_DURATION_MS,
              zoneItemTabIndex: -1,
              // Empty object overrides (not merges with) the library default
              // outline: 'rgba(255, 255, 102, 0.7) solid 2px' — disables it.
              dropTargetStyle: {},
            }}
            onconsider={(event) => handleConsider(status, event)}
            onfinalize={(event) => handleFinalize(status, event)}
          >
            {#each columns[status] as task (task.id)}
              <TaskCard {task} onUpdate={handleUpdate} onDelete={handleDelete} />
            {/each}
          </ul>
        </section>
      {/each}
    </div>
  {/if}
</main>

<style>
  .page {
    max-width: 1200px;
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

  .add-task-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2.5rem;
    height: 2.5rem;
    flex-shrink: 0;
    border-radius: var(--radius-md);
    border: none;
    background: var(--color-accent);
    color: var(--color-accent-ink);
    cursor: pointer;
    box-shadow: var(--shadow-sm);
    transition:
      background var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo),
      transform var(--duration-fast) var(--ease-out-expo);
  }

  .add-task-button:hover {
    background: var(--color-accent-hover);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .add-task-button:active {
    transform: translateY(0);
  }

  .error {
    margin: 0 0 var(--space-md);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    background: var(--color-danger-soft);
    color: var(--color-priority-high);
    font-weight: 600;
    font-size: var(--text-sm);
  }

  .loading {
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
  }

  .board {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: var(--space-lg);
  }

  .column {
    position: relative;
    display: flex;
    flex-direction: column;
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

  .column ul {
    list-style: none;
    margin: 0;
    padding: 0;
    /* Tall enough that a dragged card's center can land inside this rect
       even when the column is empty (svelte-dnd-action computes drop
       targets via element-center-inside-collection-rect). */
    min-height: 4.5rem;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .column ul:empty {
    align-items: center;
    justify-content: center;
    border: 1px dashed var(--color-border-strong);
    border-radius: var(--radius-md);
  }

  .column ul:empty::before {
    content: "Drop a task here";
    color: var(--color-ink-muted);
    font-size: var(--text-xs);
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
  }
</style>
