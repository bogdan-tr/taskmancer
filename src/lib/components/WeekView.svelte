<script lang="ts">
  import { displayState } from "$lib/displaySettings.svelte";
  import { FALLBACK_PRIORITIES, priorityColor, priorityLabel } from "$lib/priorities.svelte";
  import { projectsState } from "$lib/projects.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import { FALLBACK_STATUSES, sortedStatuses, statusLabel } from "$lib/statuses.svelte";
  import { DEFAULT_PROJECT_COLOR, type Task } from "$lib/types";
  import { addDays, addWeeks, formatDateISO, startOfWeek, weekDates } from "$lib/weekRange";
  import { groupTasksByWeek } from "$lib/weekGrouping";
  import TaskEditDialog from "./TaskEditDialog.svelte";

  interface Props {
    /** Tasks to place on the week grid (project-filtered, but not Kanban-visibility-filtered). */
    tasks: Task[];
    onUpdate: (task: Task) => void | Promise<void>;
  }

  let { tasks, onUpdate }: Props = $props();

  const priorities = $derived(settingsState.current?.priorities ?? FALLBACK_PRIORITIES);
  const statuses = $derived(sortedStatuses(settingsState.current?.statuses ?? FALLBACK_STATUSES));

  let weekStart = $state(startOfWeek(new Date(), displayState.weekStartsOn));

  /**
   * Keeps `weekStart` aligned with the current week-start-day setting. The
   * equality check is required: `startOfWeek` is idempotent for an
   * already-aligned date, so without it this effect would re-assign a new
   * (but equal) Date every run and loop forever.
   */
  $effect(() => {
    const aligned = startOfWeek(weekStart, displayState.weekStartsOn);
    if (formatDateISO(aligned) !== formatDateISO(weekStart)) {
      weekStart = aligned;
    }
  });

  let weekEnd = $derived(addDays(weekStart, 6));
  let dates = $derived(weekDates(weekStart));
  let dateStrings = $derived(dates.map(formatDateISO));
  let bars = $derived(groupTasksByWeek(tasks, dateStrings));
  let todayString = $derived(formatDateISO(new Date()));

  function goToPreviousWeek() {
    weekStart = addWeeks(weekStart, -1);
  }

  function goToNextWeek() {
    weekStart = addWeeks(weekStart, 1);
  }

  function goToToday() {
    weekStart = startOfWeek(new Date(), displayState.weekStartsOn);
  }

  function dayName(date: Date): string {
    return date.toLocaleDateString(undefined, { weekday: "short" });
  }

  function formatWeekRangeLabel(start: Date, end: Date): string {
    const startLabel = start.toLocaleDateString(undefined, { month: "short", day: "numeric" });
    const endOptions: Intl.DateTimeFormatOptions =
      start.getFullYear() === end.getFullYear()
        ? { month: "short", day: "numeric" }
        : { month: "short", day: "numeric", year: "numeric" };
    const endLabel = end.toLocaleDateString(undefined, endOptions);
    return `${startLabel} – ${endLabel}, ${end.getFullYear()}`;
  }

  let weekRangeLabel = $derived(formatWeekRangeLabel(weekStart, weekEnd));

  /** The project's color, or `DEFAULT_PROJECT_COLOR` for tasks with no project or an unknown one. */
  function projectColor(projectName: string | undefined): string {
    if (!projectName) return DEFAULT_PROJECT_COLOR;
    return projectsState.items.find((p) => p.name === projectName)?.color ?? DEFAULT_PROJECT_COLOR;
  }

  let editingTask: Task | undefined = $state(undefined);
  let editDialogOpen = $state(false);

  /** Opens the edit dialog for `task`, closing the bar's `<details>` popover it was triggered from. */
  function openEdit(task: Task, event: MouseEvent) {
    (event.currentTarget as HTMLElement).closest("details")?.removeAttribute("open");
    editingTask = task;
    editDialogOpen = true;
  }

  function closeEdit() {
    editDialogOpen = false;
    editingTask = undefined;
  }

  async function saveEdit(task: Task) {
    await onUpdate(task);
    closeEdit();
  }
</script>

<div class="week-view">
  <div class="week-header">
    <div class="nav-buttons">
      <button type="button" class="nav-button" onclick={goToPreviousWeek} aria-label="Previous week">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <path d="M15 18l-6-6 6-6" />
        </svg>
      </button>
      <button type="button" class="nav-button today-button" onclick={goToToday}>Today</button>
      <button type="button" class="nav-button" onclick={goToNextWeek} aria-label="Next week">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <path d="M9 18l6-6-6-6" />
        </svg>
      </button>
    </div>
    <p class="week-range-label">{weekRangeLabel}</p>
  </div>

  <div class="week-grid">
    {#each dates as date, index (dateStrings[index])}
      <div class="day-column" class:is-today={dateStrings[index] === todayString}>
        <div class="day-header">
          <span class="day-name">{dayName(date)}</span>
          <span class="day-number">{date.getDate()}</span>
        </div>
        <div class="day-bars">
          {#each bars[index] as bar (bar.task.id + ":" + bar.type)}
            <details class="bar" style="--bar-color: {projectColor(bar.task.project)}">
              <summary
                class="bar-summary"
                aria-label="{bar.type === 'scheduled' ? 'Scheduled' : 'Due'} – {bar.task.title}"
              >
                <span class="bar-icon" aria-hidden="true">
                  {#if bar.type === "scheduled"}
                    <svg viewBox="0 0 16 16" width="10" height="10" aria-hidden="true">
                      <circle cx="8" cy="8" r="6" fill="currentColor" />
                    </svg>
                  {:else}
                    <svg
                      viewBox="0 0 16 16"
                      width="10"
                      height="10"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="1.5"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      aria-hidden="true"
                    >
                      <path d="M3 1.5v13" />
                      <path d="M3 2h8l-1.5 3L11 8H3" />
                    </svg>
                  {/if}
                </span>
                <span class="bar-title">{bar.task.title}</span>
              </summary>
              <div class="bar-popover">
                <p class="popover-title">{bar.task.title}</p>
                <div class="popover-meta">
                  {#if bar.task.project}
                    <span class="chip project" style="--chip-color: {projectColor(bar.task.project)}">
                      {bar.task.project}
                    </span>
                  {/if}
                  <span class="chip priority" style="--chip-color: {priorityColor(priorities, bar.task.priority)}">
                    <span class="priority-dot" aria-hidden="true"></span>
                    {priorityLabel(priorities, bar.task.priority)}
                  </span>
                  <span class="chip status">{statusLabel(statuses, bar.task.status)}</span>
                </div>
                {#if bar.task.scheduled || bar.task.due}
                  <dl class="popover-dates">
                    {#if bar.task.scheduled}
                      <div class="date-row">
                        <dt>Scheduled</dt>
                        <dd>{bar.task.scheduled}</dd>
                      </div>
                    {/if}
                    {#if bar.task.due}
                      <div class="date-row">
                        <dt>Due</dt>
                        <dd>{bar.task.due}</dd>
                      </div>
                    {/if}
                  </dl>
                {/if}
                <button type="button" class="edit-button" onclick={(event) => openEdit(bar.task, event)}>
                  Edit
                </button>
              </div>
            </details>
          {/each}
        </div>
      </div>
    {/each}
  </div>
</div>

<TaskEditDialog open={editDialogOpen} task={editingTask} onSave={saveEdit} onCancel={closeEdit} />

<style>
  .week-view {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .week-header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-md);
  }

  .nav-buttons {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
  }

  .nav-button {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 2.25rem;
    min-width: 2.25rem;
    padding: 0 var(--space-sm);
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
      color var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo),
      transform var(--duration-fast) var(--ease-out-expo);
  }

  .nav-button:hover {
    background: var(--color-canvas);
    color: var(--color-ink);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .nav-button:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .week-range-label {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-ink-muted);
    font-variant-numeric: tabular-nums;
  }

  .week-grid {
    display: flex;
    align-items: flex-start;
    gap: var(--space-sm);
  }

  .day-column {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-sm);
  }

  .day-column.is-today {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 1px var(--color-accent);
  }

  .day-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-2xs);
    padding-bottom: var(--space-2xs);
    border-bottom: 1px solid var(--color-border);
  }

  .day-name {
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
  }

  .day-column.is-today .day-name {
    color: var(--color-accent);
  }

  .day-number {
    font-size: var(--text-sm);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    color: var(--color-ink);
  }

  .day-column.is-today .day-number {
    color: var(--color-accent);
  }

  .day-bars {
    display: flex;
    flex-direction: column;
    gap: var(--space-3xs);
    min-height: 1.5rem;
  }

  .bar {
    position: relative;
  }

  .bar-summary {
    display: flex;
    align-items: center;
    gap: var(--space-3xs);
    padding: var(--space-3xs) var(--space-2xs);
    border-radius: var(--radius-sm);
    border-left: 3px solid var(--bar-color, var(--color-border-strong));
    background: color-mix(in oklch, var(--bar-color, var(--color-border-strong)) 14%, var(--color-surface));
    font-size: var(--text-xs);
    line-height: var(--leading-tight);
    color: var(--color-ink);
    cursor: pointer;
    list-style: none;
    transition:
      box-shadow var(--duration-fast) var(--ease-out-expo),
      transform var(--duration-fast) var(--ease-out-expo);
  }

  .bar-summary::-webkit-details-marker {
    display: none;
  }

  .bar-summary::marker {
    content: "";
  }

  .bar-summary:hover {
    box-shadow: var(--shadow-sm);
    transform: translateY(-1px);
  }

  .bar-summary:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .bar[open] > .bar-summary {
    box-shadow: 0 0 0 2px var(--bar-color, var(--color-accent));
  }

  .bar-icon {
    display: flex;
    align-items: center;
    color: var(--bar-color, var(--color-ink-muted));
    flex-shrink: 0;
  }

  .bar-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .bar-popover {
    position: absolute;
    top: calc(100% + var(--space-3xs));
    left: 0;
    z-index: 20;
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    width: 14rem;
    padding: var(--space-sm);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface-raised);
    box-shadow: var(--shadow-lg);
  }

  .day-column:nth-last-child(-n + 2) .bar-popover {
    left: auto;
    right: 0;
  }

  .popover-title {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: 600;
    line-height: var(--leading-tight);
    word-break: break-word;
  }

  .popover-meta {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-4xs);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-3xs);
    font-size: var(--text-xs);
    line-height: var(--leading-tight);
    padding: var(--space-4xs) var(--space-2xs);
    border-radius: var(--radius-pill);
    background: var(--color-canvas);
    border: 1px solid var(--color-border);
    color: var(--color-ink-muted);
  }

  .chip.project {
    background: color-mix(in oklch, var(--chip-color, var(--color-accent)) 18%, var(--color-surface-raised));
    border-color: var(--chip-color, var(--color-accent));
    color: var(--color-ink);
    font-weight: 600;
  }

  .chip.priority {
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    font-weight: 600;
  }

  .chip.status {
    font-weight: 600;
  }

  .priority-dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: var(--radius-pill);
    background: var(--chip-color, var(--color-border-strong));
    flex-shrink: 0;
  }

  .popover-dates {
    display: flex;
    flex-direction: column;
    gap: var(--space-3xs);
    margin: 0;
    font-size: var(--text-xs);
  }

  .date-row {
    display: flex;
    justify-content: space-between;
    gap: var(--space-sm);
  }

  .date-row dt {
    color: var(--color-ink-muted);
  }

  .date-row dd {
    margin: 0;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .edit-button {
    align-self: flex-end;
    padding: var(--space-3xs) var(--space-md);
    border: none;
    border-radius: var(--radius-md);
    background: var(--color-accent);
    color: var(--color-accent-ink);
    font-weight: 600;
    font-size: var(--text-xs);
    cursor: pointer;
    box-shadow: var(--shadow-sm);
    transition:
      background var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo),
      transform var(--duration-fast) var(--ease-out-expo);
  }

  .edit-button:hover {
    background: var(--color-accent-hover);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .edit-button:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }
</style>
