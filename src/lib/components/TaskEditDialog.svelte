<script lang="ts">
  import { applyTagsSuggestion, filterSuggestions, splitTagsInput } from "$lib/autocomplete";
  import { displayState } from "$lib/displaySettings.svelte";
  import { formatDueDateDisplay } from "$lib/dueDateDisplay";
  import { FALLBACK_PRIORITIES, sortedPriorities } from "$lib/priorities.svelte";
  import { projectsState } from "$lib/projects.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import { FALLBACK_STATUSES, sortedStatuses } from "$lib/statuses.svelte";
  import { emptyToUndefined, formatTags, isValidOptionalDate, parseTags } from "$lib/taskFields";
  import { tagsState } from "$lib/tags.svelte";
  import type { Task } from "$lib/types";
  import Autocomplete from "./Autocomplete.svelte";

  interface Props {
    open: boolean;
    task: Task | undefined;
    onSave: (task: Task) => void;
    onCancel: () => void;
  }

  let { open, task, onSave, onCancel }: Props = $props();

  const priorities = $derived(settingsState.current?.priorities ?? FALLBACK_PRIORITIES);
  const statuses = $derived(sortedStatuses(settingsState.current?.statuses ?? FALLBACK_STATUSES));

  let dialogEl: HTMLDialogElement | undefined = $state();

  let draftTitle = $state("");
  let draftProject = $state("");
  let draftTags = $state("");
  let draftPriority = $state("medium");
  let draftStatus = $state("");
  let draftDue = $state("");
  let draftScheduled = $state("");
  let draftNotes = $state("");
  let editError = $state("");

  let projectSuggestions: string[] = $state([]);
  let projectSuggestionIndex = $state(0);
  let tagSuggestions: string[] = $state([]);
  let tagSuggestionIndex = $state(0);

  /** Opens/closes the dialog and (re)initializes the draft fields from `task` whenever it opens. */
  $effect(() => {
    if (!dialogEl) return;
    if (open && task) {
      draftTitle = task.title;
      draftProject = task.project ?? "";
      draftTags = formatTags(task.tags);
      draftPriority = task.priority;
      draftStatus = task.status;
      draftDue = task.due ?? "";
      draftScheduled = task.scheduled ?? "";
      draftNotes = task.notes;
      editError = "";
      projectSuggestions = [];
      tagSuggestions = [];
      if (!dialogEl.open) dialogEl.showModal();
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
      onCancel();
    }
  }

  function updateProjectSuggestions() {
    projectSuggestions = filterSuggestions(
      projectsState.items.map((project) => project.name),
      draftProject,
    );
    projectSuggestionIndex = 0;
  }

  function selectProjectSuggestion(suggestion: string) {
    draftProject = suggestion;
    projectSuggestions = [];
  }

  function handleProjectKeydown(event: KeyboardEvent) {
    if (projectSuggestions.length === 0) return;

    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        projectSuggestionIndex = (projectSuggestionIndex + 1) % projectSuggestions.length;
        break;
      case "ArrowUp":
        event.preventDefault();
        projectSuggestionIndex =
          (projectSuggestionIndex - 1 + projectSuggestions.length) % projectSuggestions.length;
        break;
      case "Enter":
      case "Tab":
        event.preventDefault();
        selectProjectSuggestion(projectSuggestions[projectSuggestionIndex]);
        break;
      case "Escape":
        event.preventDefault();
        projectSuggestions = [];
        break;
    }
  }

  function updateTagSuggestions() {
    const { current } = splitTagsInput(draftTags);
    tagSuggestions = filterSuggestions(tagsState.items, current);
    tagSuggestionIndex = 0;
  }

  function selectTagSuggestion(suggestion: string) {
    const { prefix } = splitTagsInput(draftTags);
    draftTags = applyTagsSuggestion(prefix, suggestion);
    tagSuggestions = [];
  }

  function handleTagsKeydown(event: KeyboardEvent) {
    if (tagSuggestions.length === 0) return;

    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        tagSuggestionIndex = (tagSuggestionIndex + 1) % tagSuggestions.length;
        break;
      case "ArrowUp":
        event.preventDefault();
        tagSuggestionIndex = (tagSuggestionIndex - 1 + tagSuggestions.length) % tagSuggestions.length;
        break;
      case "Enter":
      case "Tab":
        event.preventDefault();
        selectTagSuggestion(tagSuggestions[tagSuggestionIndex]);
        break;
      case "Escape":
        event.preventDefault();
        tagSuggestions = [];
        break;
    }
  }

  function saveEdit(event: Event) {
    event.preventDefault();
    if (!task) return;

    if (!isValidOptionalDate(draftDue) || !isValidOptionalDate(draftScheduled)) {
      editError = "Due and scheduled dates must be in YYYY-MM-DD format";
      return;
    }

    onSave({
      ...task,
      title: draftTitle,
      project: emptyToUndefined(draftProject),
      tags: parseTags(draftTags),
      priority: draftPriority,
      status: draftStatus,
      due: emptyToUndefined(draftDue),
      scheduled: emptyToUndefined(draftScheduled),
      notes: draftNotes,
    });
  }
</script>

<dialog
  bind:this={dialogEl}
  class="task-edit-dialog"
  aria-labelledby="task-edit-heading"
  onclose={onCancel}
  onclick={handleBackdropClick}
>
  {#if task}
    <h2 id="task-edit-heading">Edit task</h2>
    <form class="edit-form" onsubmit={saveEdit}>
      <label>
        Title
        <input type="text" bind:value={draftTitle} required />
      </label>
      <label>
        Project
        <div class="field-with-suggestions">
          <input
            type="text"
            bind:value={draftProject}
            placeholder="e.g. Inbox/Personal"
            role="combobox"
            aria-expanded={projectSuggestions.length > 0}
            aria-controls="task-edit-project-suggestions"
            aria-autocomplete="list"
            aria-activedescendant={projectSuggestions.length > 0
              ? `task-edit-project-suggestions-option-${projectSuggestionIndex}`
              : undefined}
            oninput={updateProjectSuggestions}
            onkeydown={handleProjectKeydown}
            onblur={() => (projectSuggestions = [])}
          />
          <Autocomplete
            id="task-edit-project-suggestions"
            items={projectSuggestions}
            activeIndex={projectSuggestionIndex}
            onSelect={selectProjectSuggestion}
            onHover={(index) => (projectSuggestionIndex = index)}
          />
        </div>
      </label>
      <label>
        Tags
        <div class="field-with-suggestions">
          <input
            type="text"
            bind:value={draftTags}
            placeholder="comma, separated"
            role="combobox"
            aria-expanded={tagSuggestions.length > 0}
            aria-controls="task-edit-tags-suggestions"
            aria-autocomplete="list"
            aria-activedescendant={tagSuggestions.length > 0
              ? `task-edit-tags-suggestions-option-${tagSuggestionIndex}`
              : undefined}
            oninput={updateTagSuggestions}
            onkeydown={handleTagsKeydown}
            onblur={() => (tagSuggestions = [])}
          />
          <Autocomplete
            id="task-edit-tags-suggestions"
            items={tagSuggestions}
            activeIndex={tagSuggestionIndex}
            onSelect={selectTagSuggestion}
            onHover={(index) => (tagSuggestionIndex = index)}
            prefix="#"
          />
        </div>
      </label>
      <div class="field-row">
        <label>
          Priority
          <select bind:value={draftPriority}>
            {#each sortedPriorities(priorities) as level (level.id)}
              <option value={level.id}>{level.label}</option>
            {/each}
          </select>
        </label>
        <label>
          Status
          <select bind:value={draftStatus}>
            {#each statuses as status (status.id)}
              <option value={status.id}>{status.label}</option>
            {/each}
          </select>
        </label>
      </div>
      <div class="field-row">
        <label>
          Due
          <input type="text" bind:value={draftDue} placeholder="YYYY-MM-DD" />
          {#if draftDue}
            {@const dueHint = formatDueDateDisplay(draftDue, new Date(), displayState.nlDueDates)}
            {#if dueHint}
              <span class="due-hint" class:due-today={dueHint.variant === "today"} class:due-tomorrow={dueHint.variant === "tomorrow"} class:due-overdue={dueHint.variant === "overdue"}>{dueHint.label}</span>
            {/if}
          {/if}
        </label>
        <label>
          Scheduled
          <input type="text" bind:value={draftScheduled} placeholder="YYYY-MM-DD" />
        </label>
      </div>
      <label>
        Notes
        <textarea bind:value={draftNotes} rows="3"></textarea>
      </label>
      {#if editError}
        <p class="edit-error" role="alert">{editError}</p>
      {/if}
      <div class="edit-actions">
        <button type="submit">Save</button>
        <button type="button" class="secondary" onclick={onCancel}>Cancel</button>
      </div>
    </form>
  {/if}
</dialog>

<style>
  .task-edit-dialog {
    padding: var(--space-lg);
    border: none;
    border-radius: var(--radius-lg);
    background: var(--color-surface-raised);
    color: var(--color-ink);
    box-shadow: var(--shadow-lg);
    width: min(28rem, calc(100vw - 2 * var(--space-lg)));
  }

  /* Higher specificity than `.task-edit-dialog` so a closed dialog stays
     `display: none` instead of the author-origin layout below overriding the
     UA stylesheet's `dialog:not([open]) { display: none }`. */
  .task-edit-dialog:not([open]) {
    display: none;
  }

  .task-edit-dialog::backdrop {
    background: oklch(20% 0.02 50 / 0.45);
  }

  .task-edit-dialog h2 {
    margin: 0 0 var(--space-md);
    font-size: var(--text-lg);
    font-weight: 600;
    letter-spacing: var(--tracking-tight);
  }

  .edit-form {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .edit-form label {
    display: flex;
    flex-direction: column;
    gap: var(--space-3xs);
    flex: 1;
    font-size: var(--text-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
  }

  .field-row {
    display: flex;
    gap: var(--space-sm);
  }

  .field-with-suggestions {
    position: relative;
  }

  .field-with-suggestions input {
    width: 100%;
  }

  .edit-form input,
  .edit-form select,
  .edit-form textarea {
    font: inherit;
    font-size: var(--text-sm);
    font-weight: 400;
    text-transform: none;
    letter-spacing: normal;
    padding: var(--space-2xs) var(--space-xs);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
    resize: vertical;
    transition:
      border-color var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo);
  }

  .edit-form input:focus-visible,
  .edit-form select:focus-visible,
  .edit-form textarea:focus-visible {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-soft);
    outline: none;
  }

  .due-hint {
    font-size: var(--text-xs);
    font-weight: 600;
    text-transform: none;
    letter-spacing: normal;
    color: var(--color-ink-muted);
  }

  .due-hint.due-today {
    color: var(--color-urgent);
  }

  .due-hint.due-overdue {
    color: var(--color-overdue);
  }

  .due-hint.due-tomorrow {
    color: var(--color-soon);
  }

  .edit-error {
    margin: 0;
    padding: var(--space-2xs) var(--space-xs);
    border-radius: var(--radius-sm);
    background: var(--color-danger-soft);
    color: var(--color-danger);
    font-size: var(--text-xs);
    font-weight: 600;
  }

  .edit-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-xs);
    margin-top: var(--space-3xs);
  }

  .edit-actions button {
    padding: var(--space-sm) var(--space-lg);
    border-radius: var(--radius-md);
    border: none;
    font-weight: 600;
    font-size: var(--text-base);
    cursor: pointer;
    box-shadow: var(--shadow-sm);
    background: var(--color-accent);
    color: var(--color-accent-ink);
    transition:
      background var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo),
      transform var(--duration-fast) var(--ease-out-expo);
  }

  .edit-actions button:hover {
    background: var(--color-accent-hover);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .edit-actions button.secondary {
    background: var(--color-surface);
    color: var(--color-ink);
    border: 1px solid var(--color-border);
    box-shadow: none;
  }

  .edit-actions button.secondary:hover {
    background: var(--color-canvas);
    box-shadow: none;
    transform: none;
  }
</style>
