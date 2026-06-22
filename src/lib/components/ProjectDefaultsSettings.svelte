<script lang="ts">
  import { updateProject } from "$lib/api";
  import { getErrorMessage } from "$lib/errors";
  import { hoursAndMinutesFromMinutes, minutesFromHoursAndMinutes, normalizeHoursMinutes } from "$lib/estimatedTime";
  import { projectsState, refreshProjects } from "$lib/projects.svelte";
  import { ancestorsOf } from "$lib/projectTree";
  import { DUE_RELATIVE_DATE_OPTIONS, SCHEDULED_RELATIVE_DATE_OPTIONS } from "$lib/relativeDates";
  import { formatTags, parseTags } from "$lib/taskFields";
  import type { Project, TaskDefaults } from "$lib/types";

  interface Props {
    project: Project;
  }

  let { project }: Props = $props();

  /** The nearest *ancestor* (not including this project itself) with a non-empty `defaults.tags`, if this project hasn't set its own — used to label an inherited (not self-set) value. */
  let tagsInheritedFrom = $derived(
    project.defaults.tags.length === 0
      ? ancestorsOf(projectsState.items, project.id).find((p) => p.defaults.tags.length > 0)?.name
      : undefined,
  );
  let dueInheritedFrom = $derived(
    project.defaults.due === undefined
      ? ancestorsOf(projectsState.items, project.id).find((p) => p.defaults.due !== undefined)?.name
      : undefined,
  );
  let scheduledInheritedFrom = $derived(
    project.defaults.scheduled === undefined
      ? ancestorsOf(projectsState.items, project.id).find((p) => p.defaults.scheduled !== undefined)?.name
      : undefined,
  );
  let estimatedMinutesInheritedFrom = $derived(
    project.defaults.estimated_minutes === undefined
      ? ancestorsOf(projectsState.items, project.id).find((p) => p.defaults.estimated_minutes !== undefined)?.name
      : undefined,
  );

  let baselineTags = $derived(formatTags(project.defaults.tags));
  let baselineDue = $derived(project.defaults.due ?? "");
  let baselineScheduled = $derived(project.defaults.scheduled ?? "");
  let baselineEstimatedMinutes = $derived(project.defaults.estimated_minutes);

  let draftTags = $state("");
  let draftDue = $state("");
  let draftScheduled = $state("");
  let draftEstimatedHours: number | undefined = $state(undefined);
  let draftEstimatedMinutes: number | undefined = $state(undefined);
  let initialized = $state(false);
  let errorMessage = $state("");
  let isSaving = $state(false);

  /** Seeds the draft from the project's defaults once; later edits live only in the draft. */
  $effect(() => {
    if (!initialized) {
      draftTags = baselineTags;
      draftDue = baselineDue;
      draftScheduled = baselineScheduled;
      const resolvedEstimate =
        baselineEstimatedMinutes !== undefined ? hoursAndMinutesFromMinutes(baselineEstimatedMinutes) : undefined;
      draftEstimatedHours = resolvedEstimate?.hours;
      draftEstimatedMinutes = resolvedEstimate?.minutes;
      initialized = true;
    }
  });

  let draftEstimatedTotal = $derived(
    draftEstimatedHours === undefined && draftEstimatedMinutes === undefined
      ? undefined
      : minutesFromHoursAndMinutes(draftEstimatedHours ?? 0, draftEstimatedMinutes ?? 0),
  );

  let isDirty = $derived(
    draftTags !== baselineTags ||
      draftDue !== baselineDue ||
      draftScheduled !== baselineScheduled ||
      draftEstimatedTotal !== baselineEstimatedMinutes,
  );

  function discardChanges() {
    draftTags = baselineTags;
    draftDue = baselineDue;
    draftScheduled = baselineScheduled;
    const resolvedEstimate =
      baselineEstimatedMinutes !== undefined ? hoursAndMinutesFromMinutes(baselineEstimatedMinutes) : undefined;
    draftEstimatedHours = resolvedEstimate?.hours;
    draftEstimatedMinutes = resolvedEstimate?.minutes;
    errorMessage = "";
  }

  /** Rolls minutes >= 60 over into hours, e.g. typing 90 into "mins" reads back as 1h 30m. */
  function normalizeEstimateDraft() {
    if (draftEstimatedHours === undefined && draftEstimatedMinutes === undefined) return;
    const normalized = normalizeHoursMinutes(draftEstimatedHours ?? 0, draftEstimatedMinutes ?? 0);
    draftEstimatedHours = normalized.hours;
    draftEstimatedMinutes = normalized.minutes;
  }

  async function save() {
    const defaults: TaskDefaults = {
      ...project.defaults,
      tags: parseTags(draftTags),
      due: draftDue || undefined,
      scheduled: draftScheduled || undefined,
      estimated_minutes: draftEstimatedTotal,
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
      errorMessage = getErrorMessage(error, "Failed to save defaults");
    } finally {
      isSaving = false;
    }
  }
</script>

<section aria-labelledby="project-defaults-heading">
  <h2 id="project-defaults-heading">New task defaults</h2>
  <p class="description">
    Override the defaults for tasks created in this project. Leave a field unset to inherit it from
    the nearest ancestor that has set it, or the global default if none has.
  </p>

  <div class="field">
    <label for="project-default-tags">Default tags override</label>
    <input
      id="project-default-tags"
      type="text"
      placeholder="Leave blank to inherit the default tags"
      bind:value={draftTags}
    />
    <p class="hint">
      If set, these tags replace the inherited default tags for new tasks in this project.
      Quick-add tags are still merged in on top.
    </p>
    {#if tagsInheritedFrom}
      <p class="inherited-note">Inherited from {tagsInheritedFrom}.</p>
    {/if}
  </div>

  <div class="field">
    <label for="project-default-scheduled">Default scheduled date</label>
    <select id="project-default-scheduled" bind:value={draftScheduled}>
      <option value="">Inherit default</option>
      {#each SCHEDULED_RELATIVE_DATE_OPTIONS as option (option.id)}
        <option value={option.id}>{option.label}</option>
      {/each}
    </select>
    {#if scheduledInheritedFrom}
      <p class="inherited-note">Inherited from {scheduledInheritedFrom}.</p>
    {/if}
  </div>

  <div class="field">
    <label for="project-default-due">Default due date</label>
    <select id="project-default-due" bind:value={draftDue}>
      <option value="">Inherit default</option>
      {#each DUE_RELATIVE_DATE_OPTIONS as option (option.id)}
        <option value={option.id}>{option.label}</option>
      {/each}
    </select>
    <p class="hint">Relative to the task's scheduled date, not today.</p>
    {#if dueInheritedFrom}
      <p class="inherited-note">Inherited from {dueInheritedFrom}.</p>
    {/if}
  </div>

  <div class="field">
    <span class="field-label">Default estimated time override</span>
    <span class="estimate-inputs">
      <input
        type="number"
        min="0"
        step="1"
        placeholder="0"
        bind:value={draftEstimatedHours}
        onblur={normalizeEstimateDraft}
        aria-label="Default estimated hours override"
      />
      hrs
      <input
        type="number"
        min="0"
        step="1"
        placeholder="0"
        bind:value={draftEstimatedMinutes}
        onblur={normalizeEstimateDraft}
        aria-label="Default estimated minutes override"
      />
      mins
    </span>
    {#if estimatedMinutesInheritedFrom}
      <p class="inherited-note">Inherited from {estimatedMinutesInheritedFrom}.</p>
    {/if}
    <p class="hint">Leave both blank to inherit the global default estimate.</p>
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

  .field label,
  .field-label {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-ink-muted);
  }

  .estimate-inputs {
    display: flex;
    align-items: center;
    gap: var(--space-3xs);
  }

  .estimate-inputs input {
    width: 3.5rem;
    flex: none;
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

  .inherited-note {
    margin: var(--space-2xs) 0 0;
    font-size: var(--text-xs);
    color: var(--color-ink-muted);
    font-style: italic;
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
