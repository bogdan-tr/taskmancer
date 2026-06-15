<script lang="ts">
  import { updateProject } from "$lib/api";
  import { refreshProjects } from "$lib/projects.svelte";
  import { RELATIVE_DATE_OPTIONS } from "$lib/relativeDates";
  import { formatTags, parseTags } from "$lib/taskFields";
  import type { Project, TaskDefaults } from "$lib/types";

  interface Props {
    project: Project;
  }

  let { project }: Props = $props();

  let baselineTags = $derived(formatTags(project.defaults.tags));
  let baselineDue = $derived(project.defaults.due ?? "");
  let baselineScheduled = $derived(project.defaults.scheduled ?? "");

  let draftTags = $state("");
  let draftDue = $state("");
  let draftScheduled = $state("");
  let initialized = $state(false);
  let errorMessage = $state("");
  let isSaving = $state(false);

  /** Seeds the draft from the project's defaults once; later edits live only in the draft. */
  $effect(() => {
    if (!initialized) {
      draftTags = baselineTags;
      draftDue = baselineDue;
      draftScheduled = baselineScheduled;
      initialized = true;
    }
  });

  let isDirty = $derived(
    draftTags !== baselineTags || draftDue !== baselineDue || draftScheduled !== baselineScheduled,
  );

  function discardChanges() {
    draftTags = baselineTags;
    draftDue = baselineDue;
    draftScheduled = baselineScheduled;
    errorMessage = "";
  }

  async function save() {
    const defaults: TaskDefaults = {
      ...project.defaults,
      tags: parseTags(draftTags),
      due: draftDue || undefined,
      scheduled: draftScheduled || undefined,
    };

    isSaving = true;
    try {
      await updateProject({ ...project, defaults });
      await refreshProjects();
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

<section aria-labelledby="project-defaults-heading">
  <h2 id="project-defaults-heading">New task defaults</h2>
  <p class="description">
    Override the global defaults for tasks created in this project. Leave a field on its inherited
    value to keep using the global default.
  </p>

  <div class="field">
    <label for="project-default-tags">Default tags override</label>
    <input
      id="project-default-tags"
      type="text"
      placeholder="Leave blank to use the global default tags"
      bind:value={draftTags}
    />
    <p class="hint">
      If set, these tags replace the global default tags for new tasks in this project.
      Quick-add tags are still merged in on top.
    </p>
  </div>

  <div class="field">
    <label for="project-default-due">Default due date</label>
    <select id="project-default-due" bind:value={draftDue}>
      <option value="">Inherit global default</option>
      {#each RELATIVE_DATE_OPTIONS as option (option.id)}
        <option value={option.id}>{option.label}</option>
      {/each}
    </select>
  </div>

  <div class="field">
    <label for="project-default-scheduled">Default scheduled date</label>
    <select id="project-default-scheduled" bind:value={draftScheduled}>
      <option value="">Inherit global default</option>
      {#each RELATIVE_DATE_OPTIONS as option (option.id)}
        <option value={option.id}>{option.label}</option>
      {/each}
    </select>
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
