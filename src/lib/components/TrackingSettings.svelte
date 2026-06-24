<script lang="ts">
  import { getErrorMessage } from "$lib/errors";
  import { persistSettings, settingsState } from "$lib/settings.svelte";
  import { sortedStatuses } from "$lib/statuses.svelte";

  /** Sentinel `<option>` value representing "unset" (falls back to `resolveAutoTransitionStatusId`'s runtime default) — kept distinct from any real status id, which is always a non-empty, app-generated string. */
  const DEFAULT_OPTION_VALUE = "";

  let isSaving = $state(false);
  let errorMessage = $state("");

  async function handleAutoTransitionEnabledChange(event: Event) {
    if (!settingsState.current) return;
    const checked = (event.currentTarget as HTMLInputElement).checked;

    isSaving = true;
    try {
      await persistSettings({ ...settingsState.current, tracking_auto_transition_enabled: checked });
      errorMessage = "";
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to save");
    } finally {
      isSaving = false;
    }
  }

  /** Persists the chosen target status, or clears the field back to `undefined` when the leading "Use default" option is reselected. */
  async function handleAutoTransitionStatusChange(event: Event) {
    if (!settingsState.current) return;
    const value = (event.currentTarget as HTMLSelectElement).value;

    isSaving = true;
    try {
      await persistSettings({
        ...settingsState.current,
        tracking_auto_transition_status_id: value === DEFAULT_OPTION_VALUE ? undefined : value,
      });
      errorMessage = "";
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to save");
    } finally {
      isSaving = false;
    }
  }

  async function handleCardTrackedTimeDisplayChange(event: Event) {
    if (!settingsState.current) return;
    const value = (event.currentTarget as HTMLSelectElement).value as "total" | "session";

    isSaving = true;
    try {
      await persistSettings({ ...settingsState.current, card_tracked_time_display: value });
      errorMessage = "";
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to save");
    } finally {
      isSaving = false;
    }
  }
</script>

<section aria-labelledby="tracking-heading">
  <div class="section-header">
    <h2 id="tracking-heading">Time tracking</h2>
  </div>

  {#if !settingsState.current}
    <p class="loading">Loading…</p>
  {:else}
    <label class="toggle-row">
      <input
        type="checkbox"
        checked={settingsState.current.tracking_auto_transition_enabled}
        onchange={handleAutoTransitionEnabledChange}
        disabled={isSaving}
      />
      <span class="toggle-text">
        <span class="toggle-label">Auto-transition status when tracking starts</span>
        <span class="toggle-description">
          When you start the timer on a task, automatically move it to the status chosen below.
          Stopping the timer never moves it back — that stays fully manual.
        </span>
      </span>
    </label>

    <label class="select-row" class:disabled={!settingsState.current.tracking_auto_transition_enabled}>
      <span class="toggle-text">
        <span class="toggle-label">Status to transition to</span>
        <span class="toggle-description">
          "Use default" picks the first status that isn't backlog, done, or cancelled.
        </span>
      </span>
      <select
        value={settingsState.current.tracking_auto_transition_status_id ?? DEFAULT_OPTION_VALUE}
        onchange={handleAutoTransitionStatusChange}
        disabled={isSaving || !settingsState.current.tracking_auto_transition_enabled}
        aria-label="Status to transition to"
      >
        <option value={DEFAULT_OPTION_VALUE}>Use default (first active status)</option>
        {#each sortedStatuses(settingsState.current.statuses) as status (status.id)}
          <option value={status.id}>{status.label}</option>
        {/each}
      </select>
    </label>

    <label class="select-row">
      <span class="toggle-text">
        <span class="toggle-label">Card timer display while running</span>
        <span class="toggle-description">
          "Total time" continues from a task's full tracked history across every pause/resume.
          "Current session only" restarts from 0:00 each time you resume — useful for seeing how
          long you've been at it right now. Only affects the live ticker; the chip shown once a
          timer is stopped always shows the full total either way. The sidebar's "timers running"
          list always shows the full total too, regardless of this setting.
        </span>
      </span>
      <select
        value={settingsState.current.card_tracked_time_display}
        onchange={handleCardTrackedTimeDisplayChange}
        disabled={isSaving}
        aria-label="Card timer display while running"
      >
        <option value="total">Total time</option>
        <option value="session">Current session only</option>
      </select>
    </label>

    {#if errorMessage}
      <p class="error" role="alert">{errorMessage}</p>
    {/if}
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

  .loading {
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
  }

  .toggle-row {
    display: flex;
    align-items: flex-start;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    cursor: pointer;
  }

  .toggle-row input[type="checkbox"] {
    margin-top: 0.2rem;
    width: 1.1rem;
    height: 1.1rem;
    flex-shrink: 0;
    accent-color: var(--color-accent);
    cursor: pointer;
  }

  .select-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-md);
    margin-top: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
  }

  .select-row.disabled {
    opacity: 0.6;
  }

  .select-row select {
    flex-shrink: 0;
    max-width: 14rem;
    padding: var(--space-2xs) var(--space-xs);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-canvas);
    color: var(--color-ink);
    font: inherit;
    font-size: var(--text-sm);
  }

  .select-row select:focus-visible {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-soft);
    outline: none;
  }

  .toggle-text {
    display: flex;
    flex-direction: column;
    gap: var(--space-3xs);
  }

  .toggle-label {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-ink);
  }

  .toggle-description {
    font-size: var(--text-xs);
    color: var(--color-ink-muted);
  }

  .error {
    margin-top: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    background: var(--color-danger-soft);
    color: var(--color-danger);
    font-weight: 600;
    font-size: var(--text-sm);
  }
</style>
