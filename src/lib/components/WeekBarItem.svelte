<script lang="ts">
  import { WEEK_BAR_CHROMA_BOOST, WEEK_BAR_LIGHTNESS, neonCardColor } from "$lib/colorPresets";
  import { priorityColor, priorityLabel } from "$lib/priorities.svelte";
  import { resolveProjectColor } from "$lib/projectColor";
  import { projectsState } from "$lib/projects.svelte";
  import { statusColor, statusLabel } from "$lib/statuses.svelte";
  import type { PriorityLevel, StatusDefinition, Task } from "$lib/types";
  import type { WeekBar } from "$lib/weekGrouping";

  interface Props {
    weekBar: WeekBar;
    colorCoded: boolean;
    rightAlignPopover: boolean;
    priorities: PriorityLevel[];
    statuses: StatusDefinition[];
    doneStatus: string;
    cancelledStatus: string | undefined;
    isOpen: boolean;
    onToggle: () => void;
    onClosePopover: () => void;
    onEdit: (task: Task) => void;
  }

  let {
    weekBar,
    colorCoded,
    rightAlignPopover,
    priorities,
    statuses,
    doneStatus,
    cancelledStatus,
    isOpen,
    onToggle,
    onClosePopover,
    onEdit,
  }: Props = $props();

  let task = $derived(weekBar.task);
  let barColor = $derived(resolveProjectColor(task.project, projectsState.items));
  let barPriorityColor = $derived(priorityColor(priorities, task.priority));
  let barStatusColor = $derived(statusColor(statuses, task.status));
  let barColorCodeBg = $derived(
    colorCoded ? neonCardColor(barColor, WEEK_BAR_LIGHTNESS, WEEK_BAR_CHROMA_BOOST) : undefined,
  );
  let done = $derived(task.status === doneStatus);
  let cancelled = $derived(!done && cancelledStatus !== undefined && task.status === cancelledStatus);

  /**
   * `.bar-summary` must not be a real `<button>`: `svelte-dnd-action` skips
   * drag-initiation for any mousedown target with `.value !== undefined`
   * (meant to exempt `<input>`/`<select>` elements), but `<button>` also has
   * a `.value` DOM property (default `""`, not `undefined`) — so a button
   * here would make the whole bar undraggable. Same fix as TaskCard's title
   * (see its `handleTitleKeydown`): a plain `<div role="button">` plus this
   * handler for Enter/Space keeps it keyboard-accessible without the
   * drag-exemption side effect.
   */
  function handleSummaryKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      onToggle();
    }
  }
</script>

<div
  class="bar"
  class:color-coded={colorCoded}
  class:bar-done={done}
  class:bar-cancelled={cancelled}
  style="--bar-color: {barColor}; --bar-priority-color: {barPriorityColor}; --bar-status-color: {barStatusColor}; --bar-color-code-bg: {barColorCodeBg}"
>
  <div
    class="bar-summary"
    role="button"
    tabindex="0"
    aria-label="{weekBar.type === 'scheduled' ? 'Scheduled' : 'Due'} – {task.title}"
    aria-expanded={isOpen}
    onclick={onToggle}
    onkeydown={handleSummaryKeydown}
  >
    <span class="bar-icon" aria-hidden="true">
      {#if weekBar.type === "scheduled"}
        <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
          <circle cx="8" cy="8" r="6" fill="currentColor" />
        </svg>
      {:else}
        <svg
          viewBox="0 0 16 16"
          width="15"
          height="15"
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
    <span class="bar-title">{task.title}</span>
    <span class="bar-status-box" aria-hidden="true"></span>
    {#if cancelled}
      <span class="bar-cancelled-x" aria-hidden="true">
        <svg viewBox="0 0 16 16" width="22" height="22">
          <path d="M3 3l10 10M13 3L3 13" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" />
        </svg>
      </span>
    {/if}
  </div>
  {#if isOpen}
    <div class="bar-popover" class:popover-right={rightAlignPopover}>
      <div class="popover-header">
        <p class="popover-title">{task.title}</p>
        <button type="button" class="popover-close" onclick={onClosePopover} aria-label="Close">
          <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
            <path d="M3 3l10 10M13 3L3 13" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" />
          </svg>
        </button>
      </div>
      <div class="popover-meta">
        {#if task.project}
          <span class="chip project" style="--chip-color: {barColor}">{task.project}</span>
        {/if}
        <span class="chip priority" style="--chip-color: {barPriorityColor}">
          <span class="priority-dot" aria-hidden="true"></span>
          {priorityLabel(priorities, task.priority)}
        </span>
        <span class="chip status" class:status-done={done} class:status-cancelled={cancelled}>
          {statusLabel(statuses, task.status)}
        </span>
      </div>
      {#if task.scheduled || task.due}
        <dl class="popover-dates">
          {#if task.scheduled}
            <div class="date-row">
              <dt>Scheduled</dt>
              <dd>{task.scheduled}</dd>
            </div>
          {/if}
          {#if task.due}
            <div class="date-row">
              <dt>Due</dt>
              <dd>{task.due}</dd>
            </div>
          {/if}
        </dl>
      {/if}
      <button type="button" class="edit-button" onclick={() => onEdit(task)}> Edit </button>
    </div>
  {/if}
</div>

<style>
  .bar {
    position: relative;
  }

  .bar-summary {
    display: flex;
    align-items: flex-start;
    width: 100%;
    gap: var(--space-3xs);
    padding: var(--space-3xs) var(--space-2xs);
    border: none;
    border-radius: var(--radius-sm);
    border-left: 3px solid var(--bar-priority-color, var(--color-border-strong));
    background: color-mix(in oklch, var(--bar-color, var(--color-border-strong)) 14%, var(--color-surface));
    font: inherit;
    font-size: var(--text-xs);
    font-weight: 600;
    line-height: var(--leading-tight);
    color: var(--color-ink);
    text-align: left;
    cursor: pointer;
    transition:
      box-shadow var(--duration-fast) var(--ease-out-expo),
      transform var(--duration-fast) var(--ease-out-expo);
  }

  .bar-summary:hover {
    box-shadow: var(--shadow-sm);
    transform: translateY(-1px);
  }

  .bar-summary:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .bar-summary[aria-expanded="true"] {
    box-shadow: 0 0 0 2px var(--bar-color, var(--color-accent));
  }

  /* "Color code" mode: a vivid, fixed-lightness rendering of the project's
     hue/chroma (see `neonCardColor`, shared with TaskCard's card mode but
     using Week-view-specific, deliberately darker constants). Unlike
     TaskCard's lighter target (safe for dark text), this lightness is dark
     enough that text/icons switch to a light color instead — a per-bar
     contrast check isn't needed since the target lightness is fixed. */
  .bar.color-coded .bar-summary {
    background: var(--bar-color-code-bg);
    color: oklch(96% 0.01 0);
  }

  .bar.color-coded .bar-icon {
    color: oklch(96% 0.01 0);
  }

  .bar-icon {
    display: flex;
    align-items: center;
    color: var(--bar-priority-color, var(--color-ink-muted));
    flex-shrink: 0;
    margin-top: 1px;
  }

  .bar-title {
    flex: 1 1 auto;
    min-width: 0;
    overflow-wrap: break-word;
  }

  /* Done: tint toward the configured done status's color and strike through
     the title — a glance should make "this is finished" unmistakable.
     Cancelled: tint toward the cancelled status's color and overlay an X
     (see .bar-cancelled-x) instead of a strikethrough, so the two finished
     states read as visually distinct from each other, not just both
     "muted." */
  .bar.bar-done .bar-summary {
    background: color-mix(in oklch, var(--bar-status-color) 16%, var(--color-surface));
    opacity: 0.75;
  }

  .bar.bar-done .bar-title {
    text-decoration: line-through;
    text-decoration-thickness: 1.5px;
  }

  .bar.bar-cancelled .bar-summary {
    background: color-mix(in oklch, var(--bar-status-color) 16%, var(--color-surface));
    opacity: 0.75;
  }

  .bar-cancelled-x {
    display: flex;
    align-items: center;
    flex-shrink: 0;
    color: var(--bar-status-color);
  }

  .bar-status-box {
    width: 0.65rem;
    height: 0.65rem;
    flex-shrink: 0;
    margin-top: 2px;
    border-radius: var(--radius-sm);
    background: var(--bar-status-color, var(--color-border-strong));
  }

  .bar-popover {
    position: absolute;
    top: calc(100% + var(--space-3xs));
    left: 0;
    z-index: 20;
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    width: 15rem;
    padding: var(--space-sm) var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface-raised);
    box-shadow: var(--shadow-lg);
  }

  .bar-popover.popover-right {
    left: auto;
    right: 0;
  }

  .popover-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-sm);
  }

  .popover-title {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: 700;
    line-height: var(--leading-tight);
    word-break: break-word;
  }

  .popover-close {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 1.5rem;
    height: 1.5rem;
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--color-ink-muted);
    cursor: pointer;
    transition:
      background var(--duration-fast) var(--ease-out-expo),
      color var(--duration-fast) var(--ease-out-expo);
  }

  .popover-close:hover {
    background: var(--color-canvas);
    color: var(--color-ink);
  }

  .popover-close:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
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
    border-radius: var(--radius-sm);
    background: var(--color-canvas);
    border: 1px solid var(--color-border);
    color: var(--color-ink-muted);
  }

  .chip.project {
    background: color-mix(in oklch, var(--chip-color, var(--color-accent)) 16%, var(--color-surface-raised));
    border-color: color-mix(in oklch, var(--chip-color, var(--color-accent)) 45%, transparent);
    color: var(--chip-color, var(--color-accent));
    font-weight: 600;
  }

  .chip.priority {
    font-weight: 600;
  }

  .chip.status {
    font-weight: 600;
  }

  .chip.status.status-done {
    text-decoration: line-through;
  }

  .chip.status.status-cancelled {
    color: var(--color-danger);
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
