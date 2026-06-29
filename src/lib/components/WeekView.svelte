<script lang="ts">
  import { onMount } from "svelte";
  import { dndzone, type DndEvent } from "svelte-dnd-action";
  import { displayState } from "$lib/displaySettings.svelte";
  import { FALLBACK_PRIORITIES } from "$lib/priorities.svelte";
  import type { DueRule, RecurrenceFrequency, SeriesEditScope } from "$lib/recurrence";
  import { settingsState } from "$lib/settings.svelte";
  import { FALLBACK_STATUSES, sortedStatuses } from "$lib/statuses.svelte";
  import type { Task } from "$lib/types";
  import { addDays, addWeeks, formatDateISO, startOfWeek, weekDates } from "$lib/weekRange";
  import {
    countTasksBeforeWeek,
    dedupeFinishedTaskBars,
    groupPreviousWeeksBars,
    groupTasksByWeek,
    type WeekBar,
  } from "$lib/weekGrouping";
  import { vimState } from "$lib/vim.svelte";
  import TaskEditDialog from "./TaskEditDialog.svelte";
  import WeekBarItem from "./WeekBarItem.svelte";

  const FLIP_DURATION_MS = 150;

  /** `WeekBar` plus an `id` — `svelte-dnd-action` requires every dndzone item to have one. */
  type DraggableWeekBar = WeekBar & { id: string };

  function toDraggable(weekBar: WeekBar): DraggableWeekBar {
    return { ...weekBar, id: barKey(weekBar) };
  }

  interface Props {
    /** Tasks to place on the week grid (project-filtered, but not Kanban-visibility-filtered). Excludes subtasks that shouldn't render as standalone bars — see `isHiddenAsSubtask`. */
    tasks: Task[];
    /** The global task list — for `TaskEditDialog`'s own subtask lookups (gating the "Create Subtask" button, the cascade-delete count, the recurrence read-only state). Must be global, not board-scoped — see `TaskCard.svelte`'s matching prop doc for why. */
    allTasks?: Task[];
    onUpdate: (task: Task, scope?: SeriesEditScope) => void | Promise<void>;
    onDelete: (id: string, scope?: SeriesEditScope) => void | Promise<void>;
    onRemoveRecurrence: (id: string) => void | Promise<void>;
    onUpdateRecurrence: (
      seriesId: string,
      cutoff: string,
      frequency: RecurrenceFrequency,
      dueRule: DueRule,
      endDate: string | undefined,
    ) => void | Promise<void>;
    /** Whether to show the leading "Previous" column of unfinished tasks scheduled/due before this week. */
    showPreviousWeeksColumn: boolean;
    /**
     * Called whenever the visible week's end date changes (navigating
     * forward, jumping to today, or the week-start-day setting realigning
     * the grid), with that date (`YYYY-MM-DD`) — the recurrence
     * lookahead-extension trigger: any recurring series with tasks already
     * loaded gets `ensure_occurrences_until` called up to this date, so
     * scrolling far enough into the future always finds occurrences
     * already generated rather than a gap.
     */
    onEnsureOccurrences?: (through: string) => void;
    /** Opens the Add Task modal pre-filled to create a subtask of the given task. */
    onCreateSubtask?: (task: Task) => void;
    /** Called whenever the set of visible date strings changes (week navigation, setting change). Used by KanbanBoard to keep vim cursor within the visible window. */
    onDateStringsChange?: (dateStrings: string[]) => void;
  }

  let {
    tasks,
    allTasks = [],
    onUpdate,
    onDelete,
    onRemoveRecurrence,
    onUpdateRecurrence,
    showPreviousWeeksColumn,
    onEnsureOccurrences,
    onCreateSubtask,
    onDateStringsChange,
  }: Props = $props();

  const priorities = $derived(settingsState.current?.priorities ?? FALLBACK_PRIORITIES);
  const statuses = $derived(sortedStatuses(settingsState.current?.statuses ?? FALLBACK_STATUSES));
  const doneStatus = $derived(settingsState.current?.done_status ?? "done");
  const cancelledStatus = $derived(settingsState.current?.cancelled_status);

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

  $effect(() => {
    onEnsureOccurrences?.(formatDateISO(weekEnd));
  });

  let dates = $derived(weekDates(weekStart));
  let dateStrings = $derived(dates.map(formatDateISO));
  let todayString = $derived(formatDateISO(new Date()));

  $effect(() => {
    onDateStringsChange?.(dateStrings);
  });

  /**
   * Mutable per-day mirror of `groupTasksByWeek`, kept in sync with `tasks`
   * via the effect below. Mutable (rather than plain `$derived`) so drag
   * handlers can write live preview state into it during a drag, the same
   * pattern `KanbanBoard.svelte` uses for `buckets`.
   */
  let weekColumns: DraggableWeekBar[][] = $state([]);

  $effect(() => {
    const grouped = groupTasksByWeek(tasks, dateStrings, priorities, doneStatus, cancelledStatus);
    const deduped = displayState.dedupeFinishedTasks
      ? dedupeFinishedTaskBars(grouped, doneStatus, cancelledStatus, displayState.dedupeFinishedTasksKeep)
      : grouped;
    weekColumns = deduped.map((dayBars) => dayBars.map(toDraggable));
  });

  /** Unfinished tasks scheduled/due before this week, for the optional leading column. Not draggable. */
  let previousBars = $derived(
    groupPreviousWeeksBars(tasks, dateStrings[0] ?? formatDateISO(weekStart), priorities, doneStatus, cancelledStatus),
  );

  /** Count for the header's "N tasks behind this week" indicator — always shown, regardless of the column toggle. */
  let behindCount = $derived(
    countTasksBeforeWeek(tasks, dateStrings[0] ?? formatDateISO(weekStart), doneStatus, cancelledStatus),
  );

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

  const isColorCoded = $derived(displayState.cardColorMode === "color_code");

  function barKey(bar: WeekBar): string {
    return `${bar.task.id}:${bar.type}`;
  }

  /** Live drag preview: mirrors the dropped-into column's items, exactly like `KanbanBoard.svelte`'s `handleConsider`. */
  function handleConsider(dayIndex: number, event: CustomEvent<DndEvent<DraggableWeekBar>>) {
    weekColumns[dayIndex] = event.detail.items;
  }

  /**
   * On drop, any bar in the target day whose own `date` doesn't match that
   * day's date string just moved there from elsewhere — update its task's
   * `scheduled` (for a "scheduled" bar) or `due` (for a "due" bar) to the
   * new day and persist. There's no "order" field to renumber (bars are
   * always auto-sorted by priority/title within a day), so this is simpler
   * than the Kanban board's equivalent.
   */
  async function handleFinalize(dayIndex: number, event: CustomEvent<DndEvent<DraggableWeekBar>>) {
    const items = event.detail.items;
    weekColumns[dayIndex] = items;

    const targetDate = dateStrings[dayIndex];
    const moved = items.filter((bar) => bar.date !== targetDate);
    if (moved.length === 0) return;

    for (const bar of moved) {
      const updated: Task =
        bar.type === "scheduled" ? { ...bar.task, scheduled: targetDate } : { ...bar.task, due: targetDate };
      await onUpdate(updated);
    }
  }

  let editingTask: Task | undefined = $state(undefined);
  let editDialogOpen = $state(false);

  /** The key (see `barKey`) of the bar whose popover is currently open, or `undefined` if none is. */
  let openBarKey: string | undefined = $state(undefined);

  function toggleBar(key: string) {
    openBarKey = openBarKey === key ? undefined : key;
  }

  function closePopover() {
    openBarKey = undefined;
  }

  function openEdit(task: Task) {
    closePopover();
    editingTask = task;
    editDialogOpen = true;
  }

  function closeEdit() {
    editDialogOpen = false;
    editingTask = undefined;
  }

  async function saveEdit(task: Task, scope?: SeriesEditScope) {
    await onUpdate(task, scope);
    closeEdit();
  }

  async function deleteEdit(id: string, scope?: SeriesEditScope) {
    await onDelete(id, scope);
    closeEdit();
  }

  /** Closes the open popover on a click outside any bar — in-bar clicks are handled by the bar's own button. */
  /** Ctrl+click multi-select (non-vim): toggle a task in/out of the local selection set. */
  let localSelectedIds = $state<Set<string>>(new Set());

  /** The effective selection for visual highlights — vim selection when vim is in visual mode, else local Ctrl+click selection. */
  let effectiveSelectedIds = $derived(
    vimState.active && (vimState.mode === "visual" || vimState.mode === "visual_sparse")
      ? vimState.selectedTaskIds
      : localSelectedIds,
  );

  function handleBarCtrlClick(taskId: string, event: MouseEvent) {
    if (!event.ctrlKey && !event.metaKey) return;
    event.preventDefault();
    event.stopPropagation();
    const next = new Set(localSelectedIds);
    if (next.has(taskId)) {
      next.delete(taskId);
    } else {
      next.add(taskId);
    }
    localSelectedIds = next;
  }

  function handleWindowClick(event: MouseEvent) {
    // Clear local selection when clicking on empty space (not a bar)
    if (!(event.target as HTMLElement).closest(".bar") && !event.ctrlKey && !event.metaKey) {
      if (localSelectedIds.size > 0) {
        localSelectedIds = new Set();
        return;
      }
    }
    if (openBarKey === undefined) return;
    if ((event.target as HTMLElement).closest(".bar")) return;
    closePopover();
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && openBarKey !== undefined) {
      closePopover();
    }
    // Clear local selection on Escape
    if (event.key === "Escape" && localSelectedIds.size > 0) {
      localSelectedIds = new Set();
    }
  }

  /**
   * The popover renders with `position: fixed` at viewport-clamped pixel
   * coordinates (see `WeekBarItem.svelte`), so it always stays fully
   * on-screen — but that also means it doesn't scroll along with its bar.
   * Rather than let it visually detach and float over unrelated content
   * during a scroll, just close it; reopening re-measures from the bar's
   * new position.
   */
  function handleWindowScroll() {
    if (openBarKey !== undefined) closePopover();
  }

  function handleVimNextPeriod() { goToNextWeek(); }
  function handleVimPrevPeriod() { goToPreviousWeek(); }

  onMount(() => {
    window.addEventListener("click", handleWindowClick);
    window.addEventListener("keydown", handleWindowKeydown);
    window.addEventListener("scroll", handleWindowScroll, { passive: true });
    document.addEventListener("vim:next-period", handleVimNextPeriod);
    document.addEventListener("vim:prev-period", handleVimPrevPeriod);
    return () => {
      window.removeEventListener("click", handleWindowClick);
      window.removeEventListener("keydown", handleWindowKeydown);
      window.removeEventListener("scroll", handleWindowScroll);
      document.removeEventListener("vim:next-period", handleVimNextPeriod);
      document.removeEventListener("vim:prev-period", handleVimPrevPeriod);
    };
  });
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
    {#if behindCount > 0}
      <p class="behind-indicator" title="Unfinished tasks scheduled or due before this week">
        {behindCount} {behindCount === 1 ? "task" : "tasks"} behind this week
      </p>
    {/if}
  </div>

  <div class="week-grid">
    {#if showPreviousWeeksColumn && previousBars.length > 0}
      <div class="day-column previous-column">
        <div class="day-header">
          <span class="day-name">Previous</span>
        </div>
        <div class="day-bars">
          {#each previousBars as weekBar (barKey(weekBar))}
            <WeekBarItem
              {weekBar}
              colorCoded={isColorCoded}
              rightAlignPopover={false}
              {priorities}
              {statuses}
              {doneStatus}
              {cancelledStatus}
              isOpen={openBarKey === barKey(weekBar)}
              onToggle={() => toggleBar(barKey(weekBar))}
              onClosePopover={closePopover}
              onEdit={openEdit}
            />
          {/each}
        </div>
      </div>
    {/if}
    {#each dates as date, index (dateStrings[index])}
      {@const dayItems = weekColumns[index] ?? []}
      <div
        class="day-column"
        class:is-today={dateStrings[index] === todayString}
        class:vim-day-focused={vimState.active && dateStrings[index] === vimState.weekFocusedDate}
      >
        <div class="day-header">
          <span class="day-name">{dayName(date)}</span>
          <span class="day-number">{date.getDate()}</span>
        </div>
        <div
          class="day-bars"
          use:dndzone={{
            items: dayItems,
            flipDurationMs: FLIP_DURATION_MS,
            zoneItemTabIndex: -1,
            dropTargetStyle: {},
          }}
          onconsider={(event) => handleConsider(index, event)}
          onfinalize={(event) => handleFinalize(index, event)}
        >
          {#each dayItems as weekBar (weekBar.id)}
            <div
              class:vim-task-focused={vimState.active && weekBar.task.id === vimState.weekFocusedTaskId}
              class:vim-task-selected={effectiveSelectedIds.has(weekBar.task.id)}
              role="none"
              onclick={(e) => handleBarCtrlClick(weekBar.task.id, e)}
            >
              <WeekBarItem
                {weekBar}
                colorCoded={isColorCoded}
                rightAlignPopover={index >= 5}
                {priorities}
                {statuses}
                {doneStatus}
                {cancelledStatus}
                isOpen={openBarKey === barKey(weekBar)}
                onToggle={() => toggleBar(barKey(weekBar))}
                onClosePopover={closePopover}
                onEdit={openEdit}
              />
            </div>
          {/each}
        </div>
      </div>
    {/each}
  </div>
</div>

<TaskEditDialog
  open={editDialogOpen}
  task={editingTask}
  onSave={saveEdit}
  onDelete={deleteEdit}
  {onRemoveRecurrence}
  {onUpdateRecurrence}
  onCancel={closeEdit}
  {allTasks}
  onCreateSubtask={onCreateSubtask
    ? (task) => {
        closeEdit();
        onCreateSubtask(task);
      }
    : undefined}
/>

<style>
  .week-view {
    position: relative;
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

  .behind-indicator {
    margin: 0;
    padding: var(--space-3xs) var(--space-sm);
    border-radius: var(--radius-pill);
    background: var(--color-urgent-soft);
    color: var(--color-urgent);
    font-size: var(--text-xs);
    font-weight: 700;
  }

  .week-grid {
    display: flex;
    align-items: flex-start;
    gap: var(--space-sm);
    overflow-x: auto;
  }

  .day-column {
    /* A fixed min-width (rather than 0) so adding the optional "Previous"
       column never squeezes the 7 real day columns into unreadable,
       character-per-line text — `.week-grid`'s overflow-x: auto scrolls
       horizontally instead once columns can't all fit. */
    flex: 1 1 9rem;
    min-width: 9rem;
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-sm);
  }

  .previous-column {
    flex: 0 0 12rem;
    background: var(--color-canvas);
    border-style: dashed;
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
    min-height: 2.5rem;
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .vim-day-focused {
    outline: 1px solid color-mix(in oklch, var(--color-accent) 40%, transparent);
    background: color-mix(in oklch, var(--color-accent) 5%, var(--color-surface));
  }

  .vim-task-focused {
    outline: 2px solid var(--color-accent);
    outline-offset: 1px;
    border-radius: var(--radius-sm);
  }

  .vim-task-selected {
    outline: 2px solid color-mix(in oklch, var(--color-accent) 70%, transparent);
    outline-offset: 1px;
    border-radius: var(--radius-sm);
    background: color-mix(in oklch, var(--color-accent) 8%, transparent);
  }
</style>
