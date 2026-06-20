<script lang="ts">
  import { displayState } from "$lib/displaySettings.svelte";
  import { addMonths, monthDates, startOfMonth } from "$lib/monthRange";
  import { resolveScheduledRelativeDate, SCHEDULED_RELATIVE_DATE_OPTIONS } from "$lib/relativeDates";
  import { formatDateISO } from "$lib/weekRange";

  const COLUMNS_PER_ROW = 7;

  interface Props {
    /** The currently selected date (`YYYY-MM-DD`), for highlighting in the grid. `undefined` if none. */
    selected?: string;
    /** aria-label/title for the trigger icon button, e.g. "Pick due date". */
    triggerLabel: string;
    /** Label for the clear action (e.g. "Never" for Due, "Clear" for Scheduled). Omitted hides the button. */
    clearLabel?: string;
    onSelect: (iso: string) => void;
    onClear?: () => void;
  }

  let { selected, triggerLabel, clearLabel, onSelect, onClear }: Props = $props();

  let open = $state(false);
  let dialogEl: HTMLDialogElement | undefined = $state();

  /** The month currently shown in the grid — reset to `selected`'s (or today's) month each time the dialog opens. */
  let visibleMonth = $state(startOfMonth(new Date()));

  function parseISODate(iso: string): Date {
    const [year, month, day] = iso.split("-").map(Number);
    return new Date(year, month - 1, day);
  }

  function openDialog() {
    visibleMonth = startOfMonth(selected ? parseISODate(selected) : new Date());
    open = true;
  }

  function goToPreviousMonth() {
    visibleMonth = addMonths(visibleMonth, -1);
  }

  function goToNextMonth() {
    visibleMonth = addMonths(visibleMonth, 1);
  }

  function selectDate(iso: string) {
    onSelect(iso);
    open = false;
  }

  function selectQuickPick(code: string) {
    const resolved = resolveScheduledRelativeDate(code, new Date());
    if (resolved) selectDate(resolved);
  }

  function handleClear() {
    onClear?.();
    open = false;
  }

  /**
   * `<dialog>` shown via `showModal()` is promoted to the browser's top
   * layer, the same as the task-edit/Add-Task dialogs this picker is
   * nested inside — top-layer elements stack by show order (most-recently
   * shown on top) regardless of z-index or DOM position, so opening this
   * dialog while one of those is already open correctly renders it above,
   * with no manual stacking/positioning work needed. It also centers
   * itself via plain CSS, so there's no anchor-relative position to
   * compute, clamp, or have clipped by a scrollable ancestor.
   */
  $effect(() => {
    if (!dialogEl) return;
    if (open) {
      if (!dialogEl.open) dialogEl.showModal();
    } else if (dialogEl.open) {
      dialogEl.close();
    }
  });

  /** Closes when a click lands on the `::backdrop`, not the dialog's content box — mirrors AddTaskModal/TaskEditDialog. */
  function handleBackdropClick(event: MouseEvent) {
    if (!dialogEl || event.target !== dialogEl) return;

    const rect = dialogEl.getBoundingClientRect();
    const insideContent =
      event.clientX >= rect.left &&
      event.clientX <= rect.right &&
      event.clientY >= rect.top &&
      event.clientY <= rect.bottom;

    if (!insideContent) dialogEl.close();
  }

  let dates = $derived(monthDates(visibleMonth, displayState.weekStartsOn));
  let dateStrings = $derived(dates.map(formatDateISO));
  let todayString = $derived(formatDateISO(new Date()));
  let weekdayLabels = $derived(dates.slice(0, COLUMNS_PER_ROW).map((date) => date.toLocaleDateString(undefined, { weekday: "short" })));
  let monthLabel = $derived(visibleMonth.toLocaleDateString(undefined, { month: "long", year: "numeric" }));
</script>

<button type="button" class="trigger" onclick={openDialog} aria-label={triggerLabel} aria-haspopup="dialog" title={triggerLabel}>
  <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <rect x="2" y="3" width="12" height="11" rx="1.5" />
    <path d="M2 6.5h12" />
    <path d="M5 1.5v3M11 1.5v3" />
  </svg>
</button>

<dialog
  bind:this={dialogEl}
  class="date-picker-dialog"
  aria-label={triggerLabel}
  onclose={() => (open = false)}
  onclick={handleBackdropClick}
>
  <div class="dialog-header">
    <p class="dialog-title">{triggerLabel}</p>
    <button type="button" class="close-button" onclick={() => dialogEl?.close()} aria-label="Close" title="Close">
      <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
        <path d="M3 3l10 10M13 3L3 13" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" />
      </svg>
    </button>
  </div>
  <div class="quick-picks">
    {#each SCHEDULED_RELATIVE_DATE_OPTIONS as option (option.id)}
      <button type="button" class="quick-pick" onclick={() => selectQuickPick(option.id)}>{option.label}</button>
    {/each}
    {#if clearLabel && onClear}
      <button type="button" class="quick-pick quick-pick-clear" onclick={handleClear}>{clearLabel}</button>
    {/if}
  </div>
  <div class="month-nav">
    <button type="button" class="nav-button" onclick={goToPreviousMonth} aria-label="Previous month">
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M15 18l-6-6 6-6" />
      </svg>
    </button>
    <p class="month-label">{monthLabel}</p>
    <button type="button" class="nav-button" onclick={goToNextMonth} aria-label="Next month">
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M9 18l6-6-6-6" />
      </svg>
    </button>
  </div>
  <div class="weekday-row">
    {#each weekdayLabels as label (label)}
      <span class="weekday-label">{label}</span>
    {/each}
  </div>
  <div class="month-grid">
    {#each dates as date, index (dateStrings[index])}
      {@const inCurrentMonth = date.getMonth() === visibleMonth.getMonth()}
      <button
        type="button"
        class="day-cell"
        class:is-today={dateStrings[index] === todayString}
        class:is-selected={dateStrings[index] === selected}
        class:is-outside-month={!inCurrentMonth}
        onclick={() => selectDate(dateStrings[index])}
      >
        {date.getDate()}
      </button>
    {/each}
  </div>
</dialog>

<style>
  .trigger {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 1.75rem;
    height: 1.75rem;
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink-muted);
    cursor: pointer;
    transition:
      color var(--duration-fast) var(--ease-out-expo),
      border-color var(--duration-fast) var(--ease-out-expo),
      background var(--duration-fast) var(--ease-out-expo);
  }

  .trigger:hover {
    color: var(--color-ink);
    border-color: var(--color-accent);
  }

  .trigger:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  /*
   * `svelte-dnd-action` (this trigger can sit inside a draggable Kanban
   * card) only exempts a mousedown from initiating a card drag when the
   * literal click target has a `.value` DOM property (its heuristic for
   * "this is a form control, not draggable content"). A `<button>` has
   * `.value` (an empty string, not `undefined`), so it qualifies — but the
   * icon `<svg>` inside it doesn't, so without this rule, clicks that
   * happen to land on the icon graphic (most of the visible button) get
   * treated as a potential drag instead of a click, and a few pixels of
   * ordinary mouse jitter during the click is enough to hijack it into a
   * drag attempt. `pointer-events: none` forces every click within the
   * button's hit area to register on the `<button>` itself.
   */
  .trigger svg {
    pointer-events: none;
  }

  .date-picker-dialog {
    padding: var(--space-lg);
    border: none;
    border-radius: var(--radius-lg);
    background: var(--color-surface-raised);
    color: var(--color-ink);
    box-shadow: var(--shadow-lg);
    width: min(24rem, calc(100vw - 2 * var(--space-lg)));
  }

  /* Higher specificity than `.date-picker-dialog` so a closed dialog stays
     `display: none` instead of the author-origin layout below overriding the
     UA stylesheet's `dialog:not([open]) { display: none }`. */
  .date-picker-dialog:not([open]) {
    display: none;
  }

  .date-picker-dialog::backdrop {
    background: oklch(20% 0.02 50 / 0.45);
  }

  .dialog-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-sm);
    margin-bottom: var(--space-md);
  }

  .dialog-title {
    margin: 0;
    font-size: var(--text-base);
    font-weight: 600;
    letter-spacing: var(--tracking-tight);
  }

  .close-button {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 1.75rem;
    height: 1.75rem;
    border-radius: var(--radius-md);
    border: none;
    background: transparent;
    color: var(--color-ink-muted);
    cursor: pointer;
    transition:
      color var(--duration-fast) var(--ease-out-expo),
      background var(--duration-fast) var(--ease-out-expo);
  }

  .close-button:hover {
    color: var(--color-ink);
    background: var(--color-canvas);
  }

  .close-button:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .quick-picks {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3xs);
    margin-bottom: var(--space-md);
  }

  .quick-pick {
    padding: var(--space-3xs) var(--space-xs);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-canvas);
    color: var(--color-ink);
    font-size: var(--text-xs);
    font-weight: 600;
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out-expo);
  }

  .quick-pick:hover {
    background: var(--color-accent-soft);
    color: var(--color-accent);
  }

  .quick-pick:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .quick-pick-clear {
    color: var(--color-ink-muted);
  }

  .month-nav {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2xs);
    margin-bottom: var(--space-sm);
  }

  .nav-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.75rem;
    height: 1.75rem;
    flex-shrink: 0;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-ink-muted);
    cursor: pointer;
    transition:
      color var(--duration-fast) var(--ease-out-expo),
      background var(--duration-fast) var(--ease-out-expo);
  }

  .nav-button:hover {
    color: var(--color-ink);
    background: var(--color-canvas);
  }

  .nav-button:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .month-label {
    margin: 0;
    font-size: var(--text-base);
    font-weight: 700;
  }

  .weekday-row,
  .month-grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: var(--space-3xs);
  }

  .weekday-row {
    margin-bottom: var(--space-3xs);
  }

  .weekday-label {
    text-align: center;
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--color-ink-muted);
  }

  .day-cell {
    aspect-ratio: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-ink);
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out-expo);
  }

  .day-cell:hover {
    background: var(--color-canvas);
  }

  .day-cell:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 1px;
  }

  .day-cell.is-outside-month {
    color: var(--color-ink-faint);
  }

  .day-cell.is-today {
    font-weight: 700;
    box-shadow: inset 0 0 0 1.5px var(--color-accent);
  }

  .day-cell.is-selected {
    background: var(--color-accent);
    color: var(--color-accent-ink);
    font-weight: 700;
  }
</style>
