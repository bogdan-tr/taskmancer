<script lang="ts">
  import { applyTagsSuggestion, filterSuggestions, splitTagsInput } from "$lib/autocomplete";
  import { projectsState } from "$lib/projects.svelte";
  import { emptyToUndefined, formatTags, isValidOptionalDate, parseTags } from "$lib/taskFields";
  import { tagsState } from "$lib/tags.svelte";
  import { PRIORITIES, PRIORITY_LABELS, type Priority, type Task } from "$lib/types";
  import Autocomplete from "./Autocomplete.svelte";

  interface Props {
    task: Task;
    onUpdate: (task: Task) => void;
    onDelete: (id: string) => void;
  }

  let { task, onUpdate, onDelete }: Props = $props();

  let isEditing = $state(false);
  let draftTitle = $state("");
  let draftProject = $state("");
  let draftTags = $state("");
  let draftPriority = $state<Priority>("medium");
  let draftDue = $state("");
  let draftScheduled = $state("");
  let draftNotes = $state("");
  let editError = $state("");

  let projectSuggestions: string[] = $state([]);
  let projectSuggestionIndex = $state(0);
  let tagSuggestions: string[] = $state([]);
  let tagSuggestionIndex = $state(0);

  function startEdit() {
    draftTitle = task.title;
    draftProject = task.project ?? "";
    draftTags = formatTags(task.tags);
    draftPriority = task.priority;
    draftDue = task.due ?? "";
    draftScheduled = task.scheduled ?? "";
    draftNotes = task.notes;
    editError = "";
    projectSuggestions = [];
    tagSuggestions = [];
    isEditing = true;
  }

  function cancelEdit() {
    isEditing = false;
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

    if (!isValidOptionalDate(draftDue) || !isValidOptionalDate(draftScheduled)) {
      editError = "Due and scheduled dates must be in YYYY-MM-DD format";
      return;
    }

    onUpdate({
      ...task,
      title: draftTitle,
      project: emptyToUndefined(draftProject),
      tags: parseTags(draftTags),
      priority: draftPriority,
      due: emptyToUndefined(draftDue),
      scheduled: emptyToUndefined(draftScheduled),
      notes: draftNotes,
    });
    isEditing = false;
  }

  function handleDelete() {
    if (confirm(`Delete "${task.title}"?`)) {
      onDelete(task.id);
    }
  }

  function handleTitleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      startEdit();
    }
  }
</script>

<li class="task priority-{task.priority}">
  {#if isEditing}
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
            aria-controls="draft-project-suggestions-{task.id}"
            aria-autocomplete="list"
            aria-activedescendant={projectSuggestions.length > 0
              ? `draft-project-suggestions-${task.id}-option-${projectSuggestionIndex}`
              : undefined}
            oninput={updateProjectSuggestions}
            onkeydown={handleProjectKeydown}
            onblur={() => (projectSuggestions = [])}
          />
          <Autocomplete
            id="draft-project-suggestions-{task.id}"
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
            aria-controls="draft-tags-suggestions-{task.id}"
            aria-autocomplete="list"
            aria-activedescendant={tagSuggestions.length > 0
              ? `draft-tags-suggestions-${task.id}-option-${tagSuggestionIndex}`
              : undefined}
            oninput={updateTagSuggestions}
            onkeydown={handleTagsKeydown}
            onblur={() => (tagSuggestions = [])}
          />
          <Autocomplete
            id="draft-tags-suggestions-{task.id}"
            items={tagSuggestions}
            activeIndex={tagSuggestionIndex}
            onSelect={selectTagSuggestion}
            onHover={(index) => (tagSuggestionIndex = index)}
            prefix="#"
          />
        </div>
      </label>
      <label>
        Priority
        <select bind:value={draftPriority}>
          {#each PRIORITIES as priority (priority)}
            <option value={priority}>{PRIORITY_LABELS[priority]}</option>
          {/each}
        </select>
      </label>
      <label>
        Due
        <input
          type="text"
          bind:value={draftDue}
          placeholder="YYYY-MM-DD"
          pattern="\d{4}-\d{2}-\d{2}"
          title="Format: YYYY-MM-DD"
        />
      </label>
      <label>
        Scheduled
        <input
          type="text"
          bind:value={draftScheduled}
          placeholder="YYYY-MM-DD"
          pattern="\d{4}-\d{2}-\d{2}"
          title="Format: YYYY-MM-DD"
        />
      </label>
      <label>
        Notes
        <textarea bind:value={draftNotes} rows="3"></textarea>
      </label>
      {#if editError}
        <p class="edit-error" role="alert">{editError}</p>
      {/if}
      <div class="edit-actions">
        <button type="submit">Save</button>
        <button type="button" onclick={cancelEdit}>Cancel</button>
        <button type="button" class="danger" onclick={handleDelete}>Delete</button>
      </div>
    </form>
  {:else}
    <div
      class="task-title"
      role="button"
      tabindex="0"
      onclick={startEdit}
      onkeydown={handleTitleKeydown}
    >
      {task.title}
    </div>

    {#if task.project || task.tags.length > 0 || task.due}
      <div class="task-meta">
        {#if task.project}
          <span class="chip project">{task.project}</span>
        {/if}
        {#each task.tags as tag (tag)}
          <span class="chip tag">#{tag}</span>
        {/each}
        {#if task.due}
          <span class="chip due">Due {task.due}</span>
        {/if}
      </div>
    {/if}
  {/if}
</li>

<style>
  .task {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
    padding: var(--space-sm);
    border-radius: var(--radius-md);
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-left: 3px solid transparent;
    box-shadow: var(--shadow-sm);
    transition:
      box-shadow var(--duration-fast) var(--ease-out-expo),
      transform var(--duration-fast) var(--ease-out-expo);
  }

  .task:hover {
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .task.priority-high {
    border-left-color: var(--color-priority-high);
  }

  .task.priority-medium {
    border-left-color: var(--color-priority-medium);
  }

  .task.priority-low {
    border-left-color: var(--color-priority-low);
  }

  /* Drag placeholder clone (see svelte-dnd-action SHADOW_ELEMENT_ATTRIBUTE_NAME):
     render as an empty "drop slot" outline instead of a duplicate card.
     :global() is required because the attribute is set via direct DOM
     manipulation at runtime, not statically present in this template. */
  .task:global([data-is-dnd-shadow-item-internal]) {
    background: transparent;
    border-style: dashed;
    border-color: var(--color-border-strong);
    box-shadow: none;
  }

  .task:global([data-is-dnd-shadow-item-internal]) > * {
    visibility: hidden;
  }

  .task-title {
    cursor: pointer;
    font-size: var(--text-base);
    line-height: var(--leading-tight);
    word-break: break-word;
  }

  .task-title:hover {
    color: var(--color-accent);
  }

  .task-title:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
    border-radius: var(--radius-sm);
  }

  .task-meta {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2xs);
  }

  .chip {
    font-size: var(--text-xs);
    line-height: var(--leading-tight);
    padding: var(--space-3xs) var(--space-xs);
    border-radius: var(--radius-pill);
    background: var(--color-canvas);
    border: 1px solid var(--color-border);
    color: var(--color-ink-muted);
  }

  .chip.project {
    background: var(--color-accent-soft);
    border-color: transparent;
    color: var(--color-accent);
    font-weight: 600;
  }

  .chip.due {
    font-variant-numeric: tabular-nums;
  }

  .edit-form {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .edit-form label {
    display: flex;
    flex-direction: column;
    gap: var(--space-3xs);
    font-size: var(--text-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
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

  .edit-error {
    margin: 0;
    padding: var(--space-2xs) var(--space-xs);
    border-radius: var(--radius-sm);
    background: var(--color-danger-soft);
    color: var(--color-priority-high);
    font-size: var(--text-xs);
    font-weight: 600;
  }

  .edit-actions {
    display: flex;
    gap: var(--space-2xs);
    margin-top: var(--space-3xs);
  }

  .edit-actions button {
    flex: 1;
    padding: var(--space-2xs) var(--space-xs);
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: var(--color-accent);
    color: var(--color-accent-ink);
    font-weight: 600;
    font-size: var(--text-xs);
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out-expo);
  }

  .edit-actions button:hover {
    background: var(--color-accent-hover);
  }

  .edit-actions button[type="button"]:not(.danger) {
    background: var(--color-surface);
    border-color: var(--color-border);
    color: var(--color-ink);
  }

  .edit-actions button[type="button"]:not(.danger):hover {
    background: var(--color-canvas);
  }

  .edit-actions button.danger {
    background: var(--color-priority-high);
  }

  .edit-actions button.danger:hover {
    background: var(--color-priority-high-hover);
  }
</style>
