<script lang="ts">
  import { onMount } from "svelte";
  import { displayState } from "$lib/displaySettings.svelte";
  import { addMonths, monthDates, startOfMonth } from "$lib/monthRange";
  import { computeClampedPopoverPosition } from "$lib/popoverPosition";
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
    /** Right-aligns the popover instead of left-aligning, for triggers close to the right edge. */
    rightAlign?: boolean;
    onSelect: (iso: string) => void;
    onClear?: () => void;
  }

  let { selected, triggerLabel, clearLabel, rightAlign = false, onSelect, onClear }: Props = $props();

  let open = $state(false);
  let triggerEl: HTMLButtonElement | undefined = $state();
  let popoverEl: HTMLDivElement | undefined = $state();
  let popoverPosition: { top: number; left: number } | undefined = $state();

  /** The month currently shown in the grid — reset to `selected`'s (or today's) month each time the popover opens. */
  let visibleMonth = $state(startOfMonth(new Date()));

  function parseISODate(iso: string): Date {
    const [year, month, day] = iso.split("-").map(Number);
    return new Date(year, month - 1, day);
  }

  function toggle() {
    if (!open) {
      visibleMonth = startOfMonth(selected ? parseISODate(selected) : new Date());
    }
    open = !open;
  }

  function close() {
    open = false;
  }

  function goToPreviousMonth() {
    visibleMonth = addMonths(visibleMonth, -1);
  }

  function goToNextMonth() {
    visibleMonth = addMonths(visibleMonth, 1);
  }

  function selectDate(iso: string) {
    onSelect(iso);
    close();
  }

  function selectQuickPick(code: string) {
    const resolved = resolveScheduledRelativeDate(code, new Date());
    if (resolved) selectDate(resolved);
  }

  function handleClear() {
    onClear?.();
    close();
  }

  /** Closes on a click outside the trigger and popover. */
  function handleWindowClick(event: MouseEvent) {
    if (!open) return;
    const target = event.target as HTMLElement;
    if (triggerEl?.contains(target) || popoverEl?.contains(target)) return;
    close();
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && open) close();
  }

  /** Position is `position: fixed` and doesn't track scroll — close rather than let it visually detach. */
  function handleWindowScroll() {
    if (open) close();
  }

  onMount(() => {
    window.addEventListener("click", handleWindowClick);
    window.addEventListener("keydown", handleWindowKeydown);
    window.addEventListener("scroll", handleWindowScroll, { passive: true });
    return () => {
      window.removeEventListener("click", handleWindowClick);
      window.removeEventListener("keydown", handleWindowKeydown);
      window.removeEventListener("scroll", handleWindowScroll);
    };
  });

  $effect(() => {
    if (!open || !triggerEl || !popoverEl) {
      popoverPosition = undefined;
      return;
    }
    const triggerRect = triggerEl.getBoundingClientRect();
    const popoverRect = popoverEl.getBoundingClientRect();
    popoverPosition = computeClampedPopoverPosition(triggerRect, popoverRect, {
      rightAlign,
      viewportWidth: window.innerWidth,
      viewportHeight: window.innerHeight,
    });
  });

  let dates = $derived(monthDates(visibleMonth, displayState.weekStartsOn));
  let dateStrings = $derived(dates.map(formatDateISO));
  let todayString = $derived(formatDateISO(new Date()));
  let weekdayLabels = $derived(dates.slice(0, COLUMNS_PER_ROW).map((date) => date.toLocaleDateString(undefined, { weekday: "short" })));
  let monthLabel = $derived(visibleMonth.toLocaleDateString(undefined, { month: "long", year: "numeric" }));
</script>

<button
  type="button"
  class="trigger"
  bind:this={triggerEl}
  onclick={toggle}
  aria-label={triggerLabel}
  aria-expanded={open}
  title={triggerLabel}
>
  <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <rect x="2" y="3" width="12" height="11" rx="1.5" />
    <path d="M2 6.5h12" />
    <path d="M5 1.5v3M11 1.5v3" />
  </svg>
</button>

{#if open}
  <div
    class="popover"
    role="dialog"
    aria-label={triggerLabel}
    bind:this={popoverEl}
    style={popoverPosition
      ? `position: fixed; top: ${popoverPosition.top}px; left: ${popoverPosition.left}px;`
      : ""}
  >
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
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M15 18l-6-6 6-6" />
        </svg>
      </button>
      <p class="month-label">{monthLabel}</p>
      <button type="button" class="nav-button" onclick={goToNextMonth} aria-label="Next month">
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
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
  </div>
{/if}

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

  .trigger:focus-visible,
  .trigger[aria-expanded="true"] {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .popover {
    z-index: 30;
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    width: 17rem;
    padding: var(--space-sm);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface-raised);
    box-shadow: var(--shadow-lg);
  }

  .quick-picks {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3xs);
  }

  .quick-pick {
    padding: var(--space-4xs) var(--space-2xs);
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
  }

  .nav-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.5rem;
    height: 1.5rem;
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
    font-size: var(--text-sm);
    font-weight: 700;
  }

  .weekday-row,
  .month-grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: var(--space-4xs);
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
    font-size: var(--text-xs);
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
