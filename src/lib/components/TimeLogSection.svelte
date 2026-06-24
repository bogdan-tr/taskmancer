<script lang="ts">
  import { addManualTimeEntry, deleteTimeEntry, listTimeEntries, updateTimeEntry } from "$lib/api";
  import { getErrorMessage } from "$lib/errors";
  import {
    datetimeLocalValueToIso,
    formatEntryDuration,
    formatEntryTimestamp,
    isoToDatetimeLocalValue,
    validateManualEntryRange,
  } from "$lib/timeLogDisplay";
  import { refreshTasks } from "$lib/tasks.svelte";
  import type { Task, TimeEntry } from "$lib/types";

  interface Props {
    task: Task;
  }

  let { task }: Props = $props();

  /**
   * Completed entries only (`ended_at !== null`) — the currently-active
   * session, if any, is already represented by this dialog's own play/pause
   * ticker elsewhere, not by a row here (it has no `ended_at` to edit/delete
   * against yet).
   */
  let completedEntries: TimeEntry[] = $state([]);
  let loadError = $state("");
  let isLoading = $state(false);

  let editingEntryId: string | undefined = $state(undefined);
  let editDraftStart = $state("");
  let editDraftEnd = $state("");
  let editError = $state("");

  let addFormOpen = $state(false);
  let addDraftStart = $state("");
  let addDraftEnd = $state("");
  let addError = $state("");
  let isSubmittingAdd = $state(false);

  /** Re-fetches this task's time entries. Guards a stale response against the dialog having moved on to a different task in the meantime, mirroring `TaskEditDialog`'s own `loadSeriesInfo` guard. */
  async function loadEntries(taskId: string) {
    isLoading = true;
    loadError = "";
    try {
      const entries = await listTimeEntries(taskId);
      if (task.id === taskId) {
        completedEntries = entries.filter((entry) => entry.ended_at !== null);
      }
    } catch (error) {
      if (task.id === taskId) {
        loadError = getErrorMessage(error, "Failed to load time log");
      }
    } finally {
      if (task.id === taskId) isLoading = false;
    }
  }

  $effect(() => {
    void loadEntries(task.id);
  });

  /** Re-fetches both this section's own list and the global task cache (whose `tracked_minutes` the backend just recomputed) after any successful mutation. */
  async function afterMutation() {
    await loadEntries(task.id);
    await refreshTasks();
  }

  function startEdit(entry: TimeEntry) {
    editingEntryId = entry.id;
    editDraftStart = isoToDatetimeLocalValue(entry.started_at);
    editDraftEnd = entry.ended_at ? isoToDatetimeLocalValue(entry.ended_at) : "";
    editError = "";
  }

  function cancelEdit() {
    editingEntryId = undefined;
    editError = "";
  }

  async function saveEdit(entryId: string) {
    const startedAt = datetimeLocalValueToIso(editDraftStart);
    const endedAt = datetimeLocalValueToIso(editDraftEnd);
    if (!startedAt || !endedAt) {
      editError = "Start and end date/time are required";
      return;
    }

    const rangeError = validateManualEntryRange(startedAt, endedAt);
    if (rangeError) {
      editError = rangeError;
      return;
    }

    editError = "";
    try {
      await updateTimeEntry(entryId, startedAt, endedAt);
      editingEntryId = undefined;
      await afterMutation();
    } catch (error) {
      editError = getErrorMessage(error, "Failed to save time entry");
    }
  }

  async function handleDelete(entryId: string) {
    loadError = "";
    try {
      await deleteTimeEntry(entryId);
      await afterMutation();
    } catch (error) {
      loadError = getErrorMessage(error, "Failed to delete time entry");
    }
  }

  function openAddForm() {
    addFormOpen = true;
    addDraftStart = "";
    addDraftEnd = "";
    addError = "";
  }

  function closeAddForm() {
    addFormOpen = false;
    addError = "";
  }

  async function submitAddForm(event: Event) {
    event.preventDefault();
    const startedAt = datetimeLocalValueToIso(addDraftStart);
    const endedAt = datetimeLocalValueToIso(addDraftEnd);
    if (!startedAt || !endedAt) {
      addError = "Start and end date/time are required";
      return;
    }

    const rangeError = validateManualEntryRange(startedAt, endedAt);
    if (rangeError) {
      addError = rangeError;
      return;
    }

    addError = "";
    isSubmittingAdd = true;
    try {
      await addManualTimeEntry(task.id, startedAt, endedAt);
      addFormOpen = false;
      await afterMutation();
    } catch (error) {
      addError = getErrorMessage(error, "Failed to add time entry");
    } finally {
      isSubmittingAdd = false;
    }
  }
</script>

<section class="time-log" aria-labelledby="time-log-heading">
  <div class="time-log-header">
    <h3 id="time-log-heading">Time log</h3>
    {#if !addFormOpen}
      <button type="button" class="add-entry-link" onclick={openAddForm}>+ Add manual entry</button>
    {/if}
  </div>

  {#if addFormOpen}
    <form class="add-entry-form" onsubmit={submitAddForm}>
      <label>
        Start
        <input type="datetime-local" bind:value={addDraftStart} required />
      </label>
      <label>
        End
        <input type="datetime-local" bind:value={addDraftEnd} required />
      </label>
      {#if addError}
        <p class="time-log-error" role="alert">{addError}</p>
      {/if}
      <div class="add-entry-actions">
        <button type="submit" disabled={isSubmittingAdd}>{isSubmittingAdd ? "Adding…" : "Add"}</button>
        <button type="button" class="secondary" onclick={closeAddForm}>Cancel</button>
      </div>
    </form>
  {/if}

  {#if isLoading}
    <p class="time-log-status">Loading…</p>
  {:else if loadError}
    <p class="time-log-error" role="alert">{loadError}</p>
  {:else if completedEntries.length === 0}
    <p class="time-log-status">No completed time entries yet.</p>
  {:else}
    <ul class="time-log-list">
      {#each completedEntries as entry (entry.id)}
        <li class="time-log-row">
          {#if editingEntryId === entry.id}
            <div class="edit-entry-form">
              <label>
                Start
                <input type="datetime-local" bind:value={editDraftStart} required />
              </label>
              <label>
                End
                <input type="datetime-local" bind:value={editDraftEnd} required />
              </label>
              {#if editError}
                <p class="time-log-error" role="alert">{editError}</p>
              {/if}
              <div class="edit-entry-actions">
                <button type="button" onclick={() => saveEdit(entry.id)}>Save</button>
                <button type="button" class="secondary" onclick={cancelEdit}>Cancel</button>
              </div>
            </div>
          {:else}
            <div class="time-log-row-info">
              <span class="time-log-row-range">
                {formatEntryTimestamp(entry.started_at)} – {entry.ended_at
                  ? formatEntryTimestamp(entry.ended_at)
                  : ""}
              </span>
              <span class="time-log-row-duration">
                {entry.ended_at ? formatEntryDuration(entry.started_at, entry.ended_at) : ""}
              </span>
            </div>
            <div class="time-log-row-actions">
              <button type="button" class="link-button" onclick={() => startEdit(entry)}>Edit</button>
              <button type="button" class="link-button danger" onclick={() => handleDelete(entry.id)}>
                Delete
              </button>
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .time-log {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    padding-top: var(--space-md);
    border-top: 1px solid var(--color-border);
  }

  .time-log-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-sm);
  }

  .time-log-header h3 {
    margin: 0;
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
  }

  .add-entry-link {
    padding: 0;
    border: none;
    background: transparent;
    color: var(--color-ink-muted);
    font-size: var(--text-xs);
    font-weight: 600;
    text-decoration: underline;
    cursor: pointer;
  }

  .add-entry-link:hover {
    color: var(--color-accent);
  }

  .time-log-status {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--color-ink-muted);
  }

  .time-log-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
    margin: 0;
    padding: 0;
    list-style: none;
    max-height: 14rem;
    overflow-y: auto;
  }

  .time-log-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-sm);
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-sm);
    background: var(--color-canvas);
    font-size: var(--text-xs);
  }

  .time-log-row-info {
    display: flex;
    flex-direction: column;
    gap: var(--space-3xs);
    min-width: 0;
  }

  .time-log-row-range {
    color: var(--color-ink);
    font-weight: 500;
  }

  .time-log-row-duration {
    color: var(--color-accent);
    font-weight: 700;
  }

  .time-log-row-actions {
    display: flex;
    gap: var(--space-xs);
    flex-shrink: 0;
  }

  .link-button {
    padding: 0;
    border: none;
    background: transparent;
    color: var(--color-ink-muted);
    font-size: var(--text-xs);
    font-weight: 600;
    text-decoration: underline;
    cursor: pointer;
  }

  .link-button:hover {
    color: var(--color-accent);
  }

  .link-button.danger:hover {
    color: var(--color-danger);
  }

  .add-entry-form,
  .edit-entry-form {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--space-sm);
    padding: var(--space-sm);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-canvas);
  }

  .add-entry-form label,
  .edit-entry-form label {
    display: flex;
    flex-direction: column;
    gap: var(--space-3xs);
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--color-ink-muted);
  }

  .add-entry-form input,
  .edit-entry-form input {
    font: inherit;
    font-size: var(--text-sm);
    padding: var(--space-2xs) var(--space-xs);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
  }

  .add-entry-actions,
  .edit-entry-actions {
    display: flex;
    gap: var(--space-xs);
  }

  .add-entry-actions button,
  .edit-entry-actions button {
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-md);
    border: none;
    font-weight: 600;
    font-size: var(--text-xs);
    cursor: pointer;
    background: var(--color-accent);
    color: var(--color-accent-ink);
  }

  .add-entry-actions button.secondary,
  .edit-entry-actions button.secondary {
    background: var(--color-surface);
    color: var(--color-ink);
    border: 1px solid var(--color-border);
  }

  .add-entry-actions button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .time-log-error {
    margin: 0;
    padding: var(--space-2xs) var(--space-xs);
    border-radius: var(--radius-sm);
    background: var(--color-danger-soft);
    color: var(--color-danger);
    font-size: var(--text-xs);
    font-weight: 600;
    width: 100%;
  }
</style>
