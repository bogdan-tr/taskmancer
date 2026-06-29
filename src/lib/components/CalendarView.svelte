<script lang="ts">
  import { onMount } from "svelte";
  import { dndzone, type DndEvent } from "svelte-dnd-action";
  import { displayState } from "$lib/displaySettings.svelte";
  import { addMonths, monthDates, startOfMonth } from "$lib/monthRange";
  import { FALLBACK_PRIORITIES } from "$lib/priorities.svelte";
  import type { DueRule, RecurrenceFrequency, SeriesEditScope } from "$lib/recurrence";
  import { settingsState } from "$lib/settings.svelte";
  import { FALLBACK_STATUSES, sortedStatuses } from "$lib/statuses.svelte";
  import type { Task } from "$lib/types";
  import { dedupeFinishedTaskBars, groupTasksByWeek, type WeekBar } from "$lib/weekGrouping";
  import { formatDateISO } from "$lib/weekRange";
  import { vimState } from "$lib/vim.svelte";
  import TaskEditDialog from "./TaskEditDialog.svelte";
  import WeekBarItem from "./WeekBarItem.svelte";

  const FLIP_DURATION_MS = 150;
  const COLUMNS_PER_ROW = 7;
  /** Columns at this index or later are close enough to the grid's right edge that the popover should open leftward instead. */
  const RIGHT_ALIGN_COLUMN_THRESHOLD = 5;

  /** `WeekBar` plus an `id` — `svelte-dnd-action` requires every dndzone item to have one. */
  type DraggableWeekBar = WeekBar & { id: string };

  function toDraggable(weekBar: WeekBar): DraggableWeekBar {
    return { ...weekBar, id: barKey(weekBar) };
  }

  interface Props {
    /** Tasks to place on the month grid (project-filtered, but not Kanban-visibility-filtered). Excludes subtasks that shouldn't render as standalone bars — see `isHiddenAsSubtask`. */
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
    /**
     * Called whenever the visible month grid's last date changes (navigating
     * forward, jumping to today, or the week-start-day setting realigning
     * the grid), with that date (`YYYY-MM-DD`) — see `WeekView.svelte`'s
     * matching prop for the full rationale.
     */
    onEnsureOccurrences?: (through: string) => void;
    /** Opens the Add Task modal pre-filled to create a subtask of the given task. */
    onCreateSubtask?: (task: Task) => void;
    /** Called whenever the set of visible date strings changes (month navigation, setting change). Used by KanbanBoard to keep vim cursor within the visible window. */
    onDateStringsChange?: (dateStrings: string[]) => void;
  }

  let {
    tasks,
    allTasks = [],
    onUpdate,
    onDelete,
    onRemoveRecurrence,
    onUpdateRecurrence,
    onEnsureOccurrences,
    onCreateSubtask,
    onDateStringsChange,
  }: Props = $props();

  const priorities = $derived(settingsState.current?.priorities ?? FALLBACK_PRIORITIES);
  const statuses = $derived(sortedStatuses(settingsState.current?.statuses ?? FALLBACK_STATUSES));
  const doneStatus = $derived(settingsState.current?.done_status ?? "done");
  const cancelledStatus = $derived(settingsState.current?.cancelled_status);

  let monthStart = $state(startOfMonth(new Date()));

  /**
   * Keeps `monthStart` aligned to the 1st of its month and re-derives the
   * grid below from the current week-start-day setting. `startOfMonth` is
   * idempotent for an already-aligned date, so the equality check (as in
   * `WeekView.svelte`'s analogous effect) prevents an infinite reassignment
   * loop.
   */
  $effect(() => {
    const aligned = startOfMonth(monthStart);
    if (formatDateISO(aligned) !== formatDateISO(monthStart)) {
      monthStart = aligned;
    }
  });

  let dates = $derived(monthDates(monthStart, displayState.weekStartsOn));
  let dateStrings = $derived(dates.map(formatDateISO));

  $effect(() => {
    const lastDate = dateStrings[dateStrings.length - 1];
    if (lastDate) onEnsureOccurrences?.(lastDate);
  });

  $effect(() => {
    onDateStringsChange?.(dateStrings);
  });

  let todayString = $derived(formatDateISO(new Date()));
  let weekdayLabels = $derived(dates.slice(0, COLUMNS_PER_ROW).map((date) => dayName(date)));

  /**
   * Mutable per-day mirror of `groupTasksByWeek`, kept in sync with `tasks`
   * via the effect below. Mutable (rather than plain `$derived`) so drag
   * handlers can write live preview state into it during a drag — the same
   * pattern `WeekView.svelte` and `KanbanBoard.svelte` use.
   */
  let dayColumns: DraggableWeekBar[][] = $state([]);

  $effect(() => {
    const grouped = groupTasksByWeek(tasks, dateStrings, priorities, doneStatus, cancelledStatus);
    const deduped = displayState.dedupeFinishedTasks
      ? dedupeFinishedTaskBars(grouped, doneStatus, cancelledStatus, displayState.dedupeFinishedTasksKeep)
      : grouped;
    dayColumns = deduped.map((dayBars) => dayBars.map(toDraggable));
  });

  function goToPreviousMonth() {
    monthStart = addMonths(monthStart, -1);
  }

  function goToNextMonth() {
    monthStart = addMonths(monthStart, 1);
  }

  function goToToday() {
    monthStart = startOfMonth(new Date());
  }

  function dayName(date: Date): string {
    return date.toLocaleDateString(undefined, { weekday: "short" });
  }

  function formatMonthLabel(date: Date): string {
    return date.toLocaleDateString(undefined, { month: "long", year: "numeric" });
  }

  let monthLabel = $derived(formatMonthLabel(monthStart));
  const isColorCoded = $derived(displayState.cardColorMode === "color_code");

  function barKey(bar: WeekBar): string {
    return `${bar.task.id}:${bar.type}`;
  }

  /** Live drag preview: mirrors the dropped-into column's items, exactly like `WeekView.svelte`'s equivalent. */
  function handleConsider(dayIndex: number, event: CustomEvent<DndEvent<DraggableWeekBar>>) {
    dayColumns[dayIndex] = event.detail.items;
  }

  /**
   * On drop, any bar in the target day whose own `date` doesn't match that
   * day's date string just moved there from elsewhere — update its task's
   * `scheduled` (for a "scheduled" bar) or `due` (for a "due" bar) to the
   * new day and persist. Mirrors `WeekView.svelte`'s `handleFinalize`.
   */
  async function handleFinalize(dayIndex: number, event: CustomEvent<DndEvent<DraggableWeekBar>>) {
    const items = event.detail.items;
    dayColumns[dayIndex] = items;

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

  /** Ctrl+click multi-select (non-vim): toggle a task in/out of the local selection set. */
  let localSelectedIds = $state<Set<string>>(new Set());

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

  /** Closes the open popover on a click outside any bar — in-bar clicks are handled by the bar's own button. */
  function handleWindowClick(event: MouseEvent) {
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
    if (event.key === "Escape" && localSelectedIds.size > 0) {
      localSelectedIds = new Set();
    }
  }

  /** Closes the popover on scroll, mirroring `WeekView.svelte` — see `WeekBarItem.svelte` for why (fixed-position popovers don't scroll with their bar). */
  function handleWindowScroll() {
    if (openBarKey !== undefined) closePopover();
  }

  function handleVimNextPeriod() { goToNextMonth(); }
  function handleVimPrevPeriod() { goToPreviousMonth(); }

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

<div class="calendar-view">
  <div class="calendar-header">
    <div class="nav-buttons">
      <button type="button" class="nav-button" onclick={goToPreviousMonth} aria-label="Previous month">
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
      <button type="button" class="nav-button" onclick={goToNextMonth} aria-label="Next month">
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
    <p class="month-label">{monthLabel}</p>
  </div>

  <div class="weekday-row">
    {#each weekdayLabels as label (label)}
      <span class="weekday-label">{label}</span>
    {/each}
  </div>

  <div class="month-grid">
    {#each dates as date, index (dateStrings[index])}
      {@const dayItems = dayColumns[index] ?? []}
      {@const inCurrentMonth = date.getMonth() === monthStart.getMonth()}
      <div
        class="day-cell"
        class:is-today={dateStrings[index] === todayString}
        class:is-outside-month={!inCurrentMonth}
      >
        <div class="day-header">
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
            {@const isSelected =
              (vimState.active &&
                (vimState.mode === "visual" || vimState.mode === "visual_sparse") &&
                vimState.selectedTaskIds.has(weekBar.task.id)) ||
              localSelectedIds.has(weekBar.task.id)}
            {@const isFocused = vimState.active && weekBar.task.id === vimState.weekFocusedTaskId}
            <div
              class:vim-task-focused={isFocused}
              class:vim-task-selected={isSelected}
              role="none"
              onclick={(e) => handleBarCtrlClick(weekBar.task.id, e)}
            >
              <WeekBarItem
                {weekBar}
                colorCoded={isColorCoded}
                rightAlignPopover={index % COLUMNS_PER_ROW >= RIGHT_ALIGN_COLUMN_THRESHOLD}
                {priorities}
                {statuses}
                {doneStatus}
                {cancelledStatus}
                isOpen={openBarKey === barKey(weekBar)}
                onToggle={() => toggleBar(barKey(weekBar))}
                onClosePopover={closePopover}
                onEdit={openEdit}
                muted={!inCurrentMonth}
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
  .calendar-view {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .calendar-header {
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

  .month-label {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-ink-muted);
    font-variant-numeric: tabular-nums;
  }

  .weekday-row {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: var(--space-sm);
    padding: 0 var(--space-2xs);
  }

  .weekday-label {
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
    text-align: center;
  }

  .month-grid {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    grid-auto-rows: minmax(7rem, auto);
    gap: var(--space-sm);
  }

  .day-cell {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
    min-width: 0;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-sm);
  }

  /* No `opacity` here: this is an ancestor of each day's `WeekBarItem`,
     and CSS `opacity` dims an entire descendant subtree regardless of
     `position: fixed` — including a bar's *popover* once opened, which
     should always render at normal opacity. The day number and each bar's
     own summary are muted individually instead (`.day-number`'s own rule
     below; `WeekBarItem`'s `muted` prop, scoped to just `.bar-summary`). */
  .day-cell.is-outside-month {
    background: var(--color-canvas);
  }

  .day-cell.is-outside-month .day-number {
    opacity: 0.6;
  }

  .day-cell.is-today {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 1px var(--color-accent);
  }

  .day-header {
    display: flex;
    align-items: baseline;
    justify-content: flex-end;
    padding-bottom: var(--space-2xs);
    border-bottom: 1px solid var(--color-border);
  }

  .day-number {
    font-size: var(--text-sm);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    color: var(--color-ink);
  }

  .day-cell.is-today .day-number {
    color: var(--color-accent);
  }

  .day-bars {
    display: flex;
    flex-direction: column;
    gap: var(--space-3xs);
    min-height: 2.5rem;
    flex: 1;
    list-style: none;
    margin: 0;
    padding: 0;
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
