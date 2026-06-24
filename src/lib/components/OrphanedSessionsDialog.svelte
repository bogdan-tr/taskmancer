<script lang="ts">
  import { resolveOrphanedSession } from "$lib/api";
  import { getErrorMessage } from "$lib/errors";
  import { refreshTasks, tasksState } from "$lib/tasks.svelte";
  import { refreshActiveSessions } from "$lib/tracking.svelte";
  import type { TimeEntry } from "$lib/types";

  interface Props {
    open: boolean;
    /** The orphaned subset of `trackingState.activeSessions`, computed once by the caller (`+layout.svelte`) via `isOrphaned` right after launch's `refreshActiveSessions`. */
    orphanedEntries: TimeEntry[];
    /** Called once every entry in this batch has been resolved (either action), so the caller can clear whatever opened this dialog. */
    onResolved: () => void;
  }

  let { open, orphanedEntries, onResolved }: Props = $props();

  let dialogEl: HTMLDialogElement | undefined = $state();

  /**
   * This dialog's own working copy of the batch — shrinks as each entry is
   * resolved, independent of `orphanedEntries` (which the caller never
   * mutates mid-batch). Reset from `orphanedEntries` only when the dialog
   * transitions from closed to open, mirroring `TaskEditDialog`'s draft-field
   * reset pattern.
   */
  let remainingEntries: TimeEntry[] = $state([]);
  /** The id of whichever entry currently has a resolve action in flight, so its own two buttons disable without blocking unrelated entries in the same batch. */
  let pendingEntryId: string | undefined = $state(undefined);
  let errorMessage = $state("");

  $effect(() => {
    if (!dialogEl) return;
    if (open) {
      if (!dialogEl.open) {
        remainingEntries = orphanedEntries;
        errorMessage = "";
        dialogEl.showModal();
      }
    } else if (dialogEl.open) {
      dialogEl.close();
    }
  });

  /**
   * Deliberately has no "Cancel"/dismiss action — every orphaned session
   * must be explicitly resumed or discarded. The native `<dialog>` element
   * still lets the Escape key fire its `close` event regardless, which would
   * desync `dialogEl.open` (now `false`) from the `open` prop (still `true`,
   * since `remainingEntries` hasn't emptied) without this handler. Calling
   * `showModal()` again immediately re-opens it, making "can't be dismissed
   * until resolved" hold for Escape too, not just backdrop clicks (this
   * dialog has no backdrop-click handler either, for the same reason).
   */
  function handleClose() {
    if (remainingEntries.length > 0) {
      dialogEl?.showModal();
    }
  }

  function taskTitleFor(taskId: string): string {
    return tasksState.items.find((t) => t.id === taskId)?.title ?? "Unknown task";
  }

  async function resolveEntry(entry: TimeEntry, action: "resume" | "discard") {
    pendingEntryId = entry.id;
    errorMessage = "";
    try {
      await resolveOrphanedSession(entry.id, action);
      await refreshActiveSessions();
      await refreshTasks();
      remainingEntries = remainingEntries.filter((e) => e.id !== entry.id);
      if (remainingEntries.length === 0) {
        onResolved();
      }
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to resolve session");
    } finally {
      pendingEntryId = undefined;
    }
  }
</script>

<dialog bind:this={dialogEl} class="orphaned-dialog" aria-labelledby="orphaned-dialog-heading" onclose={handleClose}>
  <h2 id="orphaned-dialog-heading">Resume or discard interrupted timers?</h2>
  <p class="intro">
    {remainingEntries.length} {remainingEntries.length === 1 ? "timer" : "timers"} were still
    showing as running when taskmancer last closed — likely from a force-quit or crash. Choose
    what to do with each one.
  </p>

  <ul class="entry-list">
    {#each remainingEntries as entry (entry.id)}
      <li class="entry-row">
        <div class="entry-info">
          <span class="entry-title">{taskTitleFor(entry.task_id)}</span>
          <span class="entry-meta">
            Started {new Date(entry.started_at).toLocaleString()} · still showing as running
          </span>
        </div>
        <div class="entry-actions">
          <button
            type="button"
            class="secondary"
            disabled={pendingEntryId === entry.id}
            onclick={() => resolveEntry(entry, "resume")}
          >
            Resume
          </button>
          <button
            type="button"
            class="danger"
            disabled={pendingEntryId === entry.id}
            onclick={() => resolveEntry(entry, "discard")}
          >
            Discard time since last check-in
          </button>
        </div>
      </li>
    {/each}
  </ul>

  {#if errorMessage}
    <p class="error" role="alert">{errorMessage}</p>
  {/if}
</dialog>

<style>
  .orphaned-dialog {
    padding: var(--space-lg);
    border: none;
    border-radius: var(--radius-lg);
    background: var(--color-surface-raised);
    color: var(--color-ink);
    box-shadow: var(--shadow-lg);
    width: min(32rem, calc(100vw - 2 * var(--space-lg)));
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  /* Higher specificity than `.orphaned-dialog` so a closed dialog stays
     `display: none` instead of the author-origin `display: flex` above
     overriding the UA stylesheet's `dialog:not([open]) { display: none }`. */
  .orphaned-dialog:not([open]) {
    display: none;
  }

  .orphaned-dialog::backdrop {
    background: oklch(20% 0.02 50 / 0.45);
  }

  .orphaned-dialog h2 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: 600;
    letter-spacing: var(--tracking-tight);
  }

  .intro {
    margin: 0;
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
  }

  .entry-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    margin: 0;
    padding: 0;
    list-style: none;
    max-height: 18rem;
    overflow-y: auto;
  }

  .entry-row {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
  }

  .entry-info {
    display: flex;
    flex-direction: column;
    gap: var(--space-3xs);
  }

  .entry-title {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-ink);
  }

  .entry-meta {
    font-size: var(--text-xs);
    color: var(--color-ink-muted);
  }

  .entry-actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-xs);
  }

  .entry-actions button {
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-md);
    border: none;
    font-weight: 600;
    font-size: var(--text-xs);
    cursor: pointer;
    box-shadow: var(--shadow-sm);
    transition:
      background var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo);
  }

  .entry-actions button.secondary {
    background: var(--color-surface);
    color: var(--color-ink);
    border: 1px solid var(--color-border);
    box-shadow: none;
  }

  .entry-actions button.secondary:hover:not(:disabled) {
    background: var(--color-canvas);
  }

  .entry-actions button.danger {
    background: var(--color-danger);
    color: var(--color-accent-ink);
  }

  .entry-actions button.danger:hover:not(:disabled) {
    background: var(--color-danger-hover);
  }

  .entry-actions button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .error {
    margin: 0;
    padding: var(--space-2xs) var(--space-xs);
    border-radius: var(--radius-sm);
    background: var(--color-danger-soft);
    color: var(--color-danger);
    font-size: var(--text-xs);
    font-weight: 600;
  }
</style>
