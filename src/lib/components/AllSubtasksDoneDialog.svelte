<script lang="ts">
  interface Props {
    open: boolean;
    /** The parent task whose subtasks just all finished, for the heading/message. `undefined` is treated the same as `open: false`. */
    task: { title: string } | undefined;
    onMarkDone: () => void;
    onDeleteSubtasks: () => void;
    onDismiss: () => void;
    /** Permanently stops this popup from ever firing again for this specific parent task. */
    onDismissPermanently: () => void;
  }

  let { open, task, onMarkDone, onDeleteSubtasks, onDismiss, onDismissPermanently }: Props = $props();

  let dialogEl: HTMLDialogElement | undefined = $state();
  let dismissButtonEl: HTMLButtonElement | undefined = $state();

  $effect(() => {
    if (!dialogEl) return;
    if (open && task) {
      if (!dialogEl.open) {
        dialogEl.showModal();
        dismissButtonEl?.focus();
      }
    } else if (dialogEl.open) {
      dialogEl.close();
    }
  });

  /** Closes the dialog when a click lands on the `::backdrop`, not its content box — mirrors `SeriesScopeDialog`/`ConfirmDialog`. */
  function handleBackdropClick(event: MouseEvent) {
    if (!dialogEl || event.target !== dialogEl) return;

    const rect = dialogEl.getBoundingClientRect();
    const insideContent =
      event.clientX >= rect.left &&
      event.clientX <= rect.right &&
      event.clientY >= rect.top &&
      event.clientY <= rect.bottom;

    if (!insideContent) {
      onDismiss();
    }
  }
</script>

<dialog
  bind:this={dialogEl}
  class="all-done-dialog"
  aria-labelledby="all-done-dialog-heading"
  onclose={onDismiss}
  onclick={handleBackdropClick}
>
  {#if task}
    <h2 id="all-done-dialog-heading">All subtasks done</h2>
    <p>Every subtask of "{task.title}" is now done. Mark the parent task done too, or clear its subtask list?</p>
    <div class="actions">
      <button bind:this={dismissButtonEl} type="button" class="secondary" onclick={onDismiss}>
        Not now
      </button>
      <button type="button" class="secondary" onclick={onDeleteSubtasks}>Delete subtask list</button>
      <button type="button" class="primary" onclick={onMarkDone}>Mark parent done</button>
    </div>
    <button type="button" class="dont-ask-again-link" onclick={onDismissPermanently}>
      Don't ask again for this task
    </button>
  {/if}
</dialog>

<style>
  .all-done-dialog {
    padding: var(--space-lg);
    border: none;
    border-radius: var(--radius-lg);
    background: var(--color-surface-raised);
    color: var(--color-ink);
    box-shadow: var(--shadow-lg);
    width: min(26rem, calc(100vw - 2 * var(--space-lg)));
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  /* Higher specificity than `.all-done-dialog` so a closed dialog stays
     `display: none` instead of the author-origin `display: flex` above
     overriding the UA stylesheet's `dialog:not([open]) { display: none }`. */
  .all-done-dialog:not([open]) {
    display: none;
  }

  .all-done-dialog::backdrop {
    background: oklch(20% 0.02 50 / 0.45);
  }

  .all-done-dialog h2 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: 600;
    letter-spacing: var(--tracking-tight);
  }

  .all-done-dialog p {
    margin: 0;
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: var(--space-xs);
  }

  .actions button {
    padding: var(--space-sm) var(--space-lg);
    border-radius: var(--radius-md);
    border: none;
    font-weight: 600;
    font-size: var(--text-base);
    cursor: pointer;
    box-shadow: var(--shadow-sm);
    transition:
      background var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo),
      transform var(--duration-fast) var(--ease-out-expo);
  }

  .actions button.secondary {
    background: var(--color-surface);
    color: var(--color-ink);
    border: 1px solid var(--color-border);
    box-shadow: none;
  }

  .actions button.secondary:hover {
    background: var(--color-canvas);
  }

  .actions button.primary {
    background: var(--color-accent);
    color: var(--color-accent-ink);
  }

  .actions button.primary:hover {
    background: var(--color-accent-hover);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .actions button:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .dont-ask-again-link {
    align-self: flex-end;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--color-ink-faint);
    font-size: var(--text-xs);
    font-weight: 600;
    text-decoration: underline;
    cursor: pointer;
  }

  .dont-ask-again-link:hover {
    color: var(--color-ink-muted);
  }
</style>
