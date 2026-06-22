<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { applyTagsSuggestion, filterSuggestions, splitTagsInput } from "$lib/autocomplete";
  import {
    isLightColor,
    legibleInkColor,
    NEON_CARD_CHROMA_BOOST,
    NEON_CARD_LIGHTNESS,
    neonCardColor,
  } from "$lib/colorPresets";
  import { displayState } from "$lib/displaySettings.svelte";
  import { formatDueDateDisplay } from "$lib/dueDateDisplay";
  import {
    formatMinutes,
    hoursAndMinutesFromMinutes,
    minutesFromHoursAndMinutes,
    normalizeHoursMinutes,
  } from "$lib/estimatedTime";
  import { generalState } from "$lib/generalSettings.svelte";
  import {
    FALLBACK_PRIORITIES,
    priorityColor,
    priorityLabel,
    priorityRank,
    sortedPriorities,
  } from "$lib/priorities.svelte";
  import { resolveCardLightness, resolveInkMode, resolveProjectColor } from "$lib/projectColor";
  import { projectsState } from "$lib/projects.svelte";
  import type { SeriesEditScope } from "$lib/recurrence";
  import { settingsState } from "$lib/settings.svelte";
  import { FALLBACK_STATUSES, statusColor, statusLabel } from "$lib/statuses.svelte";
  import { isSubtask, relevantSubtasksOf, subtaskProgress, subtasksOf } from "$lib/subtasks";
  import {
    emptyToUndefined,
    formatTags,
    isValidOptionalDate,
    parseTags,
    seriesSharedFieldsChanged,
  } from "$lib/taskFields";
  import { tagsState } from "$lib/tags.svelte";
  import type { Task } from "$lib/types";
  import { formatDateISO } from "$lib/weekRange";
  import Autocomplete from "./Autocomplete.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import DatePickerPopover from "./DatePickerPopover.svelte";
  import SeriesScopeDialog from "./SeriesScopeDialog.svelte";
  import StatusPickerDialog from "./StatusPickerDialog.svelte";

  interface Props {
    task: Task;
    onUpdate: (task: Task, scope?: SeriesEditScope) => void;
    onDelete: (id: string, scope?: SeriesEditScope) => void;
    onRemoveRecurrence: (id: string) => void;
    /**
     * The global task list (`tasksState.items`, not the current board's
     * `visibleTasks`) — for finding this task's own subtasks and for
     * hiding the "Create Subtask" button on a task that's itself already
     * a subtask. Must be the global list: when this card is rendered on
     * its *own* subtask container's board, the board-scoped list would
     * never include the owning parent task (it lives in a different
     * project entirely), making every subtask-relationship check here
     * silently wrong.
     */
    allTasks?: Task[];
    /** Opens the Add Task modal pre-filled to create a subtask of `task`. */
    onCreateSubtask?: (task: Task) => void;
  }

  let { task, onUpdate, onDelete, onRemoveRecurrence, allTasks = [], onCreateSubtask }: Props = $props();

  const priorities = $derived(settingsState.current?.priorities ?? FALLBACK_PRIORITIES);
  /** `true` when `task` is itself a subtask — hides the "Create Subtask" button (one-level-deep rule). */
  const isThisTaskSubtask = $derived(isSubtask(task, allTasks));
  /** "Today", recomputed per render rather than tracked live — matches `KanbanBoard`'s own one-shot `formatDateISO(new Date())` calls. Used to collapse a recurring subtask's many pre-generated occurrences down to the one currently relevant (see `relevantSubtasksOf`). */
  const today = $derived(formatDateISO(new Date()));
  /** Nested rows render priority-sorted (highest first), per the original feature request. */
  const taskSubtasks = $derived(
    [...relevantSubtasksOf(task, allTasks, today)].sort(
      (a, b) => priorityRank(priorities, a.priority) - priorityRank(priorities, b.priority),
    ),
  );
  /** Every generated occurrence of every subtask, unlike `taskSubtasks` above — for the delete-confirmation count, since deleting `task` cascades to every one of them, not just the currently relevant occurrence of each. */
  const taskSubtasksFullCount = $derived(subtasksOf(task, allTasks).length);
  const taskSubtaskProgress = $derived(
    subtaskProgress(
      task,
      allTasks,
      settingsState.current?.done_status,
      settingsState.current?.cancelled_status,
      today,
    ),
  );

  /** The subtask whose status picker is currently open, if any. */
  let statusPickerSubtask: Task | undefined = $state(undefined);

  function handleSubtaskStatusClick(subtask: Task) {
    statusPickerSubtask = subtask;
  }

  function selectSubtaskStatus(statusId: string) {
    if (statusPickerSubtask) {
      onUpdate({ ...statusPickerSubtask, status: statusId });
    }
    statusPickerSubtask = undefined;
  }

  function closeStatusPicker() {
    statusPickerSubtask = undefined;
  }
  const projectColor = $derived(resolveProjectColor(task.project_id, projectsState.items));
  const projectName = $derived(projectsState.items.find((p) => p.id === task.project_id)?.name);
  /** The project currently being viewed (from the URL), or `undefined` on the non-project-scoped "All Tasks" route. */
  let viewedProjectId = $derived(page.params.id);
  /** `true` when this card is shown via a parent project's rolled-up view — its own project differs from the one currently being viewed. */
  let isRolledUp = $derived(
    viewedProjectId !== undefined && task.project_id !== undefined && task.project_id !== viewedProjectId,
  );
  const isColorCoded = $derived(displayState.cardColorMode === "color_code");
  // The project's own override (Project.board.card_lightness) takes
  // precedence over the global default (Settings.card_lightness); falls
  // back further to the original hardcoded constant while settings are
  // still loading.
  const cardLightness = $derived(
    resolveCardLightness(
      task.project_id,
      projectsState.items,
      settingsState.current?.card_lightness ?? NEON_CARD_LIGHTNESS,
    ),
  );
  // A vivid rendering of the project's hue/chroma at the resolved lightness
  // above — not a color-mix dilution toward the surface color — so the
  // background reads as "neon"/saturated rather than washed-out pastel.
  // Text color adapts to the resolved lightness (see `colorCodeTextColor`)
  // since that lightness is now user-configurable across its full range,
  // not fixed at a value chosen to always be safe for one fixed ink color.
  const colorCodeBackground = $derived(
    isColorCoded ? neonCardColor(projectColor, cardLightness, NEON_CARD_CHROMA_BOOST) : undefined,
  );
  // The project's own override (Project.board.ink_mode) takes precedence
  // over the global default (Settings.ink_mode); falls back to "auto"
  // (contrast-computed, the original behavior) while settings are loading.
  const inkMode = $derived(
    resolveInkMode(task.project_id, projectsState.items, settingsState.current?.ink_mode ?? "auto"),
  );
  const colorCodeTextColor = $derived(
    colorCodeBackground ? legibleInkColor(colorCodeBackground, inkMode) : undefined,
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
  let draftEstimatedHours: number | undefined = $state(undefined);
  let draftEstimatedMinutes: number | undefined = $state(undefined);
  let draftNotes = $state("");
  let editError = $state("");
  let showDeleteConfirm = $state(false);
  let showRemoveRecurrenceConfirm = $state(false);

  /** The action awaiting a "just this task vs. this and future" choice, and the built task to save (only relevant for "save"). */
  let pendingScopeKind: "save" | "delete" | undefined = $state(undefined);
  let pendingScopeTask: Task | undefined = $state(undefined);
  let scopeDialogOpen = $derived(pendingScopeKind !== undefined);
  let scopeDialogTitle = $derived(
    pendingScopeKind === "delete" ? "Delete recurring task" : "Save changes to recurring task",
  );
  const scopeDialogMessage =
    "This task repeats. Apply this to just this task, or to this task and every future one?";

  let projectSuggestions: string[] = $state([]);
  let projectSuggestionIndex = $state(0);
  let tagSuggestions: string[] = $state([]);
  let tagSuggestionIndex = $state(0);

  function startEdit() {
    draftTitle = task.title;
    draftProject = projectsState.items.find((p) => p.id === task.project_id)?.name ?? "";
    draftTags = formatTags(task.tags);
    draftPriority = task.priority;
    draftDue = task.due ?? "";
    draftScheduled = task.scheduled ?? "";
    const resolvedEstimate =
      task.estimated_minutes !== undefined ? hoursAndMinutesFromMinutes(task.estimated_minutes) : undefined;
    draftEstimatedHours = resolvedEstimate?.hours;
    draftEstimatedMinutes = resolvedEstimate?.minutes;
    draftNotes = task.notes;
    editError = "";
    projectSuggestions = [];
    tagSuggestions = [];
    isEditing = true;
  }

  /** Rolls minutes >= 60 over into hours, e.g. typing 90 into "mins" reads back as 1h 30m. */
  function normalizeEstimateDraft() {
    if (draftEstimatedHours === undefined && draftEstimatedMinutes === undefined) return;
    const normalized = normalizeHoursMinutes(draftEstimatedHours ?? 0, draftEstimatedMinutes ?? 0);
    draftEstimatedHours = normalized.hours;
    draftEstimatedMinutes = normalized.minutes;
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

    const estimatedMinutes =
      draftEstimatedHours === undefined && draftEstimatedMinutes === undefined
        ? undefined
        : minutesFromHoursAndMinutes(draftEstimatedHours ?? 0, draftEstimatedMinutes ?? 0);

    const updated: Task = {
      ...task,
      title: draftTitle,
      project_id: projectsState.items.find((p) => p.name.toLowerCase() === draftProject.trim().toLowerCase())
        ?.id,
      tags: parseTags(draftTags),
      priority: draftPriority,
      due: emptyToUndefined(draftDue),
      scheduled: emptyToUndefined(draftScheduled),
      estimated_minutes: estimatedMinutes,
      notes: draftNotes,
    };

    if (task.series_id !== undefined && seriesSharedFieldsChanged(task, updated)) {
      pendingScopeTask = updated;
      pendingScopeKind = "save";
      return;
    }

    onUpdate(updated);
    isEditing = false;
  }

  function handleDelete() {
    if (task.series_id !== undefined) {
      pendingScopeKind = "delete";
    } else if (generalState.confirmTaskDeletion) {
      showDeleteConfirm = true;
    } else {
      onDelete(task.id);
    }
  }

  function confirmDelete() {
    showDeleteConfirm = false;
    onDelete(task.id);
  }

  function handleScopeThis() {
    if (pendingScopeKind === "save" && pendingScopeTask) {
      onUpdate(pendingScopeTask, "this");
      isEditing = false;
    } else if (pendingScopeKind === "delete") {
      onDelete(task.id, "this");
    }
    pendingScopeKind = undefined;
    pendingScopeTask = undefined;
  }

  function handleScopeFuture() {
    if (pendingScopeKind === "save" && pendingScopeTask) {
      onUpdate(pendingScopeTask, "future");
      isEditing = false;
    } else if (pendingScopeKind === "delete") {
      onDelete(task.id, "future");
    }
    pendingScopeKind = undefined;
    pendingScopeTask = undefined;
  }

  function cancelScopeDialog() {
    pendingScopeKind = undefined;
    pendingScopeTask = undefined;
  }

  function handleRemoveRecurrenceClick() {
    showRemoveRecurrenceConfirm = true;
  }

  function confirmRemoveRecurrence() {
    showRemoveRecurrenceConfirm = false;
    onRemoveRecurrence(task.id);
  }

  function cancelRemoveRecurrence() {
    showRemoveRecurrenceConfirm = false;
  }

  function cancelDelete() {
    showDeleteConfirm = false;
  }

  /**
   * Clicking the title of a task that owns a subtask container navigates to
   * that container's own board instead of opening inline edit — per the
   * "subtasks are like tasks in a subproject, where the parent task is the
   * subproject" framing, the title click is this task's "open the
   * subproject" affordance. Editing the parent task's own fields (and
   * reaching the Create Subtask button) moves to the dedicated edit icon
   * below for exactly this case; a task with no container keeps today's
   * plain click-to-edit behavior unchanged.
   */
  function handleTitleClick() {
    if (task.subtask_project_id) {
      void goto(`/projects/${task.subtask_project_id}`);
    } else {
      startEdit();
    }
  }

  function handleTitleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      handleTitleClick();
    }
  }
</script>

<li
  class="task"
  class:color-coded={isColorCoded}
  class:due-glow={showDueGlow}
  class:task-done={isDone}
  class:task-cancelled={isCancelled}
  style="--task-priority-color: {priorityColor(priorities, task.priority)}; --task-project-color: {projectColor}; --task-color-code-bg: {colorCodeBackground}; --task-color-code-text: {colorCodeTextColor}; --task-status-color: {taskStatusColor}"
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
        Priority
        <select bind:value={draftPriority}>
          {#each sortedPriorities(priorities) as level (level.id)}
            <option value={level.id}>{level.label}</option>
          {/each}
        </select>
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
        Scheduled
        <span class="date-input-row">
          <input type="text" bind:value={draftScheduled} placeholder="YYYY-MM-DD" />
          <DatePickerPopover
            selected={draftScheduled || undefined}
            triggerLabel="Pick scheduled date"
            clearLabel="Clear"
            onSelect={(iso) => (draftScheduled = iso)}
            onClear={() => (draftScheduled = "")}
          />
        </span>
      </label>
      <label>
        Due
        <span class="date-input-row">
          <input type="text" bind:value={draftDue} placeholder="YYYY-MM-DD" />
          <DatePickerPopover
            selected={draftDue || undefined}
            triggerLabel="Pick due date"
            clearLabel="Never"
            onSelect={(iso) => (draftDue = iso)}
            onClear={() => (draftDue = "")}
          />
        </span>
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
        Estimated time
        <span class="estimate-inputs">
          <input
            type="number"
            min="0"
            step="1"
            placeholder="0"
            bind:value={draftEstimatedHours}
            onblur={normalizeEstimateDraft}
            aria-label="Estimated hours"
          />
          hrs
          <input
            type="number"
            min="0"
            step="1"
            placeholder="0"
            bind:value={draftEstimatedMinutes}
            onblur={normalizeEstimateDraft}
            aria-label="Estimated minutes"
          />
          mins
        </span>
      </label>
      <label>
        Notes
        <textarea bind:value={draftNotes} rows="3"></textarea>
      </label>
      {#if task.series_id !== undefined && !isThisTaskSubtask}
        <button type="button" class="remove-recurrence-link" onclick={handleRemoveRecurrenceClick}>
          Remove recurrence
        </button>
      {/if}
      {#if task.series_id !== undefined && isThisTaskSubtask}
        <p class="recurrence-locked-note">Recurs with its parent task — can't be changed here.</p>
      {/if}
      {#if !isThisTaskSubtask && onCreateSubtask}
        <button type="button" class="create-subtask-button" onclick={() => onCreateSubtask(task)}>
          + Create subtask
        </button>
      {/if}
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
    <div class="task-title-row">
      <div
        class="task-title"
        role="button"
        tabindex="0"
        onclick={handleTitleClick}
        onkeydown={handleTitleKeydown}
      >
        {task.title}
      </div>
      {#if task.subtask_project_id}
        <button type="button" class="edit-icon-button" onclick={startEdit} aria-label="Edit task" title="Edit task">
          <svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M11 2l3 3-7.5 7.5L3 13l.5-3.5L11 2z" />
          </svg>
        </button>
      {/if}
    </div>
    {#if isDone}
      <span class="task-done-check" aria-hidden="true">
        <svg viewBox="0 0 16 16" width="30" height="30">
          <path
            d="M3 8.5l3.5 3.5L13 4.5"
            fill="none"
            stroke="currentColor"
            stroke-width="2.25"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </span>
    {:else if isCancelled}
      <span class="task-cancelled-x" aria-hidden="true">
        <svg viewBox="0 0 16 16" width="28" height="28">
          <path d="M3 3l10 10M13 3L3 13" stroke="currentColor" stroke-width="2.25" stroke-linecap="round" />
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
      {#if projectName && !isColorCoded}
        <span class="chip project" style="--chip-color: {projectColor}; --chip-text-color: {projectChipTextColor}">
          {projectName}
        </span>
      {/if}
      {#if isRolledUp && projectName}
        <span class="origin-badge" title={`From ${projectName}`}>
          <span class="origin-dot" style="background: {projectColor}" aria-hidden="true"></span>
          {projectName}
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
      {#if task.estimated_minutes !== undefined}
        <span class="chip estimated" title="Estimated time">{formatMinutes(task.estimated_minutes)}</span>
      {/if}
      {#if task.tracked_minutes > 0}
        <span class="chip tracked" title="Tracked time">{formatMinutes(task.tracked_minutes)} tracked</span>
      {/if}
      {#each task.tags as tag (tag)}
        <span class="chip tag">#{tag}</span>
      {/each}
      {#if taskSubtaskProgress.total > 0}
        <span class="chip subtask-progress" title="Subtasks done">
          {taskSubtaskProgress.done}/{taskSubtaskProgress.total}
        </span>
      {/if}
    </div>

    {#if displayState.showSubtasks && taskSubtasks.length > 0}
      <ul class="subtask-list">
        {#each taskSubtasks as subtask (subtask.id)}
          <li class="subtask-row">
            <span
              class="subtask-priority-dot"
              style="--priority-color: {priorityColor(priorities, subtask.priority)}"
              title={priorityLabel(priorities, subtask.priority)}
              aria-hidden="true"
            ></span>
            <span class="subtask-title">{subtask.title}</span>
            <button
              type="button"
              class="subtask-status-dot"
              style="--status-color: {statusColor(statuses, subtask.status)}"
              title={statusLabel(statuses, subtask.status)}
              aria-label={`Change status of subtask "${subtask.title}" (currently ${statusLabel(statuses, subtask.status)})`}
              onclick={() => handleSubtaskStatusClick(subtask)}
            ></button>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}

  <ConfirmDialog
    open={showDeleteConfirm}
    title="Delete task"
    message={taskSubtasksFullCount > 0
      ? `Delete "${task.title}"? This will also delete ${taskSubtasksFullCount} subtask${taskSubtasksFullCount === 1 ? "" : "s"}. This can't be undone.`
      : `Delete "${task.title}"? This can't be undone.`}
    confirmLabel="Delete"
    onConfirm={confirmDelete}
    onCancel={cancelDelete}
  />

  <ConfirmDialog
    open={showRemoveRecurrenceConfirm}
    title="Remove recurrence?"
    message="This stops the series from generating any more occurrences. Tasks already created (past and future) are left as they are."
    confirmLabel="Remove recurrence"
    onConfirm={confirmRemoveRecurrence}
    onCancel={cancelRemoveRecurrence}
  />

  <SeriesScopeDialog
    open={scopeDialogOpen}
    title={scopeDialogTitle}
    message={scopeDialogMessage}
    onThis={handleScopeThis}
    onFuture={handleScopeFuture}
    onCancel={cancelScopeDialog}
  />

  <StatusPickerDialog
    open={statusPickerSubtask !== undefined}
    taskTitle={statusPickerSubtask?.title ?? ""}
    {statuses}
    currentStatusId={statusPickerSubtask?.status ?? ""}
    onSelect={selectSubtaskStatus}
    onCancel={closeStatusPicker}
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

  .task.task-done .task-title {
    text-decoration: line-through;
    text-decoration-thickness: 1.5px;
  }

  .task-done-check,
  .task-cancelled-x {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    color: var(--task-status-color);
    opacity: 0.85;
    pointer-events: none;
  }

  /* "Color code" card display mode: the whole card is tinted by the
     project's color (rendered via `neonCardColor` at a configurable
     lightness — see TaskCard.svelte's script) instead of showing a project
     chip. Title text color adapts to that lightness (`legibleInkColor`,
     also computed in the script) so it stays legible across the lightness
     slider's full range, not just at one fixed value. Other chips get an
     explicit opaque background + border here so they stay clearly legible
     against a more saturated card than the default neutral surface. */
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
    color: var(--task-color-code-text);
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

  /* Done/cancelled: a small tint of the status color mixed into
     --color-finished-surface (a dedicated near-neutral gray, clearly darker
     than any normal card background in every theme — see tokens.css) so the
     card reads as "mostly gray with a faint hint of its original color,"
     not a normal-looking card. Comes after `.task.color-coded` in source
     order (same specificity — last one wins) so a finished task always
     looks finished, even when color-code mode would otherwise paint it a
     vivid project color. No opacity reduction on the whole card — that
     would equally dim the check/x overlay below, which should stand out,
     not blend in. */
  .task.task-done,
  .task.task-cancelled {
    background: color-mix(in oklch, var(--task-status-color) 16%, var(--color-finished-surface));
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

  .task-title-row {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2xs);
  }

  .task-title {
    flex: 1;
    min-width: 0;
    cursor: pointer;
    font-size: var(--text-sm);
    font-weight: 600;
    line-height: var(--leading-tight);
    word-break: break-word;
  }

  .edit-icon-button {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 1.25rem;
    height: 1.25rem;
    margin-top: 1px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-ink-faint);
    cursor: pointer;
    transition: color var(--duration-fast) var(--ease-out-expo);
  }

  .edit-icon-button:hover {
    color: var(--color-accent);
  }

  .edit-icon-button:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
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

  .origin-badge {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2xs);
    font-size: var(--text-xs);
    color: var(--color-ink-muted);
  }

  .origin-dot {
    width: 0.5rem;
    height: 0.5rem;
    flex-shrink: 0;
    border-radius: var(--radius-sm);
  }

  .chip.due {
    font-variant-numeric: tabular-nums;
  }

  /* Dark red, distinct from "due today" (--color-urgent) — same soft-tint
     treatment every other chip uses (tinted background, colored text), not
     a solid fill, for visual consistency with the priority/project/tag
     chips next to it. */
  .chip.due-overdue {
    background: var(--color-overdue-soft);
    border-color: color-mix(in oklch, var(--color-overdue) 40%, transparent);
    color: var(--color-overdue);
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

  .chip.subtask-progress {
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .subtask-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-3xs);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .subtask-row {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    padding: var(--space-3xs) var(--space-2xs);
    border-radius: var(--radius-sm);
    background: var(--color-canvas);
    font-size: var(--text-xs);
  }

  .subtask-priority-dot {
    width: 0.4rem;
    height: 0.4rem;
    border-radius: var(--radius-pill);
    background: var(--priority-color, var(--color-border-strong));
    flex-shrink: 0;
  }

  .subtask-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-ink);
  }

  .subtask-status-dot {
    width: 0.65rem;
    height: 0.65rem;
    flex-shrink: 0;
    padding: 0;
    border: none;
    border-radius: var(--radius-pill);
    background: var(--status-color, var(--color-border-strong));
    cursor: pointer;
    transition: transform var(--duration-fast) var(--ease-out-expo);
  }

  .subtask-status-dot:hover {
    transform: scale(1.2);
  }

  .subtask-status-dot:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .create-subtask-button {
    align-self: flex-start;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--color-ink-muted);
    font-size: var(--text-xs);
    font-weight: 600;
    cursor: pointer;
    text-decoration: underline;
  }

  .create-subtask-button:hover {
    color: var(--color-accent);
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

  .date-input-row {
    display: flex;
    align-items: center;
    gap: var(--space-3xs);
  }

  .date-input-row input {
    flex: 1;
    min-width: 0;
  }

  .estimate-inputs {
    display: flex;
    align-items: center;
    gap: var(--space-3xs);
    text-transform: none;
    letter-spacing: normal;
    font-weight: 400;
  }

  .estimate-inputs input {
    width: 3.5rem;
    flex: none;
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
    flex-wrap: wrap;
    gap: var(--space-2xs);
    margin-top: var(--space-3xs);
  }

  .edit-actions button {
    /* `flex-wrap` alone isn't enough — flex items default to `min-width:
       auto`, which only stops shrinking once a button's own content can't
       get any narrower, by which point all three are squeezed onto one
       row and the last one (Delete) overflows past the card's edge on a
       narrow column instead of wrapping. An explicit min-width gives the
       row a real point at which it must wrap a button onto its own line. */
    flex: 1 1 3.5rem;
    min-width: 3.5rem;
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

  .remove-recurrence-link {
    align-self: flex-start;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--color-ink-muted);
    font-size: var(--text-xs);
    font-weight: 600;
    text-decoration: underline;
    cursor: pointer;
  }

  .remove-recurrence-link:hover {
    color: var(--color-danger);
  }

  .recurrence-locked-note {
    margin: 0;
    font-size: var(--text-xs);
    font-weight: 600;
    text-transform: none;
    letter-spacing: normal;
    color: var(--color-ink-faint);
  }
</style>
