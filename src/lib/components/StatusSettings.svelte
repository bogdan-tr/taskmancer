<script lang="ts">
  import { onMount } from "svelte";
  import { countTasksByStatus } from "$lib/api";
  import { projectsState } from "$lib/projects.svelte";
  import { persistSettings, settingsState } from "$lib/settings.svelte";
  import { deleteBlockReason, projectsReferencingStatus, renumber, statusesEqual, uniqueId } from "$lib/statusSettings";
  import { FALLBACK_STATUS_COLOR, sortedStatuses } from "$lib/statuses.svelte";
  import type { StatusDefinition } from "$lib/types";

  let draft = $state<StatusDefinition[]>([]);
  let initialized = $state(false);
  let taskCounts = $state<Record<string, number>>({});
  let errorMessage = $state("");
  let isSaving = $state(false);

  let baseline = $derived(sortedStatuses(settingsState.current?.statuses ?? []));
  let isDirty = $derived(!statusesEqual(draft, baseline));
  let defaultStatusId = $derived(settingsState.current?.defaults.status);

  /** Seeds `draft` from settings once they finish loading; later edits live only in `draft`. */
  $effect(() => {
    if (settingsState.current && !initialized) {
      draft = sortedStatuses(settingsState.current.statuses).map((status) => ({ ...status }));
      initialized = true;
    }
  });

  onMount(async () => {
    try {
      taskCounts = await countTasksByStatus();
    } catch {
      // Non-critical: deletion guards just won't reflect live task counts.
    }
  });

  function addStatus() {
    const id = uniqueId(draft.map((status) => status.id), "new-status");
    draft = renumber([...draft, { id, label: "New status", color: FALLBACK_STATUS_COLOR, order: 0 }]);
  }

  function removeStatus(id: string) {
    draft = renumber(draft.filter((status) => status.id !== id));
  }

  function moveUp(index: number) {
    if (index === 0) return;
    const next = [...draft];
    [next[index - 1], next[index]] = [next[index], next[index - 1]];
    draft = renumber(next);
  }

  function moveDown(index: number) {
    if (index === draft.length - 1) return;
    moveUp(index + 1);
  }

  function discardChanges() {
    draft = baseline.map((status) => ({ ...status }));
    errorMessage = "";
  }

  async function save() {
    if (!settingsState.current) return;

    const trimmed = draft.map((status) => ({
      ...status,
      label: status.label.trim(),
      color: status.color.trim(),
    }));
    if (trimmed.some((status) => status.label === "" || status.color === "")) {
      errorMessage = "Labels and colors can't be empty";
      return;
    }
    if (trimmed.some((status) => !CSS.supports("color", status.color))) {
      errorMessage = "Colors must be valid CSS color values";
      return;
    }

    isSaving = true;
    try {
      await persistSettings({ ...settingsState.current, statuses: trimmed });
      draft = trimmed;
      errorMessage = "";
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : "Failed to save statuses";
    } finally {
      isSaving = false;
    }
  }
</script>

<section aria-labelledby="status-heading">
  <div class="section-header">
    <h2 id="status-heading">Statuses</h2>
    <button type="button" class="add-button" onclick={addStatus}>+ Add status</button>
  </div>

  {#if !settingsState.current}
    <p class="loading">Loading statuses…</p>
  {:else}
    <ul class="status-list">
      {#each draft as status, index (status.id)}
        {@const referencingProjects = projectsReferencingStatus(projectsState.items, status.id)}
        {@const blockReason = deleteBlockReason(status, draft.length, defaultStatusId, taskCounts, referencingProjects)}
        {@const taskCount = taskCounts[status.id] ?? 0}
        <li class="status-row">
          <div class="rank-controls">
            <button
              type="button"
              disabled={index === 0}
              onclick={() => moveUp(index)}
              aria-label={`Move ${status.label || "status"} up`}
            >
              ▲
            </button>
            <button
              type="button"
              disabled={index === draft.length - 1}
              onclick={() => moveDown(index)}
              aria-label={`Move ${status.label || "status"} down`}
            >
              ▼
            </button>
          </div>

          <input
            type="text"
            class="label-input"
            bind:value={draft[index].label}
            aria-label="Status label"
          />

          <div class="color-field">
            <span class="swatch" style="background: {draft[index].color}" aria-hidden="true"></span>
            <input
              type="text"
              class="color-input"
              bind:value={draft[index].color}
              aria-label={`Color for ${status.label || "status"}`}
            />
          </div>

          <div class="row-meta">
            {#if status.id === defaultStatusId}
              <span class="badge">Default</span>
            {/if}
            {#if taskCount > 0}
              <span class="badge">{taskCount} task{taskCount === 1 ? "" : "s"}</span>
            {/if}
            {#if referencingProjects.length > 0}
              <span class="badge" title={referencingProjects.join(", ")}>
                {referencingProjects.length} board{referencingProjects.length === 1 ? "" : "s"}
              </span>
            {/if}
          </div>

          <button
            type="button"
            class="danger"
            disabled={!!blockReason}
            title={blockReason ?? "Delete this status"}
            aria-label={`Delete ${status.label || "status"}`}
            onclick={() => removeStatus(status.id)}
          >
            Delete
          </button>
        </li>
      {/each}
    </ul>

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
  {/if}
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

  .add-button {
    padding: var(--space-2xs) var(--space-md);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
    font-weight: 600;
    font-size: var(--text-sm);
    cursor: pointer;
    box-shadow: var(--shadow-sm);
    transition:
      background var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo),
      transform var(--duration-fast) var(--ease-out-expo);
  }

  .add-button:hover {
    background: var(--color-canvas);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .loading {
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
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

  .label-input {
    flex: 1;
    min-width: 0;
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
    font-size: var(--text-sm);
    box-shadow: var(--shadow-sm);
    transition:
      border-color var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo);
  }

  .label-input:focus-visible,
  .color-input:focus-visible {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-soft);
    outline: none;
  }

  .color-field {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    flex-shrink: 0;
  }

  .swatch {
    width: 1.25rem;
    height: 1.25rem;
    border-radius: var(--radius-pill);
    border: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .color-input {
    width: 11rem;
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
    font-size: var(--text-xs);
    box-shadow: var(--shadow-sm);
    transition:
      border-color var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo);
  }

  .row-meta {
    display: flex;
    gap: var(--space-2xs);
    flex-shrink: 0;
  }

  .badge {
    padding: var(--space-3xs) var(--space-xs);
    border-radius: var(--radius-pill);
    background: var(--color-canvas);
    border: 1px solid var(--color-border);
    color: var(--color-ink-muted);
    font-size: var(--text-xs);
    white-space: nowrap;
  }

  .status-row button.danger {
    flex-shrink: 0;
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: var(--color-danger);
    color: var(--color-accent-ink);
    font-weight: 600;
    font-size: var(--text-xs);
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out-expo);
  }

  .status-row button.danger:hover:not(:disabled) {
    background: var(--color-danger-hover);
  }

  .status-row button.danger:disabled {
    background: var(--color-border);
    color: var(--color-ink-muted);
    cursor: not-allowed;
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
  }

  .actions button.secondary:hover:not(:disabled) {
    background: var(--color-canvas);
  }
</style>
