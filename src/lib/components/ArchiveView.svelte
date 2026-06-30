<script lang="ts">
  import { onMount } from "svelte";
  import { listArchivedTasks } from "$lib/api";
  import { projectsState } from "$lib/projects.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import { FALLBACK_STATUSES } from "$lib/statuses.svelte";
  import type { Task } from "$lib/types";
  import ArchiveTaskCard from "./ArchiveTaskCard.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";

  interface Props {
    onOpenDetail: (task: Task) => void;
    /** Called after the user confirms restore — the task has already been
     *  moved back to active by this point. */
    onRestore: (task: Task) => void;
  }

  let { onOpenDetail, onRestore }: Props = $props();

  // ─── Data ───────────────────────────────────────────────────────────────────

  let allTasks: Task[] = $state([]);
  let loading = $state(true);
  let loadError: string | undefined = $state(undefined);
  let restoring = $state(false);

  async function load() {
    loading = true;
    loadError = undefined;
    try {
      allTasks = await listArchivedTasks();
    } catch (e) {
      loadError = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void load();
  });

  // ─── Filters ─────────────────────────────────────────────────────────────────

  let searchText = $state("");
  let statusFilter: "all" | "done" | "cancelled" = $state("all");
  let selectedProjectId: string | "all" = $state("all");
  let dateFrom = $state("");
  let dateTo = $state("");
  let tagFilter = $state("");

  const statuses = $derived(settingsState.current?.statuses ?? FALLBACK_STATUSES);
  const doneStatus = $derived(settingsState.current?.done_status ?? "done");
  const cancelledStatus = $derived(settingsState.current?.cancelled_status ?? "cancelled");

  const filteredTasks = $derived(
    allTasks.filter((task) => {
      if (statusFilter === "done" && task.status !== doneStatus) return false;
      if (statusFilter === "cancelled" && task.status !== cancelledStatus) return false;
      if (selectedProjectId !== "all" && task.project_id !== selectedProjectId) return false;
      if (tagFilter) {
        const needle = tagFilter.toLowerCase().replace(/^#/, "");
        if (!task.tags.some((t) => t.toLowerCase().includes(needle))) return false;
      }
      if (dateFrom || dateTo) {
        const ts = task.archived_at ?? task.completed_at ?? task.cancelled_at ?? task.created;
        const day = ts.slice(0, 10);
        if (dateFrom && day < dateFrom) return false;
        if (dateTo && day > dateTo) return false;
      }
      if (searchText) {
        const q = searchText.toLowerCase();
        if (
          !task.title.toLowerCase().includes(q) &&
          !task.notes.toLowerCase().includes(q)
        )
          return false;
      }
      return true;
    }),
  );

  // ─── Grouping ────────────────────────────────────────────────────────────────

  /** Stable date string (YYYY-MM-DD) extracted from the best timestamp. */
  function taskDateKey(task: Task): string {
    const ts = task.archived_at ?? task.completed_at ?? task.cancelled_at ?? task.created;
    return ts.slice(0, 10);
  }

  function formatDateLabel(dateKey: string): string {
    const d = new Date(dateKey + "T12:00:00");
    return d.toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric", year: "numeric" });
  }

  interface DateGroup {
    dateKey: string;
    label: string;
    tasks: Task[];
  }

  interface ProjectGroup {
    projectId: string | undefined;
    projectName: string;
    taskCount: number;
    mostRecentDate: string;
    collapsed: boolean;
    dateGroups: DateGroup[];
  }

  let collapsedProjects = $state<Set<string>>(new Set());

  const grouped = $derived((): ProjectGroup[] => {
    const projectMap = new Map<string, Task[]>();
    for (const task of filteredTasks) {
      const key = task.project_id ?? "__no_project__";
      const existing = projectMap.get(key);
      if (existing) {
        existing.push(task);
      } else {
        projectMap.set(key, [task]);
      }
    }

    return [...projectMap.entries()]
      .map(([projectId, tasks]) => {
        const project = projectsState.items.find((p) => p.id === projectId);
        const projectName = projectId === "__no_project__" ? "No Project" : (project?.name ?? "Unknown Project");

        // Group tasks by date within this project (already sorted desc by backend)
        const dateMap = new Map<string, Task[]>();
        for (const task of tasks) {
          const dk = taskDateKey(task);
          const existing = dateMap.get(dk);
          if (existing) {
            existing.push(task);
          } else {
            dateMap.set(dk, [task]);
          }
        }

        const dateGroups: DateGroup[] = [...dateMap.entries()].map(([dateKey, dateTasks]) => ({
          dateKey,
          label: formatDateLabel(dateKey),
          tasks: dateTasks,
        }));

        const mostRecentDate = dateGroups[0]?.dateKey ?? "";

        return {
          projectId: projectId === "__no_project__" ? undefined : projectId,
          projectName,
          taskCount: tasks.length,
          mostRecentDate,
          collapsed: collapsedProjects.has(projectId),
          dateGroups,
        };
      })
      .sort((a, b) => b.mostRecentDate.localeCompare(a.mostRecentDate));
  });

  function toggleProject(projectId: string | undefined) {
    const key = projectId ?? "__no_project__";
    collapsedProjects = new Set(collapsedProjects);
    if (collapsedProjects.has(key)) {
      collapsedProjects.delete(key);
    } else {
      collapsedProjects.add(key);
    }
  }

  // ─── Flat task list for keyboard navigation ───────────────────────────────────

  const flatTasks = $derived(
    grouped().flatMap((pg) =>
      pg.collapsed ? [] : pg.dateGroups.flatMap((dg) => dg.tasks),
    ),
  );

  // ─── Keyboard navigation ─────────────────────────────────────────────────────

  let focusedIndex = $state(-1);
  let confirmRestoreTask: Task | undefined = $state(undefined);
  let searchInputEl: HTMLInputElement | undefined = $state();

  $effect(() => {
    // Reset focus when filter results change
    if (flatTasks.length === 0) focusedIndex = -1;
    else if (focusedIndex >= flatTasks.length) focusedIndex = flatTasks.length - 1;
  });

  function handleKeydown(e: KeyboardEvent) {
    const target = e.target as HTMLElement;
    const inInput = target?.matches("input, textarea");

    if (inInput) {
      if (e.key === "Escape") {
        (target as HTMLInputElement).blur();
        e.stopPropagation();
      }
      return;
    }

    if (e.key === "j" || e.key === "ArrowDown") {
      e.preventDefault();
      focusedIndex = Math.min(focusedIndex + 1, flatTasks.length - 1);
    } else if (e.key === "k" || e.key === "ArrowUp") {
      e.preventDefault();
      focusedIndex = Math.max(focusedIndex - 1, 0);
    } else if ((e.key === "e" || e.key === "Enter") && focusedIndex >= 0) {
      e.preventDefault();
      const task = flatTasks[focusedIndex];
      if (task) onOpenDetail(task);
    } else if (e.key === "r" && focusedIndex >= 0) {
      e.preventDefault();
      const task = flatTasks[focusedIndex];
      if (task) confirmRestoreTask = task;
    } else if (e.key === "/" || e.key === "f") {
      e.preventDefault();
      searchInputEl?.focus();
    }
  }

  // ─── Restore ─────────────────────────────────────────────────────────────────

  async function handleRestore(task: Task) {
    confirmRestoreTask = task;
  }

  async function confirmRestore() {
    const task = confirmRestoreTask;
    confirmRestoreTask = undefined;
    if (!task) return;
    restoring = true;
    try {
      allTasks = allTasks.filter((t) => t.id !== task.id);
      onRestore(task);
    } finally {
      restoring = false;
    }
  }

  // ─── Unique project list for filter dropdown ─────────────────────────────────

  const projectOptions = $derived(
    [
      { id: "all", name: "All projects" },
      ...projectsState.items
        .filter((p) => allTasks.some((t) => t.project_id === p.id))
        .map((p) => ({ id: p.id, name: p.name })),
      ...(allTasks.some((t) => !t.project_id)
        ? [{ id: "__no_project__", name: "No Project" }]
        : []),
    ],
  );
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="archive-view" onkeydown={handleKeydown}>
  <!-- Filter toolbar -->
  <div class="filter-bar">
    <div class="search-wrap">
      <span class="search-icon">⌕</span>
      <input
        bind:this={searchInputEl}
        type="search"
        class="search-input"
        placeholder="Search title or notes…"
        bind:value={searchText}
        aria-label="Search archived tasks"
      />
    </div>

    <select
      class="filter-select"
      bind:value={statusFilter}
      aria-label="Filter by status"
    >
      <option value="all">All statuses</option>
      <option value="done">Done</option>
      <option value="cancelled">Cancelled</option>
    </select>

    {#if projectOptions.length > 2}
      <select
        class="filter-select"
        bind:value={selectedProjectId}
        aria-label="Filter by project"
      >
        {#each projectOptions as opt}
          <option value={opt.id}>{opt.name}</option>
        {/each}
      </select>
    {/if}

    <input
      type="text"
      class="filter-select tag-input"
      placeholder="#tag"
      bind:value={tagFilter}
      aria-label="Filter by tag"
    />

    <div class="date-range">
      <input
        type="date"
        class="filter-select date-input"
        bind:value={dateFrom}
        title="Archived from"
        aria-label="Archived from date"
      />
      <span class="range-sep">–</span>
      <input
        type="date"
        class="filter-select date-input"
        bind:value={dateTo}
        title="Archived to"
        aria-label="Archived to date"
      />
    </div>
  </div>

  <!-- Content -->
  {#if loading}
    <div class="empty-state">Loading archive…</div>
  {:else if loadError}
    <div class="empty-state error">{loadError}</div>
  {:else if filteredTasks.length === 0 && allTasks.length === 0}
    <div class="empty-state">
      <span class="empty-icon">📦</span>
      <p>No archived tasks yet.</p>
      <p class="empty-hint">Use "Finish day" to archive completed and cancelled tasks.</p>
    </div>
  {:else if filteredTasks.length === 0}
    <div class="empty-state">No tasks match your filters.</div>
  {:else}
    <div class="groups">
      {#each grouped() as pg (pg.projectId ?? "__no_project__")}
        <div class="project-group">
          <button
            type="button"
            class="project-header"
            onclick={() => toggleProject(pg.projectId)}
          >
            <span class="collapse-arrow" class:collapsed={pg.collapsed}>▼</span>
            <span class="project-header-name">{pg.projectName}</span>
            <span class="project-meta">
              {pg.taskCount} task{pg.taskCount !== 1 ? "s" : ""}
              {#if pg.mostRecentDate}
                · most recent {formatDateLabel(pg.mostRecentDate)}
              {/if}
            </span>
          </button>

          {#if !pg.collapsed}
            <div class="date-groups">
              {#each pg.dateGroups as dg (dg.dateKey)}
                <div class="date-separator">
                  <span class="date-separator-label">{dg.label}</span>
                </div>
                {#each dg.tasks as task (task.id)}
                  {@const globalIdx = flatTasks.indexOf(task)}
                  <ArchiveTaskCard
                    {task}
                    focused={focusedIndex === globalIdx}
                    onOpenDetail={(t) => { focusedIndex = globalIdx; onOpenDetail(t); }}
                    onRestore={handleRestore}
                  />
                {/each}
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<ConfirmDialog
  open={confirmRestoreTask !== undefined}
  title="Restore task"
  message={`Restore "${confirmRestoreTask?.title}" back to your board? It will return to your default status.`}
  confirmLabel="Restore"
  onConfirm={confirmRestore}
  onCancel={() => (confirmRestoreTask = undefined)}
/>

<style>
  .archive-view {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    outline: none;
  }

  /* ── Filter bar ──────────────────────────────────────────────────────────── */

  .filter-bar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-sm);
  }

  .search-wrap {
    position: relative;
    flex: 1;
    min-width: 180px;
  }

  .search-icon {
    position: absolute;
    left: var(--space-sm);
    top: 50%;
    transform: translateY(-50%);
    color: var(--color-text-muted);
    pointer-events: none;
    font-size: 1rem;
  }

  .search-input {
    width: 100%;
    padding: var(--space-xs) var(--space-sm) var(--space-xs) calc(var(--space-sm) + 1.25rem);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    font-size: 0.875rem;
    box-sizing: border-box;
  }

  .search-input:focus {
    outline: none;
    border-color: var(--color-accent);
  }

  .filter-select {
    padding: var(--space-xs) var(--space-sm);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    font-size: 0.8125rem;
    cursor: pointer;
  }

  .filter-select:focus {
    outline: none;
    border-color: var(--color-accent);
  }

  .tag-input {
    width: 90px;
  }

  .date-range {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
  }

  .date-input {
    width: 130px;
    cursor: text;
  }

  .range-sep {
    color: var(--color-text-muted);
    font-size: 0.875rem;
  }

  /* ── Groups ──────────────────────────────────────────────────────────────── */

  .groups {
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
  }

  .project-group {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .project-header {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    background: none;
    border: none;
    border-bottom: 1px solid var(--color-border);
    padding: var(--space-xs) 0 var(--space-xs);
    cursor: pointer;
    text-align: left;
    width: 100%;
    color: var(--color-text);
  }

  .project-header:hover {
    color: var(--color-accent);
  }

  .collapse-arrow {
    font-size: 0.625rem;
    transition: transform 150ms;
    color: var(--color-text-muted);
  }

  .collapse-arrow.collapsed {
    transform: rotate(-90deg);
  }

  .project-header-name {
    font-weight: 600;
    font-size: 0.9375rem;
  }

  .project-meta {
    font-size: 0.75rem;
    color: var(--color-text-muted);
    margin-left: auto;
  }

  .date-groups {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-left: var(--space-sm);
  }

  /* ── Date separators ─────────────────────────────────────────────────────── */

  .date-separator {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    margin: var(--space-sm) 0 var(--space-xs);
  }

  .date-separator::before,
  .date-separator::after {
    content: "";
    flex: 1;
    height: 1px;
    background: var(--color-border);
  }

  .date-separator-label {
    font-size: 0.7rem;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--color-text-muted);
    white-space: nowrap;
  }

  /* ── Empty state ─────────────────────────────────────────────────────────── */

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-2xl) var(--space-xl);
    color: var(--color-text-muted);
    text-align: center;
    gap: var(--space-sm);
  }

  .empty-icon {
    font-size: 2.5rem;
    line-height: 1;
  }

  .empty-hint {
    font-size: 0.8125rem;
    opacity: 0.7;
  }

  .error {
    color: var(--color-danger, #e55);
  }
</style>
