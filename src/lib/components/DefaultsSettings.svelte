<script lang="ts">
  import { DUE_RELATIVE_DATE_OPTIONS, SCHEDULED_RELATIVE_DATE_OPTIONS } from "$lib/relativeDates";
  import { persistSettings, settingsState } from "$lib/settings.svelte";
  import { formatTags, parseTags } from "$lib/taskFields";
  import type { TaskDefaults } from "$lib/types";

  let draftDefaultProject = $state("");
  let draftTags = $state("");
  let draftDue = $state("");
  let draftScheduled = $state("");
  let initialized = $state(false);
  let errorMessage = $state("");
  let isSaving = $state(false);

  let baselineDefaultProject = $derived(settingsState.current?.default_project ?? "");
  let baselineTags = $derived(formatTags(settingsState.current?.defaults.tags ?? []));
  let baselineDue = $derived(settingsState.current?.defaults.due ?? "");
  let baselineScheduled = $derived(settingsState.current?.defaults.scheduled ?? "");

  /** Seeds the draft from settings once they finish loading; later edits live only in the draft. */
  $effect(() => {
    if (settingsState.current && !initialized) {
      draftDefaultProject = baselineDefaultProject;
      draftTags = baselineTags;
      draftDue = baselineDue;
      draftScheduled = baselineScheduled;
      initialized = true;
    }
  });

  let isDirty = $derived(
    draftDefaultProject !== baselineDefaultProject ||
      draftTags !== baselineTags ||
      draftDue !== baselineDue ||
      draftScheduled !== baselineScheduled,
  );

  function discardChanges() {
    draftDefaultProject = baselineDefaultProject;
    draftTags = baselineTags;
    draftDue = baselineDue;
    draftScheduled = baselineScheduled;
    errorMessage = "";
  }

  async function save() {
    if (!settingsState.current || !initialized) return;

    const defaultProject = draftDefaultProject.trim();
    if (!defaultProject) {
      errorMessage = "Default project name cannot be empty";
      return;
    }

    const defaults: TaskDefaults = {
      ...settingsState.current.defaults,
      tags: parseTags(draftTags),
      due: draftDue,
      scheduled: draftScheduled,
    };

    isSaving = true;
    try {
      await persistSettings({ ...settingsState.current, default_project: defaultProject, defaults });
      draftDefaultProject = defaultProject;
      draftTags = formatTags(defaults.tags);
      draftDue = defaults.due ?? "";
      draftScheduled = defaults.scheduled ?? "";
      errorMessage = "";
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : "Failed to save defaults";
    } finally {
      isSaving = false;
    }
  }
</script>

<section aria-labelledby="defaults-heading">
  <h2 id="defaults-heading">New task defaults</h2>
  <p class="description">
    Applied to every new task. The default project is used whenever a task isn't created with a
    `+Project` or from a project's board. Quick-add tags are added alongside the default tags,
    and an explicit quick-add due/scheduled date always takes precedence over these defaults.
  </p>

  {#if !settingsState.current}
    <p class="loading">Loading defaults…</p>
  {:else}
    <div class="field">
      <label for="default-project">Default project name</label>
      <input
        id="default-project"
        type="text"
        placeholder="e.g. General"
        bind:value={draftDefaultProject}
      />
    </div>

    <div class="field">
      <label for="default-tags">Default tags</label>
      <input
        id="default-tags"
        type="text"
        placeholder="e.g. inbox, review"
        bind:value={draftTags}
      />
    </div>

    <div class="field">
      <label for="default-scheduled">Default scheduled date</label>
      <select id="default-scheduled" bind:value={draftScheduled}>
        {#each SCHEDULED_RELATIVE_DATE_OPTIONS as option (option.id)}
          <option value={option.id}>{option.label}</option>
        {/each}
      </select>
      <p class="hint">Every task must have a scheduled date.</p>
    </div>

    <div class="field">
      <label for="default-due">Default due date</label>
      <select id="default-due" bind:value={draftDue}>
        {#each DUE_RELATIVE_DATE_OPTIONS as option (option.id)}
          <option value={option.id}>{option.label}</option>
        {/each}
      </select>
      <p class="hint">Relative to the task's scheduled date, not today.</p>
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
  {/if}
</section>

<style>
  section {
    margin-top: var(--space-2xl);
  }

  h2 {
    margin: 0 0 var(--space-md);
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

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
    margin: 0 0 var(--space-md);
    max-width: 20rem;
  }

  .field label {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-ink-muted);
  }

  .field input,
  .field select {
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

  .field input:focus-visible,
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
  }

  .actions button.secondary:hover:not(:disabled) {
    background: var(--color-canvas);
  }
</style>
