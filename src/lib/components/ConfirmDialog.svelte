<script lang="ts">
  interface Props {
    open: boolean;
    title: string;
    message: string;
    confirmLabel?: string;
    onConfirm: () => void;
    onCancel: () => void;
  }

  let { open, title, message, confirmLabel = "Delete", onConfirm, onCancel }: Props = $props();

  let dialogEl: HTMLDialogElement | undefined = $state();
  let cancelButtonEl: HTMLButtonElement | undefined = $state();

  $effect(() => {
    if (!dialogEl) return;
    if (open) {
      if (!dialogEl.open) {
        dialogEl.showModal();
        cancelButtonEl?.focus();
      }
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
  class="confirm-dialog"
  aria-labelledby="confirm-dialog-heading"
  onclose={onCancel}
  onclick={handleBackdropClick}
>
  <h2 id="confirm-dialog-heading">{title}</h2>
  <p>{message}</p>
  <div class="actions">
    <button bind:this={cancelButtonEl} type="button" class="secondary" onclick={onCancel}>
      Cancel
    </button>
    <button type="button" class="danger" onclick={onConfirm}>{confirmLabel}</button>
  </div>
</dialog>

<style>
  .confirm-dialog {
    padding: 0;
    border: none;
    border-radius: var(--radius-lg);
    background: var(--color-surface-raised);
    color: var(--color-ink);
    box-shadow: var(--shadow-lg);
    width: min(24rem, calc(100vw - 2 * var(--space-lg)));
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    padding: var(--space-lg);
  }

  /* Higher specificity than `.confirm-dialog` so a closed dialog stays
     `display: none` instead of the author-origin `display: flex` above
     overriding the UA stylesheet's `dialog:not([open]) { display: none }`. */
  .confirm-dialog:not([open]) {
    display: none;
  }

  .confirm-dialog::backdrop {
    background: oklch(20% 0.02 50 / 0.45);
  }

  .confirm-dialog h2 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: 600;
    letter-spacing: var(--tracking-tight);
  }

  .confirm-dialog p {
    margin: 0;
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
  }

  .actions {
    display: flex;
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

  .actions button.danger {
    background: var(--color-danger);
    color: var(--color-accent-ink);
  }

  .actions button.danger:hover {
    background: var(--color-danger-hover);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }
</style>
