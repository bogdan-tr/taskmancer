<script lang="ts">
  import { sortedStatuses, statusColor } from "$lib/statuses.svelte";
  import type { StatusDefinition } from "$lib/types";

  interface Props {
    open: boolean;
    /** The title shown in the dialog heading, e.g. the subtask's own title. */
    taskTitle: string;
    statuses: StatusDefinition[];
    currentStatusId: string;
    onSelect: (statusId: string) => void;
    onCancel: () => void;
  }

  let { open, taskTitle, statuses, currentStatusId, onSelect, onCancel }: Props = $props();

  let dialogEl: HTMLDialogElement | undefined = $state();

  /**
   * A plain native `<dialog>`, centered by the browser's own UA styling —
   * deliberately not a positioned/anchored popover like `WeekBarItem`'s bar
   * popup, which needs its own left/right-alignment math to stay on
   * screen near whichever bar opened it. A glued subtask row's status dot
   * has no such constraint to solve, so this avoids that whole class of
   * positioning bug entirely rather than reimplementing it unnecessarily.
   */
  $effect(() => {
    if (!dialogEl) return;
    if (open) {
      if (!dialogEl.open) dialogEl.showModal();
    } else if (dialogEl.open) {
      dialogEl.close();
    }
  });

  /** Closes the dialog when a click lands on the `::backdrop`, not its content box. */
  function handleBackdropClick(event: MouseEvent) {
    if (!dialogEl || event.target !== dialogEl) return;

    const rect = dialogEl.getBoundingClientRect();
    const insideContent =
      event.clientX >= rect.left &&
      event.clientX <= rect.right &&
      event.clientY >= rect.top &&
      event.clientY <= rect.bottom;

    if (!insideContent) {
      onCancel();
    }
  }
</script>

<dialog
  bind:this={dialogEl}
  class="status-picker-dialog"
  aria-labelledby="status-picker-heading"
  onclose={onCancel}
  onclick={handleBackdropClick}
>
  <h2 id="status-picker-heading">{taskTitle}</h2>
  <ul class="status-options">
    {#each sortedStatuses(statuses) as status (status.id)}
      <li>
        <button
          type="button"
          class="status-option"
          class:active={status.id === currentStatusId}
          onclick={() => onSelect(status.id)}
        >
          <span class="status-dot" style="background: {statusColor(statuses, status.id)}" aria-hidden="true"
          ></span>
          {status.label}
        </button>
      </li>
    {/each}
  </ul>
</dialog>

<style>
  .status-picker-dialog {
    padding: var(--space-lg);
    border: none;
    border-radius: var(--radius-lg);
    background: var(--color-surface-raised);
    color: var(--color-ink);
    box-shadow: var(--shadow-lg);
    width: min(20rem, calc(100vw - 2 * var(--space-lg)));
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  /* Higher specificity than `.status-picker-dialog` so a closed dialog stays
     `display: none` instead of the author-origin `display: flex` above
     overriding the UA stylesheet's `dialog:not([open]) { display: none }`. */
  .status-picker-dialog:not([open]) {
    display: none;
  }

  .status-picker-dialog::backdrop {
    background: oklch(20% 0.02 50 / 0.45);
  }

  .status-picker-dialog h2 {
    margin: 0;
    font-size: var(--text-base);
    font-weight: 600;
    letter-spacing: var(--tracking-tight);
    word-break: break-word;
  }

  .status-options {
    display: flex;
    flex-direction: column;
    gap: var(--space-3xs);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .status-option {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    width: 100%;
    padding: var(--space-xs) var(--space-sm);
    border: none;
    border-radius: var(--radius-md);
    background: var(--color-surface);
    color: var(--color-ink);
    font: inherit;
    font-size: var(--text-sm);
    font-weight: 600;
    text-align: left;
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out-expo);
  }

  .status-option:hover {
    background: var(--color-canvas);
  }

  .status-option:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .status-option.active {
    background: var(--color-accent-soft);
    color: var(--color-accent);
  }

  .status-dot {
    width: 0.6rem;
    height: 0.6rem;
    flex-shrink: 0;
    border-radius: var(--radius-pill);
  }
</style>
