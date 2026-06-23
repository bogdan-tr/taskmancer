<script lang="ts">
  import { getErrorMessage } from "$lib/errors";
  import { persistSettings, settingsState } from "$lib/settings.svelte";

  let isSaving = $state(false);
  let errorMessage = $state("");

  async function handleIncludeOwnEstimateChange(event: Event) {
    if (!settingsState.current) return;
    const checked = (event.currentTarget as HTMLInputElement).checked;

    isSaving = true;
    try {
      await persistSettings({ ...settingsState.current, parent_estimate_includes_own_value: checked });
      errorMessage = "";
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to save");
    } finally {
      isSaving = false;
    }
  }

  /** Ignores anything that doesn't parse to a non-negative integer, leaving the stored setting untouched — the number input's own `min="0"` already discourages this, but a typed/pasted value can still bypass it. */
  async function handleMaxVisibleSubtasksChange(event: Event) {
    if (!settingsState.current) return;
    const parsed = Number.parseInt((event.currentTarget as HTMLInputElement).value, 10);
    if (!Number.isInteger(parsed) || parsed < 0) return;

    isSaving = true;
    try {
      await persistSettings({ ...settingsState.current, max_visible_subtasks: parsed });
      errorMessage = "";
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to save");
    } finally {
      isSaving = false;
    }
  }
</script>

<section aria-labelledby="subtasks-heading">
  <div class="section-header">
    <h2 id="subtasks-heading">Subtasks</h2>
  </div>

  {#if !settingsState.current}
    <p class="loading">Loading…</p>
  {:else}
    <label class="toggle-row">
      <input
        type="checkbox"
        checked={settingsState.current.parent_estimate_includes_own_value}
        onchange={handleIncludeOwnEstimateChange}
        disabled={isSaving}
      />
      <span class="toggle-text">
        <span class="toggle-label">Add a task's own estimate on top of its subtasks' total</span>
        <span class="toggle-description">
          When a task has subtasks, its displayed estimated time is normally just the sum of its
          subtasks' own estimates. Turn this on to add the task's own estimate on top of that sum
          instead of being replaced by it. Display only — never changes any stored estimate.
        </span>
      </span>
    </label>

    <label class="number-row">
      <span class="toggle-text">
        <span class="toggle-label">Max subtasks shown on a card</span>
        <span class="toggle-description">
          A parent card's nested subtask preview shows up to this many rows before collapsing the
          rest into a "+N more" line. Set to 0 to always collapse them all.
        </span>
      </span>
      <input
        type="number"
        min="0"
        step="1"
        value={settingsState.current.max_visible_subtasks}
        onchange={handleMaxVisibleSubtasksChange}
        disabled={isSaving}
        aria-label="Max subtasks shown on a card"
      />
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

  .number-row {
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

  .number-row input[type="number"] {
    flex-shrink: 0;
    width: 4rem;
    padding: var(--space-3xs) var(--space-2xs);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-canvas);
    color: var(--color-ink);
    font: inherit;
    font-size: var(--text-sm);
    text-align: right;
  }

  .number-row input[type="number"]:focus-visible {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-soft);
    outline: none;
  }

  .toggle-row input[type="checkbox"] {
    margin-top: 0.2rem;
    width: 1.1rem;
    height: 1.1rem;
    flex-shrink: 0;
    accent-color: var(--color-accent);
    cursor: pointer;
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
