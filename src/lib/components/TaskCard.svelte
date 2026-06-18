<script lang="ts">
  import { applyTagsSuggestion, filterSuggestions, splitTagsInput } from "$lib/autocomplete";
  import { isLightColor, NEON_CARD_CHROMA_BOOST, NEON_CARD_LIGHTNESS, neonCardColor } from "$lib/colorPresets";
  import { displayState } from "$lib/displaySettings.svelte";
  import { formatDueDateDisplay } from "$lib/dueDateDisplay";
  import { generalState } from "$lib/generalSettings.svelte";
  import {
    FALLBACK_PRIORITIES,
    priorityColor,
    priorityLabel,
    sortedPriorities,
  } from "$lib/priorities.svelte";
  import { resolveProjectColor } from "$lib/projectColor";
  import { projectsState } from "$lib/projects.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import { FALLBACK_STATUSES, statusColor } from "$lib/statuses.svelte";
  import { emptyToUndefined, formatTags, isValidOptionalDate, parseTags } from "$lib/taskFields";
  import { tagsState } from "$lib/tags.svelte";
  import type { Task } from "$lib/types";
  import Autocomplete from "./Autocomplete.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";

  interface Props {
    task: Task;
    onUpdate: (task: Task) => void;
    onDelete: (id: string) => void;
  }

  let { task, onUpdate, onDelete }: Props = $props();

  const priorities = $derived(settingsState.current?.priorities ?? FALLBACK_PRIORITIES);
  const projectColor = $derived(resolveProjectColor(task.project, projectsState.items));
  const isColorCoded = $derived(displayState.cardColorMode === "color_code");
  // A vivid, fixed-lightness rendering of the project's hue/chroma — not a
  // color-mix dilution toward the surface color — so the background always
  // stays bright enough for dark title text, regardless of the project's
  // own color, and reads as "neon"/saturated rather than washed-out pastel.
  const colorCodeBackground = $derived(
    isColorCoded ? neonCardColor(projectColor, NEON_CARD_LIGHTNESS, NEON_CARD_CHROMA_BOOST) : undefined,
  );
  // The urgency category (overdue/today/tomorrow/normal) doesn't depend on
  // the natural-language-phrasing setting, so `nlEnabled` is irrelevant here.
  const dueUrgency = $derived(formatDueDateDisplay(task.due, new Date(), false)?.variant);
  const showDueGlow = $derived(
    displayState.dueDateGlow && (dueUrgency === "overdue" || dueUrgency === "today"),
  );
  // The "Project tag" chip uses `projectColor` directly as text color, which
  // is illegible for very light project colors (e.g. a pale cream) on the
  // chip's near-white tint — fall back to the standard ink color in that case.
  const projectChipTextColor = $derived(isLightColor(projectColor) ? "var(--color-ink)" : projectColor);

  const isDone = $derived(task.status === (settingsState.current?.done_status ?? "done"));
  const cancelledStatus = $derived(settingsState.current?.cancelled_status);
  const isCancelled = $derived(!isDone && cancelledStatus !== undefined && task.status === cancelledStatus);
  const statuses = $derived(settingsState.current?.statuses ?? FALLBACK_STATUSES);
  const taskStatusColor = $derived(statusColor(statuses, task.status));

  let isEditing = $state(false);
  let draftTitle = $state("");
  let draftProject = $state("");
  let draftTags = $state("");
  let draftPriority = $state("medium");
  let draftDue = $state("");
  let draftScheduled = $state("");
  let draftNotes = $state("");
  let editError = $state("");
  let showDeleteConfirm = $state(false);

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
    if (generalState.confirmTaskDeletion) {
      showDeleteConfirm = true;
    } else {
      onDelete(task.id);
    }
  }

  function confirmDelete() {
    showDeleteConfirm = false;
    onDelete(task.id);
  }

  function cancelDelete() {
    showDeleteConfirm = false;
  }

  function handleTitleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      startEdit();
    }
  }
</script>

<li
  class="task"
  class:color-coded={isColorCoded}
  class:due-glow={showDueGlow}
  class:task-done={isDone}
  class:task-cancelled={isCancelled}
  style="--task-priority-color: {priorityColor(priorities, task.priority)}; --task-project-color: {projectColor}; --task-color-code-bg: {colorCodeBackground}; --task-status-color: {taskStatusColor}"
>
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
          {#each sortedPriorities(priorities) as level (level.id)}
            <option value={level.id}>{level.label}</option>
          {/each}
        </select>
      </label>
      <label>
        Due
        <input type="text" bind:value={draftDue} placeholder="YYYY-MM-DD" />
        {#if draftDue}
          {@const dueHint = formatDueDateDisplay(draftDue, new Date(), displayState.nlDueDates)}
          {#if dueHint}
            <span
              class="due-hint"
              class:due-today={dueHint.variant === "today"}
              class:due-tomorrow={dueHint.variant === "tomorrow"}
              class:due-overdue={dueHint.variant === "overdue"}>{dueHint.label}</span
            >
          {/if}
        {/if}
      </label>
      <label>
        Scheduled
        <input type="text" bind:value={draftScheduled} placeholder="YYYY-MM-DD" />
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
    {#if isCancelled}
      <span class="task-cancelled-x" aria-hidden="true">
        <svg viewBox="0 0 16 16" width="28" height="28">
          <path d="M3 3l10 10M13 3L3 13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
        </svg>
      </span>
    {/if}

    <div class="task-meta">
      {#if displayState.showPriorityChip}
        <span class="chip priority">
          <span class="priority-dot" aria-hidden="true"></span>
          {priorityLabel(priorities, task.priority)}
        </span>
      {/if}
      {#if task.project && !isColorCoded}
        <span class="chip project" style="--chip-color: {projectColor}; --chip-text-color: {projectChipTextColor}">
          {task.project}
        </span>
      {/if}
      {#if task.due}
        {@const dueDisplay = formatDueDateDisplay(task.due, new Date(), displayState.nlDueDates)}
        {#if dueDisplay}
          <span
            class="chip due"
            class:due-today={dueDisplay.variant === "today"}
            class:due-tomorrow={dueDisplay.variant === "tomorrow"}
            class:due-overdue={dueDisplay.variant === "overdue"}
          >
            {#if dueDisplay.variant !== "normal"}
              <span class="due-dot" aria-hidden="true"></span>
            {/if}
            {dueDisplay.label}
          </span>
        {/if}
      {/if}
      {#each task.tags as tag (tag)}
        <span class="chip tag">#{tag}</span>
      {/each}
    </div>
  {/if}

  <ConfirmDialog
    open={showDeleteConfirm}
    title="Delete task"
    message={`Delete "${task.title}"? This can't be undone.`}
    confirmLabel="Delete"
    onConfirm={confirmDelete}
    onCancel={cancelDelete}
  />
</li>

<style>
  .task {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--space-3xs);
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-md);
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-left: 5px solid var(--task-priority-color, transparent);
    box-shadow: var(--shadow-sm);
    transition:
      box-shadow var(--duration-fast) var(--ease-out-expo),
      transform var(--duration-fast) var(--ease-out-expo);
  }

  .task:hover {
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  /* "Due-date glow" setting: a soft red halo fading outward from the card,
     layered with the existing shadow rather than replacing it (multiple
     box-shadows stack), so hover's lift/shadow still works normally.
     Uses a dedicated, more saturated red (not --color-urgent, which is
     tuned for badge fills/text) at higher opacity than a first pass used —
     diluted further, the warm ivory canvas's own undertone dominated the
     outer ring and it read as peachy/orange instead of red. */
  .task.due-glow {
    box-shadow:
      var(--shadow-sm),
      0 0 0 1px oklch(55% 0.22 18 / 0.85),
      0 0 12px 4px oklch(55% 0.22 18 / 0.55),
      0 0 22px 8px oklch(55% 0.22 18 / 0.3);
  }

  .task.due-glow:hover {
    box-shadow:
      var(--shadow-md),
      0 0 0 1px oklch(55% 0.22 18 / 0.85),
      0 0 12px 4px oklch(55% 0.22 18 / 0.55),
      0 0 22px 8px oklch(55% 0.22 18 / 0.3);
  }

  /* Done: tint toward the configured done status's color and strike
     through the title — a glance should make "this is finished"
     unmistakable. Cancelled: tint toward the cancelled status's color and
     overlay an X (see .task-cancelled-x) instead of a strikethrough, so the
     two finished states read as visually distinct from each other, not
     just both "muted." Layered on top of color-coded mode's background
     (not mutually exclusive with it) by mixing rather than replacing. */
  .task.task-done,
  .task.task-cancelled {
    background: color-mix(in oklch, var(--task-status-color) 14%, var(--task-color-code-bg, var(--color-surface-raised)));
    opacity: 0.8;
  }

  .task.task-done .task-title {
    text-decoration: line-through;
    text-decoration-thickness: 1.5px;
  }

  .task-cancelled-x {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    color: var(--task-status-color);
    opacity: 0.55;
    pointer-events: none;
  }

  /* "Color code" card display mode: the whole card is tinted by the
     project's color (rendered via `neonCardColor` at a fixed, bright
     lightness — see TaskCard.svelte's script) instead of showing a project
     chip. Title text is always dark ink: the fixed lightness target makes
     that safe for every project color, with no per-color contrast check
     needed. Other chips get an explicit opaque background + border here so
     they stay clearly legible against a more saturated card than the
     default neutral surface. */
  .task.color-coded {
    background: var(--task-color-code-bg);
    /* Only the non-priority sides — `border-color` is a shorthand that would
       otherwise also overwrite `border-left`, which is how the priority
       accent stripe is rendered (see `.task`'s base rule below). That stripe
       must stay visible in color-coded mode, especially since it's the
       *only* priority indicator left when "Show priority labels" is off. */
    border-top-color: color-mix(in oklch, var(--task-project-color) 65%, var(--color-border));
    border-right-color: color-mix(in oklch, var(--task-project-color) 65%, var(--color-border));
    border-bottom-color: color-mix(in oklch, var(--task-project-color) 65%, var(--color-border));
  }

  .task.color-coded .task-title {
    color: var(--color-ink);
  }

  /* Tag/priority chips use the generic muted-gray chip styling, which can
     blend into a saturated color-coded background — give them an explicit
     opaque background here. The due chip (overdue/today/tomorrow/plain) and
     project chip are excluded: the due chip's solid/soft red/amber fills
     already have strong, intentional contrast, and the project chip is
     hidden entirely in this mode (see the template). */
  .task.color-coded .chip.tag,
  .task.color-coded .chip.priority {
    background: var(--color-surface-raised);
    border-color: var(--color-border-strong);
    color: var(--color-ink);
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
    font-size: var(--text-sm);
    font-weight: 600;
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
    gap: var(--space-4xs);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-3xs);
    font-size: var(--text-xs);
    line-height: var(--leading-tight);
    padding: var(--space-4xs) var(--space-2xs);
    border-radius: var(--radius-sm);
    background: var(--color-canvas);
    border: 1px solid var(--color-border);
    color: var(--color-ink-muted);
  }

  /* Tags are the one attribute that keeps the pill shape — every other
     attribute chip (priority/project/due) is rectangular, per design. */
  .chip.tag {
    border-radius: var(--radius-pill);
  }

  .chip.project {
    background: color-mix(in oklch, var(--chip-color, var(--color-accent)) 16%, var(--color-surface-raised));
    border-color: color-mix(in oklch, var(--chip-color, var(--color-accent)) 45%, transparent);
    color: var(--chip-text-color, var(--chip-color, var(--color-accent)));
    font-weight: 600;
  }

  .chip.due {
    font-variant-numeric: tabular-nums;
  }

  /* "Overdue" is the most urgent signal a card can show, so it gets a
     solid fill rather than the soft-tint treatment every other chip uses —
     it should visually outrank the priority/project/tag chips next to it. */
  .chip.due-overdue {
    background: var(--color-urgent);
    border-color: var(--color-urgent);
    color: var(--color-urgent-ink);
    font-weight: 700;
  }

  .chip.due-today {
    background: var(--color-urgent-soft);
    border-color: color-mix(in oklch, var(--color-urgent) 40%, transparent);
    color: var(--color-urgent);
    font-weight: 700;
  }

  .chip.due-tomorrow {
    background: var(--color-soon-soft);
    border-color: color-mix(in oklch, var(--color-soon) 40%, transparent);
    color: var(--color-soon);
    font-weight: 700;
  }

  .due-dot {
    width: 0.4rem;
    height: 0.4rem;
    border-radius: var(--radius-pill);
    background: currentColor;
    flex-shrink: 0;
  }

  .chip.due-overdue .due-dot {
    background: var(--color-urgent-ink);
  }

  /* Plain sentence case, matching the due chip's style — uppercase + tracked
     letter-spacing (the prior treatment) reads visually larger than the
     other chips at the same font-size, even though the size itself matches. */
  .chip.priority {
    font-weight: 600;
  }

  .priority-dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: var(--radius-pill);
    background: var(--task-priority-color, var(--color-border-strong));
    flex-shrink: 0;
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

  .due-hint {
    font-size: var(--text-xs);
    font-weight: 600;
    text-transform: none;
    letter-spacing: normal;
    color: var(--color-ink-muted);
  }

  .due-hint.due-today,
  .due-hint.due-overdue {
    color: var(--color-urgent);
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
    background: var(--color-danger);
  }

  .edit-actions button.danger:hover {
    background: var(--color-danger-hover);
  }
</style>
