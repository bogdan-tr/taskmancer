<script lang="ts">
  import { updateProject } from "$lib/api";
  import { boardsEqual, effectiveBoardStatuses } from "$lib/projectBoardSettings";
  import { refreshProjects } from "$lib/projects.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import { FALLBACK_STATUSES, sortedStatuses, statusColor, statusLabel } from "$lib/statuses.svelte";
  import type { Project } from "$lib/types";

  interface Props {
    project: Project;
  }

  let { project }: Props = $props();

  let statuses = $derived(sortedStatuses(settingsState.current?.statuses ?? FALLBACK_STATUSES));
  let allStatusIds = $derived(statuses.map((status) => status.id));

  let baselineStatuses = $derived(effectiveBoardStatuses(project.board, allStatusIds));
  let baselineDefault = $derived(project.board.default_status ?? "");
  /** "" = inherit the global default, "true"/"false" = explicit override. */
  let baselineShowPreviousWeeks = $derived(
    project.board.show_previous_weeks === undefined ? "" : String(project.board.show_previous_weeks),
  );

  let draftStatuses = $state<string[]>([]);
  let draftDefault = $state("");
  let draftShowPreviousWeeks = $state("");
  let initialized = $state(false);
  let errorMessage = $state("");
  let isSaving = $state(false);

  /** Seeds the draft from the project's board once; later edits live only in the draft. */
  $effect(() => {
    if (settingsState.current && !initialized) {
      draftStatuses = [...baselineStatuses];
      draftDefault = baselineDefault;
      draftShowPreviousWeeks = baselineShowPreviousWeeks;
      initialized = true;
    }
  });

  let isDirty = $derived(
    !boardsEqual(
      { statuses: draftStatuses, default_status: draftDefault || undefined },
      { statuses: baselineStatuses, default_status: project.board.default_status },
    ) || draftShowPreviousWeeks !== baselineShowPreviousWeeks,
  );

  let availableStatusIds = $derived(allStatusIds.filter((id) => !draftStatuses.includes(id)));

  function moveUp(index: number) {
    if (index === 0) return;
    const next = [...draftStatuses];
    [next[index - 1], next[index]] = [next[index], next[index - 1]];
    draftStatuses = next;
  }

  function moveDown(index: number) {
    if (index === draftStatuses.length - 1) return;
    moveUp(index + 1);
  }

  function removeStatus(id: string) {
    if (draftStatuses.length <= 1) return;
    draftStatuses = draftStatuses.filter((statusId) => statusId !== id);
    if (draftDefault === id) draftDefault = "";
  }

  function addStatus(id: string) {
    draftStatuses = [...draftStatuses, id];
  }

  function discardChanges() {
    draftStatuses = [...baselineStatuses];
    draftDefault = baselineDefault;
    draftShowPreviousWeeks = baselineShowPreviousWeeks;
    errorMessage = "";
  }

  async function save() {
    isSaving = true;
    try {
      await updateProject({
        ...project,
        board: {
          statuses: draftStatuses,
          default_status: draftDefault || undefined,
          show_previous_weeks: draftShowPreviousWeeks === "" ? undefined : draftShowPreviousWeeks === "true",
        },
      });
      await refreshProjects();
      errorMessage = "";
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : "Failed to save board settings";
    } finally {
      isSaving = false;
    }
  }
</script>

<section aria-labelledby="board-heading">
  <div class="section-header">
    <h2 id="board-heading">Board columns</h2>
  </div>
  <p class="description">
    These statuses appear as columns on this project's board, in this order. Tasks with any other
    status appear in a trailing "Other" column.
  </p>

  <ul class="status-list">
    {#each draftStatuses as id, index (id)}
      <li class="status-row">
        <div class="rank-controls">
          <button
            type="button"
            disabled={index === 0}
            onclick={() => moveUp(index)}
            aria-label={`Move ${statusLabel(statuses, id)} up`}
          >
            ▲
          </button>
          <button
            type="button"
            disabled={index === draftStatuses.length - 1}
            onclick={() => moveDown(index)}
            aria-label={`Move ${statusLabel(statuses, id)} down`}
          >
            ▼
          </button>
        </div>

        <span class="swatch" style="background: {statusColor(statuses, id)}" aria-hidden="true"
        ></span>
        <span class="status-name">{statusLabel(statuses, id)}</span>

        <button
          type="button"
          class="secondary"
          disabled={draftStatuses.length <= 1}
          title={draftStatuses.length <= 1
            ? "At least one column is required"
            : `Move ${statusLabel(statuses, id)} to Other statuses`}
          aria-label={`Remove ${statusLabel(statuses, id)} from this board`}
          onclick={() => removeStatus(id)}
        >
          Remove
        </button>
      </li>
    {/each}
  </ul>

  {#if availableStatusIds.length > 0}
    <h3 id="other-statuses-heading">Other statuses</h3>
    <ul class="status-list">
      {#each availableStatusIds as id (id)}
        <li class="status-row">
          <span class="swatch" style="background: {statusColor(statuses, id)}" aria-hidden="true"
          ></span>
          <span class="status-name">{statusLabel(statuses, id)}</span>
          <button
            type="button"
            class="secondary"
            aria-label={`Add ${statusLabel(statuses, id)} to this board`}
            onclick={() => addStatus(id)}
          >
            Add to board
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  <div class="field">
    <label for="default-status">Default status for new tasks in this project</label>
    <select id="default-status" bind:value={draftDefault}>
      <option value="">Use global default</option>
      {#each draftStatuses as id (id)}
        <option value={id}>{statusLabel(statuses, id)}</option>
      {/each}
    </select>
  </div>

  <div class="field">
    <label for="show-previous-weeks">Week view "Previous" column</label>
    <select id="show-previous-weeks" bind:value={draftShowPreviousWeeks}>
      <option value="">Use global default</option>
      <option value="true">Always show</option>
      <option value="false">Always hide</option>
    </select>
    <p class="hint">
      Overrides the global Display setting for this project's Week view only.
    </p>
  </div>

  {#if errorMessage}
    <p class="error" role="alert">{errorMessage}</p>
  {/if}

  <div class="actions">
    <button type="button" class="secondary" disabled={!isDirty || isSaving} onclick={discardChanges}>
      Discard changes
    </button>
    <button type="button" disabled={!isDirty || isSaving} onclick={save}>
      {isSaving ? "Saving…" : "Save changes"}
    </button>
  </div>
</section>

<style>
  section {
    margin-top: var(--space-2xl);
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-md);
  }

  .section-header h2 {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
  }

  .description {
    margin: 0 0 var(--space-md);
    font-size: var(--text-sm);
    color: var(--color-ink-muted);
  }

  h3 {
    margin: var(--space-lg) 0 var(--space-md);
    font-size: var(--text-sm);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
  }

  .status-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    list-style: none;
    margin: 0 0 var(--space-md);
    padding: 0;
  }

  .status-row {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
  }

  .rank-controls {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex-shrink: 0;
  }

  .rank-controls button {
    width: 1.5rem;
    height: 1.25rem;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink-muted);
    font-size: var(--text-xs);
    line-height: 1;
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out-expo);
  }

  .rank-controls button:hover:not(:disabled) {
    background: var(--color-canvas);
    color: var(--color-ink);
  }

  .rank-controls button:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .swatch {
    width: 1.25rem;
    height: 1.25rem;
    border-radius: var(--radius-pill);
    border: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .status-name {
    flex: 1;
    min-width: 0;
    font-size: var(--text-sm);
  }

  button.secondary {
    flex-shrink: 0;
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
    font-weight: 600;
    font-size: var(--text-xs);
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out-expo);
  }

  button.secondary:hover:not(:disabled) {
    background: var(--color-canvas);
  }

  button.secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
    margin: var(--space-lg) 0;
    max-width: 20rem;
  }

  .field label {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-ink-muted);
  }

  .field select {
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
    font-size: var(--text-sm);
    box-shadow: var(--shadow-sm);
  }

  .field select:focus-visible {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-soft);
    outline: none;
  }

  .hint {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--color-ink-faint);
  }

  .error {
    margin: 0 0 var(--space-md);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    background: var(--color-danger-soft);
    color: var(--color-danger);
    font-weight: 600;
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

  .actions button:not(.secondary) {
    background: var(--color-accent);
    color: var(--color-accent-ink);
  }

  .actions button:not(.secondary):hover:not(:disabled) {
    background: var(--color-accent-hover);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .actions button:disabled {
    background: var(--color-border);
    color: var(--color-ink-muted);
    cursor: not-allowed;
    box-shadow: none;
    transform: none;
  }

  .actions button.secondary {
    background: var(--color-surface);
    color: var(--color-ink);
    border: 1px solid var(--color-border);
    box-shadow: none;
    padding: var(--space-sm) var(--space-lg);
    font-size: var(--text-base);
  }

  .actions button.secondary:hover:not(:disabled) {
    background: var(--color-canvas);
  }
</style>
