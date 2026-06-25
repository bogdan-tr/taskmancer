<script lang="ts">
  import { onMount } from "svelte";
  import { listStatusLayouts, updateProject } from "$lib/api";
  import { getErrorMessage } from "$lib/errors";
  import { FALLBACK_PRIORITIES } from "$lib/priorities.svelte";
  import { refreshProjects } from "$lib/projects.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import {
    buildStatusTierRuleOverrides,
    overriddenTierSlots,
    selectValueToOptional,
    TIER_LABELS,
  } from "$lib/statusTierRuleOverrides";
  import type { Project, StatLayout, StatusTierRule } from "$lib/types";
  import StatusTierRuleFields from "./StatusTierRuleFields.svelte";

  interface Props {
    project: Project;
  }

  let { project }: Props = $props();

  let layouts = $state<StatLayout[]>([]);
  let errorMessage = $state("");
  let isSaving = $state(false);
  let initialized = $state(false);

  onMount(async () => {
    try {
      layouts = await listStatusLayouts();
    } catch {
      // The layout override <select> just shows no extra options besides
      // "Use global default"; nothing else on this panel depends on it.
    }
  });

  let priorities = $derived(settingsState.current?.priorities ?? FALLBACK_PRIORITIES);
  let globalTierRules = $derived(settingsState.current?.default_status_tier_rules);

  let baselineLayoutId = $derived(project.board.status_line_layout_id ?? "");
  let baselineOverridden = $derived(overriddenTierSlots(project.board.status_tier_rule_overrides));
  /** Seeded once from the project's own saved per-slot rule when overridden, or from the matching global tier otherwise — see the `$effect` below for why this only matters at seed time, not on every render. */
  let baselineRules = $derived(
    Array.from({ length: 4 }, (_, index): StatusTierRule => {
      const ownRule = project.board.status_tier_rule_overrides?.[index];
      if (ownRule) return ownRule;
      return globalTierRules?.[index] ?? {};
    }),
  );

  let draftLayoutId = $state("");
  let draftOverridden = $state<boolean[]>([false, false, false, false]);
  let draftRules = $state<StatusTierRule[]>([{}, {}, {}, {}]);

  /** Seeds every draft from the project's board once settings/layouts context is ready; later edits live only in the draft, per this codebase's `ProjectBoardSettings.svelte` convention. */
  $effect(() => {
    if (settingsState.current && !initialized) {
      draftLayoutId = baselineLayoutId;
      draftOverridden = [...baselineOverridden];
      draftRules = baselineRules.map((rule) => ({ ...rule }));
      initialized = true;
    }
  });

  let isDirty = $derived(
    draftLayoutId !== baselineLayoutId ||
      draftOverridden.some((value, index) => value !== baselineOverridden[index]) ||
      draftRules.some((rule, index) => draftOverridden[index] && !rulesEqual(rule, baselineRules[index])),
  );

  function rulesEqual(a: StatusTierRule, b: StatusTierRule): boolean {
    return (
      a.due_within_days === b.due_within_days &&
      a.min_priority === b.min_priority &&
      a.estimated_time_left_exceeds_minutes === b.estimated_time_left_exceeds_minutes
    );
  }

  function handleLayoutChange(event: Event) {
    draftLayoutId = (event.currentTarget as HTMLSelectElement).value;
  }

  /** Checking "override" seeds that tier's draft from the *current global* rule (a sensible starting point to tweak), not from whatever stale value happened to be sitting in `draftRules` from before — unchecking leaves the draft rule as-is so re-checking later doesn't lose in-progress edits within the same unsaved session. */
  function handleOverrideToggle(index: number, checked: boolean) {
    draftOverridden = draftOverridden.map((value, i) => (i === index ? checked : value));
    if (checked) {
      draftRules = draftRules.map((rule, i) => (i === index ? { ...(globalTierRules?.[index] ?? {}) } : rule));
    }
  }

  function handleRuleChange(index: number, rule: StatusTierRule) {
    draftRules = draftRules.map((existing, i) => (i === index ? rule : existing));
  }

  function discardChanges() {
    draftLayoutId = baselineLayoutId;
    draftOverridden = [...baselineOverridden];
    draftRules = baselineRules.map((rule) => ({ ...rule }));
    errorMessage = "";
  }

  async function save() {
    isSaving = true;
    try {
      await updateProject({
        ...project,
        board: {
          ...project.board,
          status_line_layout_id: selectValueToOptional(draftLayoutId),
          status_tier_rule_overrides: buildStatusTierRuleOverrides(draftOverridden, draftRules),
        },
      });
      await refreshProjects();
      errorMessage = "";
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to save status line settings");
    } finally {
      isSaving = false;
    }
  }
</script>

<section aria-labelledby="project-status-line-heading">
  <div class="section-header">
    <h2 id="project-status-line-heading">Status line</h2>
  </div>
  <p class="description">
    Override which layout this project's status line renders, and/or any individual health-badge
    tier, instead of inheriting the global defaults.
  </p>

  <div class="field">
    <label for="project-status-line-layout">Layout</label>
    <select id="project-status-line-layout" value={draftLayoutId} onchange={handleLayoutChange}>
      <option value="">Use global default</option>
      {#each layouts as layout (layout.id)}
        <option value={layout.id}>{layout.name}</option>
      {/each}
    </select>
  </div>

  <h3>Health badge thresholds</h3>
  <div class="tier-list">
    {#each TIER_LABELS as label, index (label)}
      <div class="tier-row">
        <label class="checkbox-row" for="project-tier-override-{index}">
          <input
            id="project-tier-override-{index}"
            type="checkbox"
            checked={draftOverridden[index]}
            onchange={(event) => handleOverrideToggle(index, (event.currentTarget as HTMLInputElement).checked)}
          />
          Override "{label}"
        </label>
        {#if draftOverridden[index]}
          <StatusTierRuleFields
            rule={draftRules[index]}
            {priorities}
            idPrefix="project-tier-{index}"
            onChange={(rule) => handleRuleChange(index, rule)}
          />
        {/if}
      </div>
    {/each}
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

  h3 {
    margin: var(--space-lg) 0 var(--space-md);
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

  .tier-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .tier-row {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
  }

  .checkbox-row {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-ink);
    cursor: pointer;
  }

  .checkbox-row input[type="checkbox"] {
    width: 1.1rem;
    height: 1.1rem;
    accent-color: var(--color-accent);
    cursor: pointer;
  }

  .error {
    margin: var(--space-md) 0;
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
    margin-top: var(--space-lg);
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
