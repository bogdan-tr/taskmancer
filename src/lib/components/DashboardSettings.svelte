<script lang="ts">
  import { onMount } from "svelte";
  import { listStatusLayouts } from "$lib/api";
  import { getErrorMessage } from "$lib/errors";
  import { persistSettings, settingsState } from "$lib/settings.svelte";
  import type { StatLayout } from "$lib/types";

  let layouts = $state<StatLayout[]>([]);
  let isSaving = $state(false);
  let errorMessage = $state("");

  onMount(async () => {
    try {
      const all = await listStatusLayouts();
      layouts = all.filter((l) => l.kind === "dashboard");
    } catch {
      // cosmetic only — a missing layout list just leaves the select empty
    }
  });

  async function handleDefaultLayoutChange(event: Event) {
    if (!settingsState.current) return;
    const value = (event.currentTarget as HTMLSelectElement).value;
    isSaving = true;
    try {
      await persistSettings({ ...settingsState.current, default_dashboard_layout_id: value });
      errorMessage = "";
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to save");
    } finally {
      isSaving = false;
    }
  }
</script>

<section aria-labelledby="dashboard-settings-heading">
  <div class="section-header">
    <h2 id="dashboard-settings-heading">Dashboard</h2>
  </div>
  <p class="description">
    Configure the default widget layout shown on the global and per-project dashboards.
  </p>

  {#if settingsState.current}
    <div class="field">
      <label for="default-dashboard-layout">Default layout</label>
      <select
        id="default-dashboard-layout"
        value={settingsState.current.default_dashboard_layout_id}
        onchange={handleDefaultLayoutChange}
        disabled={isSaving}
      >
        <option value="">— No layout selected —</option>
        {#each layouts as layout (layout.id)}
          <option value={layout.id}>{layout.name}</option>
        {/each}
      </select>
    </div>
  {/if}

  {#if errorMessage}
    <p class="error" role="alert">{errorMessage}</p>
  {/if}
</section>

<style>
  section {
    margin-top: var(--space-2xl);
  }

  .section-header h2 {
    margin: 0 0 var(--space-sm);
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

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
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
    cursor: pointer;
  }

  .field select:focus-visible {
    border-color: var(--color-accent);
    outline: none;
    box-shadow: 0 0 0 3px var(--color-accent-soft);
  }

  .error {
    margin-top: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    background: var(--color-danger-soft);
    color: var(--color-danger);
    font-size: var(--text-sm);
  }
</style>
