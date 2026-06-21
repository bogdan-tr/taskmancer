<script lang="ts">
  import type { DeleteStrategyKind } from "$lib/deleteProject";
  import type { Project } from "$lib/types";

  interface Props {
    open: boolean;
    projectName: string;
    taskCount: number;
    /** How many descendant subprojects will also be deleted (0 if none). */
    descendantCount: number;
    otherProjects: Project[];
    onConfirm: (strategy: DeleteStrategyKind, targetProjectId: string) => void;
    onCancel: () => void;
  }

  let { open, projectName, taskCount, descendantCount, otherProjects, onConfirm, onCancel }: Props = $props();

  let canReassign = $derived(otherProjects.length > 0);
  let strategyKind = $state<DeleteStrategyKind>("reassign");
  let targetProjectId = $state("");

  let dialogEl: HTMLDialogElement | undefined = $state();
  let cancelButtonEl: HTMLButtonElement | undefined = $state();

  $effect(() => {
    if (!dialogEl) return;
    if (open) {
      if (!dialogEl.open) {
        strategyKind = canReassign ? "reassign" : "archive";
        targetProjectId = otherProjects[0]?.id ?? "";
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

  let confirmDisabled = $derived(strategyKind === "reassign" && !targetProjectId);

  function handleConfirm() {
    if (confirmDisabled) return;
    onConfirm(strategyKind, targetProjectId);
  }
</script>

<dialog
  bind:this={dialogEl}
  class="confirm-dialog"
  aria-labelledby="delete-project-dialog-heading"
  onclose={onCancel}
  onclick={handleBackdropClick}
>
  <h2 id="delete-project-dialog-heading">
    Delete "{projectName}"{descendantCount > 0 ? ` and its ${descendantCount} ${descendantCount === 1 ? "subproject" : "subprojects"}` : ""}?
  </h2>
  <p>
    {taskCount} {taskCount === 1 ? "task" : "tasks"}
    {descendantCount > 0 ? "across this project and its subprojects" : ""} still
    {taskCount === 1 ? "belongs" : "belong"} {descendantCount > 0 ? "here" : "to this project"}. Choose what to
    do with {taskCount === 1 ? "it" : "them"} before deleting.
  </p>

  <fieldset>
    <legend>What should happen to these tasks?</legend>

    <label class="option">
      <input
        type="radio"
        name="task-strategy"
        value="reassign"
        bind:group={strategyKind}
        disabled={!canReassign}
      />
      <span>
        <strong>Reassign</strong> — move them to another project
      </span>
    </label>

    {#if strategyKind === "reassign"}
      <div class="target-picker">
        <label for="delete-project-target">Move to</label>
        <select id="delete-project-target" bind:value={targetProjectId}>
          {#each otherProjects as candidate (candidate.id)}
            <option value={candidate.id}>{candidate.name}</option>
          {/each}
        </select>
      </div>
    {/if}

    <label class="option">
      <input type="radio" name="task-strategy" value="archive" bind:group={strategyKind} />
      <span>
        <strong>Archive</strong> — move them out of the board for now
      </span>
    </label>

    <label class="option">
      <input type="radio" name="task-strategy" value="delete" bind:group={strategyKind} />
      <span>
        <strong>Delete permanently</strong> — remove them for good
      </span>
    </label>
  </fieldset>

  <div class="actions">
    <button bind:this={cancelButtonEl} type="button" class="secondary" onclick={onCancel}>
      Cancel
    </button>
    <button type="button" class="danger" disabled={confirmDisabled} onclick={handleConfirm}>
      Delete project
    </button>
  </div>
</dialog>

<style>
  .confirm-dialog {
    padding: var(--space-lg);
    border: none;
    border-radius: var(--radius-lg);
    background: var(--color-surface-raised);
    color: var(--color-ink);
    box-shadow: var(--shadow-lg);
    width: min(28rem, calc(100vw - 2 * var(--space-lg)));
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
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

  fieldset {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    margin: 0;
    padding: var(--space-sm) var(--space-md);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }

  legend {
    padding: 0 var(--space-2xs);
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-ink-muted);
  }

  .option {
    display: flex;
    align-items: flex-start;
    gap: var(--space-xs);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .option input {
    margin-top: 0.2em;
    accent-color: var(--color-accent);
  }

  .target-picker {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
    margin-left: calc(1em + var(--space-xs));
    padding: var(--space-2xs) 0 0;
  }

  .target-picker label {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-ink-muted);
  }

  .target-picker select {
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
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

  .actions button.danger:hover:not(:disabled) {
    background: var(--color-danger-hover);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .actions button.danger:disabled {
    background: var(--color-border);
    color: var(--color-ink-muted);
    cursor: not-allowed;
    box-shadow: none;
  }
</style>
