<script lang="ts">
  import { generalState, setConfirmTaskDeletion } from "$lib/generalSettings.svelte";

  function handleConfirmTaskDeletionChange(event: Event) {
    setConfirmTaskDeletion((event.currentTarget as HTMLInputElement).checked);
  }
</script>

<section aria-labelledby="general-heading">
  <div class="section-header">
    <h2 id="general-heading">General</h2>
  </div>

  <label class="toggle-row">
    <input
      type="checkbox"
      checked={generalState.confirmTaskDeletion}
      onchange={handleConfirmTaskDeletionChange}
    />
    <span class="toggle-text">
      <span class="toggle-label">Confirm task deletion</span>
      <span class="toggle-description">
        Show a confirmation dialog before deleting a task. When off, the Delete button removes
        the task immediately.
      </span>
    </span>
  </label>
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
</style>
