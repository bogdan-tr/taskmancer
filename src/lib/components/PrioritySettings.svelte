<script lang="ts">
  import { onMount } from "svelte";
  import { countTasksByPriority } from "$lib/api";
  import { deleteBlockReason, levelsEqual, renumber, uniqueId } from "$lib/prioritySettings";
  import { FALLBACK_PRIORITY_COLOR, sortedPriorities } from "$lib/priorities.svelte";
  import { persistSettings, settingsState } from "$lib/settings.svelte";
  import type { PriorityLevel } from "$lib/types";

  let draft = $state<PriorityLevel[]>([]);
  let initialized = $state(false);
  let taskCounts = $state<Record<string, number>>({});
  let errorMessage = $state("");
  let isSaving = $state(false);

  let baseline = $derived(sortedPriorities(settingsState.current?.priorities ?? []));
  let isDirty = $derived(!levelsEqual(draft, baseline));
  let defaultPriorityId = $derived(settingsState.current?.defaults.priority);

  /** Seeds `draft` from settings once they finish loading; later edits live only in `draft`. */
  $effect(() => {
    if (settingsState.current && !initialized) {
      draft = sortedPriorities(settingsState.current.priorities).map((level) => ({ ...level }));
      initialized = true;
    }
  });

  onMount(async () => {
    try {
      taskCounts = await countTasksByPriority();
    } catch {
      // Non-critical: deletion guards just won't reflect live task counts.
    }
  });

  function addLevel() {
    const id = uniqueId(draft.map((level) => level.id), "new-priority");
    draft = renumber([...draft, { id, label: "New priority", color: FALLBACK_PRIORITY_COLOR, rank: 0 }]);
  }

  function removeLevel(id: string) {
    draft = renumber(draft.filter((level) => level.id !== id));
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
    draft = baseline.map((level) => ({ ...level }));
    errorMessage = "";
  }

  async function save() {
    if (!settingsState.current) return;

    const trimmed = draft.map((level) => ({
      ...level,
      label: level.label.trim(),
      color: level.color.trim(),
    }));
    if (trimmed.some((level) => level.label === "" || level.color === "")) {
      errorMessage = "Labels and colors can't be empty";
      return;
    }
    if (trimmed.some((level) => !CSS.supports("color", level.color))) {
      errorMessage = "Colors must be valid CSS color values";
      return;
    }

    isSaving = true;
    try {
      await persistSettings({ ...settingsState.current, priorities: trimmed });
      draft = trimmed;
      errorMessage = "";
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : "Failed to save priority levels";
    } finally {
      isSaving = false;
    }
  }
</script>

<section aria-labelledby="priority-heading">
  <div class="section-header">
    <h2 id="priority-heading">Priority levels</h2>
    <button type="button" class="add-button" onclick={addLevel}>+ Add level</button>
  </div>

  {#if !settingsState.current}
    <p class="loading">Loading priority levels…</p>
  {:else}
    <ul class="priority-list">
      {#each draft as level, index (level.id)}
        {@const blockReason = deleteBlockReason(level, draft.length, defaultPriorityId, taskCounts)}
        {@const taskCount = taskCounts[level.id] ?? 0}
        <li class="priority-row">
          <div class="rank-controls">
            <button
              type="button"
              disabled={index === 0}
              onclick={() => moveUp(index)}
              aria-label={`Move ${level.label || "priority"} up`}
            >
              ▲
            </button>
            <button
              type="button"
              disabled={index === draft.length - 1}
              onclick={() => moveDown(index)}
              aria-label={`Move ${level.label || "priority"} down`}
            >
              ▼
            </button>
          </div>

          <input
            type="text"
            class="label-input"
            bind:value={draft[index].label}
            aria-label="Priority label"
          />

          <div class="color-field">
            <span class="swatch" style="background: {draft[index].color}" aria-hidden="true"></span>
            <input
              type="text"
              class="color-input"
              bind:value={draft[index].color}
              aria-label={`Color for ${level.label || "priority"}`}
            />
          </div>

          <div class="row-meta">
            {#if level.id === defaultPriorityId}
              <span class="badge">Default</span>
            {/if}
            {#if taskCount > 0}
              <span class="badge">{taskCount} task{taskCount === 1 ? "" : "s"}</span>
            {/if}
          </div>

          <button
            type="button"
            class="danger"
            disabled={!!blockReason}
            title={blockReason ?? "Delete this priority level"}
            aria-label={`Delete ${level.label || "priority"}`}
            onclick={() => removeLevel(level.id)}
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

  .priority-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    list-style: none;
    margin: 0 0 var(--space-md);
    padding: 0;
  }

  .priority-row {
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

  .priority-row button.danger {
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

  .priority-row button.danger:hover:not(:disabled) {
    background: var(--color-danger-hover);
  }

  .priority-row button.danger:disabled {
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
