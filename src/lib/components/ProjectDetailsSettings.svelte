<script lang="ts">
  import { updateProject } from "$lib/api";
  import { refreshProjects } from "$lib/projects.svelte";
  import { cssColorToHex, isHexColor } from "$lib/colorPresets";
  import ColorPicker from "$lib/components/ColorPicker.svelte";
  import type { Project } from "$lib/types";

  interface Props {
    project: Project;
  }

  let { project }: Props = $props();

  let baselineName = $derived(project.name);
  // Normalized to hex so `baseline` matches what `ColorPicker` displays (its
  // `$effect` migrates legacy oklch values to hex on display) — otherwise the
  // form would appear dirty as soon as it loads a legacy color.
  let baselineColor = $derived(cssColorToHex(project.color));

  let draftName = $state("");
  let draftColor = $state("");
  let initialized = $state(false);
  let errorMessage = $state("");
  let isSaving = $state(false);

  /** Seeds the draft from the project's details once; later edits live only in the draft. */
  $effect(() => {
    if (!initialized) {
      draftName = baselineName;
      draftColor = baselineColor;
      initialized = true;
    }
  });

  let isDirty = $derived(draftName !== baselineName || draftColor !== baselineColor);

  function discardChanges() {
    draftName = baselineName;
    draftColor = baselineColor;
    errorMessage = "";
  }

  async function save() {
    const trimmedName = draftName.trim();
    const trimmedColor = draftColor.trim();
    if (trimmedName === "") {
      errorMessage = "Name can't be empty";
      return;
    }
    if (trimmedColor !== baselineColor && !isHexColor(trimmedColor)) {
      errorMessage = "Color must be a hex value like #3b82f6";
      return;
    }

    isSaving = true;
    try {
      await updateProject({ ...project, name: trimmedName, color: trimmedColor });
      await refreshProjects();
      draftName = trimmedName;
      draftColor = trimmedColor;
      errorMessage = "";
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : "Failed to save project details";
    } finally {
      isSaving = false;
    }
  }
</script>

<section aria-labelledby="project-details-heading">
  <h2 id="project-details-heading">Project details</h2>

  <div class="field">
    <label for="project-name">Name</label>
    <input id="project-name" type="text" bind:value={draftName} />
  </div>

  <div class="field">
    <span class="field-label">Color</span>
    <ColorPicker bind:value={draftColor} label="Project color" />
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

  .field input {
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

  .field input:focus-visible {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-soft);
    outline: none;
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
