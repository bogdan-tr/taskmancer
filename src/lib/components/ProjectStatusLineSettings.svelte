<script lang="ts">
  import { onMount } from "svelte";
  import { listStatusLayouts } from "$lib/api";
  import { getErrorMessage } from "$lib/errors";
  import { FALLBACK_PRIORITIES } from "$lib/priorities.svelte";
  import { persistSettings, settingsState } from "$lib/settings.svelte";
  import { TIER_LABELS } from "$lib/statusTierRuleOverrides";
  import type { StatLayout, StatusTierRule } from "$lib/types";
  import StatusTierRuleFields from "./StatusTierRuleFields.svelte";

  let layouts = $state<StatLayout[]>([]);
  let isSaving = $state(false);
  let errorMessage = $state("");

  onMount(async () => {
    try {
      layouts = await listStatusLayouts();
    } catch {
      // The default-layout <select> just shows no options besides whatever
      // is already selected; every other control on this panel is unaffected.
    }
  });

  let priorities = $derived(settingsState.current?.priorities ?? FALLBACK_PRIORITIES);

  async function handleDefaultLayoutChange(event: Event) {
    if (!settingsState.current) return;
    const value = (event.currentTarget as HTMLSelectElement).value;

    isSaving = true;
    try {
      await persistSettings({ ...settingsState.current, default_status_line_layout_id: value });
      errorMessage = "";
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to save");
    } finally {
      isSaving = false;
    }
  }

  async function handleBarStyleChange(event: Event) {
    if (!settingsState.current) return;
    const value = (event.currentTarget as HTMLSelectElement).value as "tiles" | "chips" | "tint";

    isSaving = true;
    try {
      await persistSettings({ ...settingsState.current, status_bar_style: value });
      errorMessage = "";
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to save");
    } finally {
      isSaving = false;
    }
  }

  /** Ignores anything that doesn't parse to a positive integer, leaving the stored setting untouched — mirrors `SubtasksSettings.svelte`'s `handleMaxVisibleSubtasksChange`, except 0 is invalid here (a 0-week trailing average is meaningless). */
  async function handleAvgTimePerWeekWindowChange(event: Event) {
    if (!settingsState.current) return;
    const parsed = Number.parseInt((event.currentTarget as HTMLInputElement).value, 10);
    if (!Number.isInteger(parsed) || parsed <= 0) return;

    isSaving = true;
    try {
      await persistSettings({ ...settingsState.current, avg_time_per_week_window: parsed });
      errorMessage = "";
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to save");
    } finally {
      isSaving = false;
    }
  }

  async function handleTierRuleChange(tierIndex: number, rule: StatusTierRule) {
    if (!settingsState.current) return;
    const nextRules = settingsState.current.default_status_tier_rules.map((existing, index) =>
      index === tierIndex ? rule : existing,
    );

    isSaving = true;
    try {
      await persistSettings({ ...settingsState.current, default_status_tier_rules: nextRules });
      errorMessage = "";
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to save");
    } finally {
      isSaving = false;
    }
  }
</script>

<section aria-labelledby="status-line-heading">
  <div class="section-header">
    <h2 id="status-line-heading">Project status line</h2>
  </div>

  {#if !settingsState.current}
    <p class="loading">Loading…</p>
  {:else}
    <label class="number-row">
      <span class="toggle-text">
        <span class="toggle-label">Default layout</span>
        <span class="toggle-description">
          Which saved layout a project's status line renders when it hasn't set its own override.
        </span>
      </span>
      <select
        value={settingsState.current.default_status_line_layout_id}
        onchange={handleDefaultLayoutChange}
        disabled={isSaving}
        aria-label="Default layout"
      >
        {#each layouts as layout (layout.id)}
          <option value={layout.id}>{layout.name}</option>
        {/each}
      </select>
    </label>

    <label class="number-row">
      <span class="toggle-text">
        <span class="toggle-label">Bar style</span>
        <span class="toggle-description">
          The visual treatment for every project's status line: a row of labeled tiles, small rounded
          chips, or plain inline text with a tinted background.
        </span>
      </span>
      <select value={settingsState.current.status_bar_style} onchange={handleBarStyleChange} disabled={isSaving} aria-label="Bar style">
        <option value="tiles">Tiles</option>
        <option value="chips">Chips</option>
        <option value="tint">Tint</option>
      </select>
    </label>

    <label class="number-row">
      <span class="toggle-text">
        <span class="toggle-label">Average-per-week window</span>
        <span class="toggle-description">
          How many trailing complete weeks the "Avg/week" stat averages over. The current,
          still-in-progress week never counts.
        </span>
      </span>
      <input
        type="number"
        min="1"
        step="1"
        value={settingsState.current.avg_time_per_week_window}
        onchange={handleAvgTimePerWeekWindowChange}
        disabled={isSaving}
        aria-label="Average-per-week window"
      />
    </label>

    <h3>Health badge thresholds</h3>
    <p class="description">
      Each tier matches when every condition it has set is true (an unset condition is skipped). The
      most severe matching tier wins; a project matching none of them shows "Great".
    </p>
    <div class="tier-list">
      {#each settingsState.current.default_status_tier_rules as rule, index (index)}
        <div class="tier-row">
          <span class="tier-label">{TIER_LABELS[index]}</span>
          <StatusTierRuleFields
            {rule}
            {priorities}
            disabled={isSaving}
            idPrefix="global-tier-{index}"
            onChange={(next) => handleTierRuleChange(index, next)}
          />
        </div>
      {/each}
    </div>

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

  h3 {
    margin: var(--space-lg) 0 var(--space-2xs);
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

  .loading {
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
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

  .number-row select {
    flex-shrink: 0;
    padding: var(--space-3xs) var(--space-2xs);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-canvas);
    color: var(--color-ink);
    font: inherit;
    font-size: var(--text-sm);
  }

  .number-row input[type="number"]:focus-visible,
  .number-row select:focus-visible {
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

  .tier-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .tier-row {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
  }

  .tier-label {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-ink);
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
