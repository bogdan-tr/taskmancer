<script lang="ts">
  import type { RecurrenceFrequency } from "$lib/recurrence";
  import {
    clampDayOfMonth,
    clampNonNegativeInteger,
    clampPositiveInteger,
    WEEKDAY_LABELS,
    type DueRule,
    type RecurrenceBuilderValue,
  } from "$lib/recurrence";
  import DatePickerPopover from "./DatePickerPopover.svelte";

  interface Props {
    /** The current value to pre-fill when the dialog opens — from a manual override if set, else the parsed NL recurrence/due tokens. `undefined` shows sensible defaults (daily, no end date, due rule "use default"). */
    value?: RecurrenceBuilderValue;
    triggerLabel: string;
    onApply: (value: RecurrenceBuilderValue) => void;
    /** Reverts to whatever's typed in the title, discarding any manual override this dialog previously set. */
    onClear: () => void;
  }

  let { value, triggerLabel, onApply, onClear }: Props = $props();

  let open = $state(false);
  let dialogEl: HTMLDialogElement | undefined = $state();

  type FrequencyKind = RecurrenceFrequency["kind"];
  type DueRuleKind = "default" | "never" | "afterScheduled" | "weekday";

  const WEEKDAYS_SHORTCUT = [1, 2, 3, 4, 5];
  const WEEKEND_SHORTCUT = [0, 6];

  let draftKind: FrequencyKind = $state("EveryNDays");
  let draftEveryNDaysInterval = $state(1);
  let draftWeeklyWeekdays: number[] = $state([1]);
  let draftWeeklyIntervalWeeks = $state(1);
  let draftMonthlyDay = $state(1);
  let draftHasEndDate = $state(false);
  let draftEndDate: string | undefined = $state(undefined);
  let draftDueRuleKind: DueRuleKind = $state("default");
  let draftDueRuleDays = $state(0);
  let draftDueRuleWeekday = $state(1);
  let draftDueRuleIntervalWeeks = $state(1);

  /** Resets every draft field from `value` (or sensible defaults) — called each time the dialog opens, mirroring `DatePickerPopover`'s reset-on-open for its visible month. */
  function initializeDraft() {
    const frequency = value?.frequency;
    if (frequency?.kind === "Weekly") {
      draftKind = "Weekly";
      draftWeeklyWeekdays = [...frequency.weekdays];
      draftWeeklyIntervalWeeks = frequency.interval_weeks;
    } else if (frequency?.kind === "MonthlyByDay") {
      draftKind = "MonthlyByDay";
      draftMonthlyDay = frequency.day;
    } else {
      draftKind = "EveryNDays";
      draftEveryNDaysInterval = frequency?.kind === "EveryNDays" ? frequency.interval : 1;
      draftWeeklyWeekdays = [1];
      draftWeeklyIntervalWeeks = 1;
      draftMonthlyDay = 1;
    }

    draftHasEndDate = value?.endDate !== undefined;
    draftEndDate = value?.endDate;

    const rule = value?.dueRule;
    if (rule?.kind === "Never") {
      draftDueRuleKind = "never";
    } else if (rule?.kind === "AfterScheduled") {
      draftDueRuleKind = "afterScheduled";
      draftDueRuleDays = clampNonNegativeInteger(rule.days);
    } else if (rule?.kind === "Weekday") {
      draftDueRuleKind = "weekday";
      draftDueRuleWeekday = rule.weekday;
      draftDueRuleIntervalWeeks = rule.interval_weeks;
    } else {
      draftDueRuleKind = "default";
      draftDueRuleDays = 0;
      draftDueRuleWeekday = 1;
      draftDueRuleIntervalWeeks = 1;
    }
  }

  function openDialog() {
    initializeDraft();
    open = true;
  }

  function toggleWeekday(day: number) {
    draftWeeklyWeekdays = draftWeeklyWeekdays.includes(day)
      ? draftWeeklyWeekdays.filter((d) => d !== day)
      : [...draftWeeklyWeekdays, day].sort((a, b) => a - b);
  }

  function applyWeekdayShortcut(days: number[]) {
    draftWeeklyWeekdays = [...days];
  }

  let weeklyMatchesShortcut = $derived((days: number[]) => {
    const sorted = [...draftWeeklyWeekdays].sort((a, b) => a - b);
    return sorted.length === days.length && sorted.every((day, index) => day === days[index]);
  });

  function buildFrequency(): RecurrenceFrequency {
    switch (draftKind) {
      case "EveryNDays":
        return { kind: "EveryNDays", interval: clampPositiveInteger(draftEveryNDaysInterval) };
      case "Weekly":
        return {
          kind: "Weekly",
          weekdays: draftWeeklyWeekdays.length > 0 ? draftWeeklyWeekdays : [1],
          interval_weeks: clampPositiveInteger(draftWeeklyIntervalWeeks),
        };
      case "MonthlyByDay":
        return { kind: "MonthlyByDay", day: clampDayOfMonth(draftMonthlyDay) };
    }
  }

  function buildDueRule(): DueRule | undefined {
    switch (draftDueRuleKind) {
      case "default":
        return undefined;
      case "never":
        return { kind: "Never" };
      case "afterScheduled":
        return { kind: "AfterScheduled", days: clampNonNegativeInteger(draftDueRuleDays) };
      case "weekday":
        return {
          kind: "Weekday",
          weekday: draftDueRuleWeekday,
          interval_weeks: clampPositiveInteger(draftDueRuleIntervalWeeks),
        };
    }
  }

  function handleApply() {
    onApply({
      frequency: buildFrequency(),
      endDate: draftHasEndDate ? draftEndDate : undefined,
      dueRule: buildDueRule(),
    });
    open = false;
  }

  function handleClear() {
    onClear();
    open = false;
  }

  /** Same top-layer-stacking rationale as `DatePickerPopover` — see its own doc comment. */
  $effect(() => {
    if (!dialogEl) return;
    if (open) {
      if (!dialogEl.open) dialogEl.showModal();
    } else if (dialogEl.open) {
      dialogEl.close();
    }
  });

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
</script>

<button type="button" class="trigger" onclick={openDialog} aria-haspopup="dialog" title={triggerLabel}>
  <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d="M2 4.5a5.5 5.5 0 0 1 9.5-3.5M14 2v3h-3" />
    <path d="M14 11.5a5.5 5.5 0 0 1-9.5 3.5M2 14v-3h3" />
  </svg>
  <span class="trigger-label">{triggerLabel}</span>
</button>

<dialog
  bind:this={dialogEl}
  class="builder-dialog"
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

  <section class="builder-section">
    <h3 class="section-title">Repeats</h3>
    <div class="frequency-tabs">
      <button type="button" class="frequency-tab" aria-pressed={draftKind === "EveryNDays"} class:is-active={draftKind === "EveryNDays"} onclick={() => (draftKind = "EveryNDays")}>Daily</button>
      <button type="button" class="frequency-tab" aria-pressed={draftKind === "Weekly"} class:is-active={draftKind === "Weekly"} onclick={() => (draftKind = "Weekly")}>Weekly</button>
      <button type="button" class="frequency-tab" aria-pressed={draftKind === "MonthlyByDay"} class:is-active={draftKind === "MonthlyByDay"} onclick={() => (draftKind = "MonthlyByDay")}>Monthly</button>
    </div>

    {#if draftKind === "EveryNDays"}
      <label class="control-row">
        Every
        <input type="number" min="1" step="1" bind:value={draftEveryNDaysInterval} />
        day(s)
      </label>
    {:else if draftKind === "Weekly"}
      <div class="weekday-shortcuts">
        <button type="button" class="shortcut" class:is-active={weeklyMatchesShortcut(WEEKDAYS_SHORTCUT)} onclick={() => applyWeekdayShortcut(WEEKDAYS_SHORTCUT)}>
          Weekdays
        </button>
        <button type="button" class="shortcut" class:is-active={weeklyMatchesShortcut(WEEKEND_SHORTCUT)} onclick={() => applyWeekdayShortcut(WEEKEND_SHORTCUT)}>
          Weekends
        </button>
      </div>
      <div class="weekday-checkboxes">
        {#each WEEKDAY_LABELS as label, day (label)}
          <label class="weekday-checkbox" class:is-checked={draftWeeklyWeekdays.includes(day)}>
            <input type="checkbox" checked={draftWeeklyWeekdays.includes(day)} onchange={() => toggleWeekday(day)} />
            {label}
          </label>
        {/each}
      </div>
      <label class="control-row">
        Every
        <input type="number" min="1" step="1" bind:value={draftWeeklyIntervalWeeks} />
        week(s)
      </label>
    {:else if draftKind === "MonthlyByDay"}
      <label class="control-row">
        On day
        <input type="number" min="1" max="31" step="1" bind:value={draftMonthlyDay} />
        of the month
      </label>
    {/if}
  </section>

  <section class="builder-section">
    <h3 class="section-title">Ends</h3>
    <label class="control-row">
      <input type="checkbox" bind:checked={draftHasEndDate} />
      On a specific date
    </label>
    {#if draftHasEndDate}
      <div class="control-row">
        <span class="end-date-value">{draftEndDate ?? "Pick a date"}</span>
        <DatePickerPopover
          selected={draftEndDate}
          triggerLabel="Pick end date"
          clearLabel="Clear"
          onSelect={(iso) => (draftEndDate = iso)}
          onClear={() => (draftEndDate = undefined)}
        />
      </div>
    {/if}
  </section>

  <section class="builder-section">
    <fieldset class="due-rule-fieldset">
      <legend class="section-title">Due</legend>
      <div class="due-rule-options">
        <label class="due-rule-option">
          <input type="radio" name="due-rule-kind" value="default" checked={draftDueRuleKind === "default"} onchange={() => (draftDueRuleKind = "default")} />
          Use the default
        </label>
        <label class="due-rule-option">
          <input type="radio" name="due-rule-kind" value="never" checked={draftDueRuleKind === "never"} onchange={() => (draftDueRuleKind = "never")} />
          Never due
        </label>
        <label class="due-rule-option">
          <input type="radio" name="due-rule-kind" value="afterScheduled" checked={draftDueRuleKind === "afterScheduled"} onchange={() => (draftDueRuleKind = "afterScheduled")} />
          <span class="control-row">
            <input type="number" min="0" step="1" disabled={draftDueRuleKind !== "afterScheduled"} bind:value={draftDueRuleDays} />
            day(s) after each occurrence
          </span>
        </label>
        <label class="due-rule-option">
          <input type="radio" name="due-rule-kind" value="weekday" checked={draftDueRuleKind === "weekday"} onchange={() => (draftDueRuleKind = "weekday")} />
          <span class="control-row">
            On
            <select disabled={draftDueRuleKind !== "weekday"} bind:value={draftDueRuleWeekday}>
              {#each WEEKDAY_LABELS as label, day (label)}
                <option value={day}>{label}</option>
              {/each}
            </select>
            every
            <input type="number" min="1" step="1" disabled={draftDueRuleKind !== "weekday"} bind:value={draftDueRuleIntervalWeeks} />
            week(s)
          </span>
        </label>
      </div>
    </fieldset>
  </section>

  <div class="dialog-actions">
    <button type="button" class="action action-clear" onclick={handleClear}>Clear</button>
    <div class="action-group">
      <button type="button" class="action action-cancel" onclick={() => dialogEl?.close()}>Cancel</button>
      <button type="button" class="action action-apply" onclick={handleApply}>Apply</button>
    </div>
  </div>
</dialog>

<style>
  .trigger {
    display: flex;
    align-items: center;
    gap: var(--space-3xs);
    padding: var(--space-3xs) var(--space-xs);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink-muted);
    font-size: var(--text-xs);
    font-weight: 600;
    cursor: pointer;
    transition:
      color var(--duration-fast) var(--ease-out-expo),
      border-color var(--duration-fast) var(--ease-out-expo);
  }

  .trigger:hover {
    color: var(--color-ink);
    border-color: var(--color-accent);
  }

  .trigger:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .trigger svg {
    flex-shrink: 0;
  }

  .builder-dialog {
    padding: var(--space-lg);
    border: none;
    border-radius: var(--radius-lg);
    background: var(--color-surface-raised);
    color: var(--color-ink);
    box-shadow: var(--shadow-lg);
    width: min(26rem, calc(100vw - 2 * var(--space-lg)));
    max-height: calc(100vh - 4 * var(--space-lg));
    overflow-y: auto;
  }

  .builder-dialog:not([open]) {
    display: none;
  }

  .builder-dialog::backdrop {
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

  .builder-section {
    padding: var(--space-sm) 0;
    border-top: 1px solid var(--color-border);
  }

  .builder-section:first-of-type {
    border-top: none;
    padding-top: 0;
  }

  .section-title {
    margin: 0 0 var(--space-xs);
    padding: 0;
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
  }

  .frequency-tabs {
    display: flex;
    gap: var(--space-3xs);
    margin-bottom: var(--space-sm);
  }

  .frequency-tab {
    flex: 1;
    padding: var(--space-2xs) var(--space-xs);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-canvas);
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
    font-weight: 600;
    cursor: pointer;
    transition:
      color var(--duration-fast) var(--ease-out-expo),
      background var(--duration-fast) var(--ease-out-expo),
      border-color var(--duration-fast) var(--ease-out-expo);
  }

  .frequency-tab:hover {
    color: var(--color-ink);
  }

  .frequency-tab:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .frequency-tab.is-active {
    background: var(--color-accent);
    color: var(--color-accent-ink);
    border-color: var(--color-accent);
  }

  .control-row {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    font-size: var(--text-sm);
  }

  .control-row input[type="number"] {
    width: 3.5rem;
    padding: var(--space-3xs) var(--space-2xs);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
    font: inherit;
    font-size: var(--text-sm);
    text-align: right;
  }

  .control-row select {
    padding: var(--space-3xs) var(--space-2xs);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
    font: inherit;
    font-size: var(--text-sm);
  }

  .weekday-shortcuts {
    display: flex;
    gap: var(--space-3xs);
    margin-bottom: var(--space-xs);
  }

  .shortcut {
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

  .shortcut:hover {
    background: var(--color-accent-soft);
    color: var(--color-accent);
  }

  .shortcut.is-active {
    background: var(--color-accent);
    color: var(--color-accent-ink);
    border-color: var(--color-accent);
  }

  .weekday-checkboxes {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3xs);
    margin-bottom: var(--space-xs);
  }

  .weekday-checkbox {
    display: flex;
    align-items: center;
    gap: var(--space-3xs);
    padding: var(--space-3xs) var(--space-xs);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-canvas);
    font-size: var(--text-xs);
    font-weight: 600;
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out-expo);
  }

  .weekday-checkbox.is-checked {
    background: var(--color-accent-soft);
    border-color: var(--color-accent);
    color: var(--color-accent);
  }

  .weekday-checkbox input {
    margin: 0;
  }

  .end-date-value {
    font-size: var(--text-sm);
    color: var(--color-ink-faint);
  }

  .due-rule-fieldset {
    margin: 0;
    padding: 0;
    border: none;
  }

  .due-rule-options {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .due-rule-option {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    font-size: var(--text-sm);
  }

  .due-rule-option input[type="radio"] {
    flex-shrink: 0;
  }

  .dialog-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-sm);
    margin-top: var(--space-md);
    padding-top: var(--space-sm);
    border-top: 1px solid var(--color-border);
  }

  .action-group {
    display: flex;
    gap: var(--space-2xs);
  }

  .action {
    padding: var(--space-2xs) var(--space-md);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-canvas);
    color: var(--color-ink);
    font-size: var(--text-sm);
    font-weight: 600;
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out-expo);
  }

  .action:hover {
    background: var(--color-accent-soft);
  }

  .action:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .action-clear {
    color: var(--color-ink-muted);
    background: transparent;
    border-color: transparent;
  }

  .action-clear:hover {
    background: var(--color-canvas);
  }

  .action-apply {
    background: var(--color-accent);
    color: var(--color-accent-ink);
    border-color: var(--color-accent);
  }

  .action-apply:hover {
    filter: brightness(1.05);
  }
</style>
