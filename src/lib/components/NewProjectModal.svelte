<script lang="ts">
  import { createProject } from "$lib/api";
  import { shadesOf } from "$lib/colorPresets";
  import { DEFAULT_PROJECT_COLOR, type Project } from "$lib/types";
  import ColorPicker from "$lib/components/ColorPicker.svelte";

  interface Props {
    open: boolean;
    onClose: () => void;
    onCreated: (project: Project) => void;
    /** When set, the created project becomes a subproject of this one. */
    parentProject?: Project;
  }

  let { open, onClose, onCreated, parentProject }: Props = $props();

  let dialogEl: HTMLDialogElement | undefined = $state();
  let inputEl: HTMLInputElement | undefined = $state();
  let name = $state("");
  let color = $state(DEFAULT_PROJECT_COLOR);
  let errorMessage = $state("");
  let isSubmitting = $state(false);

  $effect(() => {
    if (!dialogEl) return;
    if (open) {
      if (!dialogEl.open) {
        name = "";
        color = parentProject ? shadesOf(parentProject.color, 5)[0] : DEFAULT_PROJECT_COLOR;
        errorMessage = "";
        dialogEl.showModal();
        inputEl?.focus();
      }
    } else if (dialogEl.open) {
      dialogEl.close();
    }
  });

  /** Closes the dialog when a click lands on the `::backdrop`, not its content box. */
  function handleBackdropClick(event: MouseEvent) {
    if (!dialogEl || event.target !== dialogEl) return;

    const rect = dialogEl.getBoundingClientRect();
    const insideContent =
      event.clientX >= rect.left &&
      event.clientX <= rect.right &&
      event.clientY >= rect.top &&
      event.clientY <= rect.bottom;

    if (!insideContent) {
      dialogEl.close();
    }
  }

  async function handleSubmit(event: Event) {
    event.preventDefault();
    const trimmed = name.trim();
    if (!trimmed) return;

    isSubmitting = true;
    try {
      const project = await createProject(trimmed, color, parentProject?.id);
      errorMessage = "";
      onCreated(project);
      dialogEl?.close();
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : "Failed to create project";
    } finally {
      isSubmitting = false;
    }
  }
</script>

<dialog
  bind:this={dialogEl}
  class="new-project-modal"
  aria-labelledby="new-project-heading"
  onclose={onClose}
  onclick={handleBackdropClick}
>
  <form onsubmit={handleSubmit}>
    <header class="modal-header">
      <h2 id="new-project-heading">{parentProject ? `New subproject of ${parentProject.name}` : "New project"}</h2>
      <button
        type="button"
        class="close-button"
        onclick={() => dialogEl?.close()}
        aria-label="Close"
        title="Close"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <line x1="18" y1="6" x2="6" y2="18" />
          <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
      </button>
    </header>

    <label class="field">
      <span class="field-label">Name</span>
      <input
        bind:this={inputEl}
        type="text"
        bind:value={name}
        placeholder="e.g. Homework"
        aria-label="Project name"
      />
    </label>

    <div class="field">
      <span class="field-label">Color</span>
      <ColorPicker
        bind:value={color}
        label="Project color"
        parentColor={parentProject?.color}
        parentName={parentProject?.name}
      />
    </div>

    {#if errorMessage}
      <p class="error" role="alert">{errorMessage}</p>
    {/if}

    <div class="actions">
      <button type="button" class="secondary" onclick={() => dialogEl?.close()}>Cancel</button>
      <button type="submit" disabled={name.trim() === "" || isSubmitting}>Create project</button>
    </div>
  </form>
</dialog>

<style>
  .new-project-modal {
    padding: 0;
    border: none;
    border-radius: var(--radius-lg);
    background: var(--color-surface-raised);
    color: var(--color-ink);
    box-shadow: var(--shadow-lg);
    width: min(28rem, calc(100vw - 2 * var(--space-lg)));
    max-height: calc(100vh - 2 * var(--space-2xl));
  }

  .new-project-modal::backdrop {
    background: oklch(20% 0.02 50 / 0.45);
  }

  .new-project-modal form {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    padding: var(--space-lg);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-sm);
  }

  .modal-header h2 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: 600;
    letter-spacing: var(--tracking-tight);
  }

  .close-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    flex-shrink: 0;
    border-radius: var(--radius-md);
    border: 1px solid transparent;
    background: transparent;
    color: var(--color-ink-muted);
    cursor: pointer;
    transition:
      color var(--duration-fast) var(--ease-out-expo),
      background var(--duration-fast) var(--ease-out-expo);
  }

  .close-button:hover {
    color: var(--color-ink);
    background: var(--color-canvas);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
  }

  .field-label {
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
  }

  .field input[type="text"] {
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
    font-size: var(--text-base);
    box-shadow: var(--shadow-sm);
    transition:
      border-color var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo);
  }

  .field input[type="text"]:focus-visible {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-soft);
    outline: none;
  }

  .error {
    margin: 0;
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

  .actions button[type="submit"] {
    background: var(--color-accent);
    color: var(--color-accent-ink);
  }

  .actions button[type="submit"]:hover {
    background: var(--color-accent-hover);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .actions button[type="submit"]:disabled {
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

  .actions button.secondary:hover {
    background: var(--color-canvas);
  }
</style>
