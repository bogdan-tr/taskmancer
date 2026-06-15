<script lang="ts">
  import { tick } from "svelte";
  import {
    applyTokenSuggestion,
    filterSuggestions,
    findActiveToken,
    type ActiveToken,
  } from "$lib/autocomplete";
  import { parseTaskInput, type ParsedTaskInput } from "$lib/naturalLanguage";
  import { FALLBACK_PRIORITIES, priorityColor, priorityLabel } from "$lib/priorities.svelte";
  import { projectsState } from "$lib/projects.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import { tagsState } from "$lib/tags.svelte";
  import { resolveTaskPreview } from "$lib/taskPreview";
  import Autocomplete from "./Autocomplete.svelte";

  interface Props {
    open: boolean;
    onClose: () => void;
    onSubmit: (parsed: ParsedTaskInput) => Promise<void> | void;
    errorMessage?: string;
    /** When set, this dialog was opened from a project-scoped board: new tasks default to this project. */
    projectFilter?: string;
  }

  let { open, onClose, onSubmit, errorMessage = "", projectFilter }: Props = $props();

  let dialogEl: HTMLDialogElement | undefined = $state();
  let inputEl: HTMLInputElement | undefined = $state();
  let title = $state("");

  let priorities = $derived(settingsState.current?.priorities ?? FALLBACK_PRIORITIES);
  let knownPriorities = $derived(priorities.map(({ id, label }) => ({ id, label })));
  let parsed = $derived(parseTaskInput(title, undefined, knownPriorities));

  let defaultProjectName = $derived(settingsState.current?.default_project ?? "General");
  let globalDefaults = $derived(settingsState.current?.defaults ?? { tags: [] });

  /**
   * The project the task will be created under: the `+Project` quick-add
   * token, else this dialog's `projectFilter`, else the configured default
   * project. Looked up case-insensitively (mirroring `find_project` in the
   * Rust command layer) to resolve its `TaskDefaults` overrides.
   */
  let resolvedProjectName = $derived(parsed.project ?? projectFilter ?? defaultProjectName);
  let matchedProject = $derived(
    projectsState.items.find((project) => project.name.toLowerCase() === resolvedProjectName.toLowerCase()),
  );

  /** The effective project, priority, tags, due, and scheduled this task will be created with. */
  let preview = $derived(
    resolveTaskPreview({
      parsed,
      projectFilter,
      defaultProjectName,
      globalDefaults,
      projectDefaults: matchedProject?.defaults,
      matchedProjectName: matchedProject?.name,
      priorities,
    }),
  );

  // The `+Project` quick-add token is a single whitespace-delimited word, so
  // only single-word project names can be completed through it.
  let projectNames = $derived(
    projectsState.items.map((project) => project.name).filter((name) => !/\s/.test(name)),
  );

  let activeToken: ActiveToken | undefined = $state();
  let suggestions: string[] = $state([]);
  let activeSuggestionIndex = $state(0);

  $effect(() => {
    if (!dialogEl) return;
    if (open) {
      if (!dialogEl.open) {
        title = "";
        suggestions = [];
        activeToken = undefined;
        dialogEl.showModal();
        inputEl?.focus();
        inputEl?.setSelectionRange(title.length, title.length);
      }
    } else if (dialogEl.open) {
      dialogEl.close();
    }
  });

  /** Recomputes the active token and its suggestions from the input's current value and cursor. */
  function updateSuggestions() {
    if (!inputEl) return;

    const value = inputEl.value;
    const cursor = inputEl.selectionStart ?? value.length;
    const token = findActiveToken(value, cursor);
    activeToken = token;

    if (!token) {
      suggestions = [];
      return;
    }

    const options = token.prefix === "#" ? tagsState.items : projectNames;
    suggestions = filterSuggestions(options, token.text);
    activeSuggestionIndex = 0;
  }

  async function selectSuggestion(suggestion: string) {
    if (!activeToken) return;

    const result = applyTokenSuggestion(title, activeToken, suggestion);
    title = result.value;
    suggestions = [];
    activeToken = undefined;

    await tick();
    inputEl?.setSelectionRange(result.cursor, result.cursor);
    inputEl?.focus();
  }

  function handleTitleKeydown(event: KeyboardEvent) {
    if (suggestions.length === 0) return;

    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        activeSuggestionIndex = (activeSuggestionIndex + 1) % suggestions.length;
        break;
      case "ArrowUp":
        event.preventDefault();
        activeSuggestionIndex = (activeSuggestionIndex - 1 + suggestions.length) % suggestions.length;
        break;
      case "Enter":
      case "Tab":
        event.preventDefault();
        void selectSuggestion(suggestions[activeSuggestionIndex]);
        break;
      case "Escape":
        // Stop the keydown from also dismissing the dialog.
        event.preventDefault();
        suggestions = [];
        activeToken = undefined;
        break;
    }
  }

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
    if (!parsed.title) return;
    await onSubmit({ ...parsed, project: preview.project });
  }
</script>

<dialog
  bind:this={dialogEl}
  class="add-task-modal"
  aria-labelledby="add-task-heading"
  onclose={onClose}
  onclick={handleBackdropClick}
>
  <form onsubmit={handleSubmit}>
    <header class="modal-header">
      <h2 id="add-task-heading">Add task</h2>
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

    <div class="title-field">
      <input
        bind:this={inputEl}
        type="text"
        bind:value={title}
        placeholder="What needs doing? (#tag +project, high/medium/low, due friday, sch next monday)"
        aria-label="New task title"
        title="Quick-add syntax: #tag, +Project, high/medium/low (or !h/!m/!l), due/sch <today|tomorrow|YYYY-MM-DD|weekday|next weekday|month day[ year]>. 'next weekday' skips the upcoming one (e.g. 'next monday' is a week later than 'monday')."
        role="combobox"
        aria-expanded={suggestions.length > 0}
        aria-controls="add-task-suggestions"
        aria-autocomplete="list"
        aria-activedescendant={suggestions.length > 0
          ? `add-task-suggestions-option-${activeSuggestionIndex}`
          : undefined}
        oninput={updateSuggestions}
        onclick={updateSuggestions}
        onkeyup={(event) => {
          if (["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) updateSuggestions();
        }}
        onkeydown={handleTitleKeydown}
        onblur={() => (suggestions = [])}
      />
      <Autocomplete
        id="add-task-suggestions"
        items={suggestions}
        activeIndex={activeSuggestionIndex}
        onSelect={selectSuggestion}
        onHover={(index) => (activeSuggestionIndex = index)}
        prefix={activeToken?.prefix ?? ""}
      />
    </div>

    <dl class="field-list">
      <div class="field-row">
        <dt>Project</dt>
        <dd class="filled">{preview.project}</dd>
      </div>
      <div class="field-row">
        <dt>Priority</dt>
        <dd class="filled" style={`color: ${priorityColor(priorities, preview.priorityId)}`}>
          {priorityLabel(priorities, preview.priorityId)}
        </dd>
      </div>
      <div class="field-row">
        <dt>Tags</dt>
        <dd class="tags">
          {#if preview.tags.length > 0}
            {#each preview.tags as tag (tag)}
              <span class="chip">#{tag}</span>
            {/each}
          {:else}
            —
          {/if}
        </dd>
      </div>
      <div class="field-row">
        <dt>Scheduled</dt>
        <dd class:filled={!!preview.scheduled}>{preview.scheduled ?? "—"}</dd>
      </div>
      <div class="field-row">
        <dt>Due</dt>
        <dd class:filled={!!preview.due}>{preview.due ?? "—"}</dd>
      </div>
    </dl>

    {#if errorMessage}
      <p class="error" role="alert">{errorMessage}</p>
    {/if}

    <div class="actions">
      <button type="button" class="secondary" onclick={() => dialogEl?.close()}>Cancel</button>
      <button type="submit" disabled={parsed.title === ""}>Add task</button>
    </div>
  </form>
</dialog>

<style>
  .add-task-modal {
    padding: 0;
    border: none;
    border-radius: var(--radius-lg);
    background: var(--color-surface-raised);
    color: var(--color-ink);
    box-shadow: var(--shadow-lg);
    width: min(32rem, calc(100vw - 2 * var(--space-lg)));
    max-height: calc(100vh - 2 * var(--space-2xl));
  }

  .add-task-modal::backdrop {
    background: oklch(20% 0.02 50 / 0.45);
  }

  .add-task-modal form {
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

  .title-field {
    position: relative;
  }

  .add-task-modal input[type="text"] {
    width: 100%;
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

  .add-task-modal input[type="text"]:focus-visible {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-soft);
    outline: none;
  }

  .field-list {
    display: flex;
    flex-direction: column;
    margin: 0;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    overflow: hidden;
  }

  .field-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-md);
    padding: var(--space-2xs) var(--space-md);
  }

  .field-row + .field-row {
    border-top: 1px solid var(--color-border);
  }

  .field-row dt {
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
  }

  .field-row dd {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--color-ink-faint);
    text-align: right;
  }

  .field-row dd.filled {
    color: var(--color-ink);
    font-weight: 600;
  }

  .field-row dd.tags {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: var(--space-3xs);
  }

  .chip {
    font-size: var(--text-xs);
    line-height: var(--leading-tight);
    padding: var(--space-3xs) var(--space-xs);
    border-radius: var(--radius-pill);
    background: var(--color-accent-soft);
    border: 1px solid transparent;
    color: var(--color-accent);
    font-weight: 600;
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
