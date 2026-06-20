<script lang="ts">
  interface Props {
    open: boolean;
    title: string;
    message: string;
    /** Label for the "just this occurrence" action, e.g. "Just this task". */
    thisLabel?: string;
    /** Label for the "this and every future occurrence" action, e.g. "This and future". */
    futureLabel?: string;
    onThis: () => void;
    onFuture: () => void;
    onCancel: () => void;
  }

  let {
    open,
    title,
    message,
    thisLabel = "Just this task",
    futureLabel = "This and future",
    onThis,
    onFuture,
    onCancel,
  }: Props = $props();

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
  class="series-scope-dialog"
  aria-labelledby="series-scope-dialog-heading"
  onclose={onCancel}
  onclick={handleBackdropClick}
>
  <h2 id="series-scope-dialog-heading">{title}</h2>
  <p>{message}</p>
  <div class="actions">
    <button bind:this={cancelButtonEl} type="button" class="secondary" onclick={onCancel}>
      Cancel
    </button>
    <button type="button" class="secondary" onclick={onThis}>{thisLabel}</button>
    <button type="button" class="primary" onclick={onFuture}>{futureLabel}</button>
  </div>
</dialog>

<style>
  .series-scope-dialog {
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

  /* Higher specificity than `.series-scope-dialog` so a closed dialog stays
     `display: none` instead of the author-origin `display: flex` above
     overriding the UA stylesheet's `dialog:not([open]) { display: none }`. */
  .series-scope-dialog:not([open]) {
    display: none;
  }

  .series-scope-dialog::backdrop {
    background: oklch(20% 0.02 50 / 0.45);
  }

  .series-scope-dialog h2 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: 600;
    letter-spacing: var(--tracking-tight);
  }

  .series-scope-dialog p {
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
</style>
