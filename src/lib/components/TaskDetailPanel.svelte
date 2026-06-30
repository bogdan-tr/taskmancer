<script lang="ts">
  import { goto } from "$app/navigation";
  import { getSeries, getTaskHistory } from "$lib/api";
  import { applyTagsSuggestion, filterSuggestions, splitTagsInput } from "$lib/autocomplete";
  import { getErrorMessage } from "$lib/errors";
  import {
    hoursAndMinutesFromMinutes,
    minutesFromHoursAndMinutes,
    normalizeHoursMinutes,
  } from "$lib/estimatedTime";
  import { generalState } from "$lib/generalSettings.svelte";
  import { parseMarkdown, toggleCheckboxAt, type Inline } from "$lib/markdown";
  import { FALLBACK_PRIORITIES, sortedPriorities } from "$lib/priorities.svelte";
  import { projectsState } from "$lib/projects.svelte";
  import { selfAndAncestors } from "$lib/projectTree";
  import {
    dueRuleFromDefaultCode,
    formatDueRule,
    formatRecurrenceFrequency,
    type DueRule,
    type RecurrenceBuilderValue,
    type RecurrenceFrequency,
    type SeriesEditScope,
  } from "$lib/recurrence";
  import { settingsState } from "$lib/settings.svelte";
  import { FALLBACK_STATUSES, sortedStatuses } from "$lib/statuses.svelte";
  import { buildStatusTimeline, formatStatusDuration } from "$lib/statusTimeline";
  import { containerOwner, subtasksOf } from "$lib/subtasks";
  import {
    emptyToUndefined,
    formatTags,
    isValidOptionalDate,
    parseTags,
    taskEditableFieldsChanged,
  } from "$lib/taskFields";
  import { effectiveDefaultCode } from "$lib/taskPreview";
  import { tagsState } from "$lib/tags.svelte";
  import { upsertCachedTask } from "$lib/tasks.svelte";
  import {
    isTaskActive,
    liveDisplaySecondsFor,
    startTaskTracking,
    stopTaskTracking,
  } from "$lib/tracking.svelte";
  import { formatHms } from "$lib/liveTimer";
  import type { Series, StatusHistoryEntry, Task } from "$lib/types";
  import Autocomplete from "./Autocomplete.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import DatePickerPopover from "./DatePickerPopover.svelte";
  import RecurrenceBuilderDialog from "./RecurrenceBuilderDialog.svelte";
  import SeriesScopeDialog from "./SeriesScopeDialog.svelte";
  import TimeLogSection from "./TimeLogSection.svelte";

  interface Props {
    open: boolean;
    task: Task | undefined;
    /** Persists an edit. Returns a promise so the panel can show Saving→Saved. */
    onSave: (task: Task, scope?: SeriesEditScope) => void | Promise<void>;
    onDelete: (id: string, scope?: SeriesEditScope) => void;
    onRemoveRecurrence: (id: string) => void;
    onUpdateRecurrence: (
      seriesId: string,
      cutoff: string,
      frequency: RecurrenceFrequency,
      dueRule: DueRule,
      endDate: string | undefined,
    ) => void;
    /** Closes the panel. */
    onClose: () => void;
    /** Global task list — for subtask lookups and parent-owner gating. */
    allTasks?: Task[];
    /** Opens the Add Task modal pre-filled to create a subtask of this task. */
    onCreateSubtask?: (task: Task) => void;
  }

  let {
    open,
    task,
    onSave,
    onDelete,
    onRemoveRecurrence,
    onUpdateRecurrence,
    onClose,
    allTasks = [],
    onCreateSubtask,
  }: Props = $props();

  const priorities = $derived(settingsState.current?.priorities ?? FALLBACK_PRIORITIES);
  const statuses = $derived(sortedStatuses(settingsState.current?.statuses ?? FALLBACK_STATUSES));
  /** The task that owns this task's container, if it's itself a subtask. */
  const parentTask = $derived(task?.project_id ? containerOwner(task.project_id, allTasks) : undefined);
  const isThisTaskSubtask = $derived(parentTask !== undefined);
  const taskSubtasks = $derived(task ? subtasksOf(task, allTasks) : []);
  const subtasksDone = $derived(
    taskSubtasks.filter((t) => t.status === (settingsState.current?.done_status ?? "done")).length,
  );

  /** Root→leaf project chain for the header breadcrumb. */
  const projectChain = $derived(
    task?.project_id ? [...selfAndAncestors(projectsState.items, task.project_id)].reverse() : [],
  );

  // ── Draft fields (re-initialized when the panel switches to a new task) ──
  let draftTitle = $state("");
  let draftProject = $state("");
  let draftTags = $state("");
  let draftPriority = $state("medium");
  let draftStatus = $state("");
  let draftDue = $state("");
  let draftScheduled = $state("");
  let draftEstimatedHours: number | undefined = $state(undefined);
  let draftEstimatedMinutes: number | undefined = $state(undefined);
  let draftNotes = $state("");
  let editError = $state("");

  /** "" | "saving" | "saved" — drives the header save indicator. */
  let saveStatus: "" | "saving" | "saved" = $state("");
  let savedFlashTimer: ReturnType<typeof setTimeout> | undefined;

  /** Notes edit mode: false renders markdown, true shows the textarea. */
  let notesEditing = $state(false);

  let projectSuggestions: string[] = $state([]);
  let projectSuggestionIndex = $state(0);
  let tagSuggestions: string[] = $state([]);
  let tagSuggestionIndex = $state(0);

  let seriesInfo: Series | undefined = $state();
  let seriesLoadError = $state("");
  let history: StatusHistoryEntry[] = $state([]);

  let showDeleteConfirm = $state(false);
  let showRemoveRecurrenceConfirm = $state(false);
  let pendingScopeKind: "save" | "delete" | undefined = $state(undefined);
  let pendingScopeTask: Task | undefined = $state(undefined);
  let scopeDialogOpen = $derived(pendingScopeKind !== undefined);
  let scopeDialogTitle = $derived(
    pendingScopeKind === "delete" ? "Delete recurring task" : "Save changes to recurring task",
  );
  const scopeDialogMessage =
    "This task repeats. Apply this to just this task, or to this task and every future one?";

  /** Tracks which task the drafts were loaded from, so an in-place data
   *  refresh of the SAME task (e.g. after our own save) doesn't clobber drafts —
   *  only switching to a DIFFERENT task re-initializes them. */
  let loadedTaskId: string | undefined = $state(undefined);

  /** The panel root + title input, for the focus trap and focus-on-open. */
  let panelEl: HTMLElement | undefined = $state();
  let titleInputEl: HTMLInputElement | undefined = $state();

  const timelineRows = $derived(buildStatusTimeline(history));

  $effect(() => {
    if (open && task && task.id !== loadedTaskId) {
      loadedTaskId = task.id;
      draftTitle = task.title;
      draftProject = projectsState.items.find((p) => p.id === task.project_id)?.name ?? "";
      draftTags = formatTags(task.tags);
      draftPriority = task.priority;
      draftStatus = task.status;
      draftDue = task.due ?? "";
      draftScheduled = task.scheduled ?? "";
      const est =
        task.estimated_minutes !== undefined ? hoursAndMinutesFromMinutes(task.estimated_minutes) : undefined;
      draftEstimatedHours = est?.hours;
      draftEstimatedMinutes = est?.minutes;
      draftNotes = task.notes;
      editError = "";
      saveStatus = "";
      notesEditing = false;
      projectSuggestions = [];
      tagSuggestions = [];
      seriesInfo = undefined;
      seriesLoadError = "";
      history = [];
      if (task.series_id !== undefined) void loadSeriesInfo(task.series_id);
      void loadHistory(task.id);
      // Move focus into the panel on open (spec 3.4) so Tab cycles its
      // fields immediately. Deferred a frame so the just-rendered input is
      // focusable even as the panel slides in.
      requestAnimationFrame(() => titleInputEl?.focus());
    } else if (!open) {
      loadedTaskId = undefined;
      // Don't leave focus stranded on a now-hidden field — return it to the
      // body so the board's window key handlers (vim) take over again.
      if (panelEl?.contains(document.activeElement)) {
        (document.activeElement as HTMLElement | null)?.blur();
      }
    }
  });

  /** Guards against a stale response when the panel moves to another task. */
  async function loadSeriesInfo(seriesId: string) {
    try {
      const result = await getSeries(seriesId);
      if (task?.series_id === seriesId) seriesInfo = result;
    } catch (error) {
      if (task?.series_id === seriesId) {
        seriesLoadError = getErrorMessage(error, "Failed to load recurrence");
      }
    }
  }

  async function loadHistory(taskId: string) {
    try {
      const result = await getTaskHistory(taskId);
      if (task?.id === taskId) history = result;
    } catch {
      // History is supplementary — a load failure leaves the section empty
      // rather than blocking the rest of the panel.
      if (task?.id === taskId) history = [];
    }
  }

  function normalizeEstimateDraft() {
    if (draftEstimatedHours === undefined && draftEstimatedMinutes === undefined) {
      commit();
      return;
    }
    const normalized = normalizeHoursMinutes(draftEstimatedHours ?? 0, draftEstimatedMinutes ?? 0);
    draftEstimatedHours = normalized.hours;
    draftEstimatedMinutes = normalized.minutes;
    commit();
  }

  /** Builds the updated task from the current drafts. */
  function buildUpdatedTask(): Task | undefined {
    if (!task) return undefined;
    const estimatedMinutes =
      draftEstimatedHours === undefined && draftEstimatedMinutes === undefined
        ? undefined
        : minutesFromHoursAndMinutes(draftEstimatedHours ?? 0, draftEstimatedMinutes ?? 0);
    return {
      ...task,
      title: draftTitle.trim() === "" ? task.title : draftTitle,
      project_id: parentTask
        ? task.project_id
        : projectsState.items.find((p) => p.name.toLowerCase() === draftProject.trim().toLowerCase())?.id,
      tags: parentTask ? task.tags : parseTags(draftTags),
      priority: draftPriority,
      status: draftStatus,
      due: parentTask ? task.due : emptyToUndefined(draftDue),
      scheduled: parentTask ? task.scheduled : emptyToUndefined(draftScheduled),
      estimated_minutes: estimatedMinutes,
      notes: draftNotes,
    };
  }

  /**
   * Auto-save entry point — called on blur of every editable field. Validates,
   * skips when nothing changed, and for a recurring occurrence saves with
   * scope "this" (per-occurrence edits; series-wide changes go through the
   * "Edit recurrence" path, keeping auto-save free of scope prompts).
   */
  async function commit() {
    if (!task) return;
    if (!isValidOptionalDate(draftDue) || !isValidOptionalDate(draftScheduled)) {
      editError = "Due and scheduled dates must be in YYYY-MM-DD format";
      return;
    }
    editError = "";
    const updated = buildUpdatedTask();
    if (!updated || !taskEditableFieldsChanged(task, updated)) return;

    const scope: SeriesEditScope | undefined = task.series_id !== undefined ? "this" : undefined;
    saveStatus = "saving";
    try {
      await onSave(updated, scope);
      saveStatus = "saved";
      clearTimeout(savedFlashTimer);
      savedFlashTimer = setTimeout(() => (saveStatus = ""), 1500);
    } catch (error) {
      saveStatus = "";
      editError = getErrorMessage(error, "Failed to save");
    }
  }

  // ── Project / tag autocomplete (mirrors TaskEditDialog) ──
  function updateProjectSuggestions() {
    projectSuggestions = filterSuggestions(
      projectsState.items.map((p) => p.name),
      draftProject,
    );
    projectSuggestionIndex = 0;
  }
  function selectProjectSuggestion(suggestion: string) {
    draftProject = suggestion;
    projectSuggestions = [];
    commit();
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
    commit();
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

  // ── Notes (markdown) ──
  const notesBlocks = $derived(parseMarkdown(draftNotes));
  function startEditingNotes() {
    notesEditing = true;
  }
  function commitNotes() {
    notesEditing = false;
    commit();
  }
  function handleCheckboxToggle(checkboxIndex: number) {
    draftNotes = toggleCheckboxAt(draftNotes, checkboxIndex);
    commit();
  }

  // ── Delete + recurrence (reuse existing sub-dialogs) ──
  function handleDelete() {
    if (!task) return;
    if (task.series_id !== undefined) {
      pendingScopeKind = "delete";
    } else if (generalState.confirmTaskDeletion) {
      showDeleteConfirm = true;
    } else {
      onDelete(task.id);
    }
  }
  function confirmDelete() {
    if (!task) return;
    showDeleteConfirm = false;
    onDelete(task.id);
  }
  function handleScopeThis() {
    if (pendingScopeKind === "save" && pendingScopeTask) onSave(pendingScopeTask, "this");
    else if (pendingScopeKind === "delete" && task) onDelete(task.id, "this");
    pendingScopeKind = undefined;
    pendingScopeTask = undefined;
  }
  function handleScopeFuture() {
    if (pendingScopeKind === "save" && pendingScopeTask) onSave(pendingScopeTask, "future");
    else if (pendingScopeKind === "delete" && task) onDelete(task.id, "future");
    pendingScopeKind = undefined;
    pendingScopeTask = undefined;
  }
  function cancelScopeDialog() {
    pendingScopeKind = undefined;
    pendingScopeTask = undefined;
  }
  function confirmRemoveRecurrence() {
    if (!task) return;
    showRemoveRecurrenceConfirm = false;
    onRemoveRecurrence(task.id);
    if (seriesInfo) seriesInfo = { ...seriesInfo, active: false };
  }

  let recurrenceBuilderValue: RecurrenceBuilderValue | undefined = $derived(
    seriesInfo
      ? { frequency: seriesInfo.frequency, endDate: seriesInfo.end_date, dueRule: seriesInfo.due_rule }
      : undefined,
  );
  function handleRecurrenceBuilderApply(value: RecurrenceBuilderValue) {
    if (!task?.series_id || !task.scheduled) return;
    const chain = task.project_id ? selfAndAncestors(projectsState.items, task.project_id) : [];
    const globalDefaults = settingsState.current?.defaults ?? { tags: [] };
    const chainDue = chain.find((p) => p.defaults.due !== undefined)?.defaults.due;
    const dueRule = value.dueRule ?? dueRuleFromDefaultCode(effectiveDefaultCode(globalDefaults.due, chainDue));
    onUpdateRecurrence(task.series_id, task.scheduled, value.frequency, dueRule, value.endDate);
    onClose();
  }

  // ── Tracking ──
  const isTracking = $derived(task !== undefined && isTaskActive(task.id));
  let isTrackingPending = $state(false);
  async function handleTrackingToggle() {
    if (!task) return;
    const taskId = task.id;
    const wasTracking = isTracking;
    isTrackingPending = true;
    editError = "";
    try {
      if (wasTracking) {
        const trackedMinutes = await stopTaskTracking(taskId);
        if (task?.id === taskId) upsertCachedTask({ ...task, tracked_minutes: trackedMinutes });
      } else {
        const autoTransitioned = await startTaskTracking(taskId);
        if (autoTransitioned && task?.id === taskId) draftStatus = autoTransitioned.status;
      }
    } catch (error) {
      editError = getErrorMessage(error, "Failed to update tracking");
    } finally {
      isTrackingPending = false;
    }
  }

  // ── Subtasks ──
  function toggleSubtask(subtask: Task) {
    const doneStatus = settingsState.current?.done_status ?? "done";
    // Un-checking returns the subtask to the first (leftmost) status —
    // typically "backlog", the same status new tasks default to.
    const undoneStatus = statuses[0]?.id ?? "backlog";
    const next = subtask.status === doneStatus ? undoneStatus : doneStatus;
    void onSave({ ...subtask, status: next });
  }

  function navigateToProject(id: string) {
    void goto(`/projects/${id}`);
  }

  const FOCUSABLE_SELECTOR =
    'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

  /** Currently-visible focusable elements inside the panel, in DOM order. */
  function focusableEls(): HTMLElement[] {
    if (!panelEl) return [];
    return Array.from(panelEl.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
      (el) => el.offsetParent !== null || el === document.activeElement,
    );
  }

  /**
   * Keeps keyboard focus inside the open panel and adds vim-style dropdown
   * editing:
   * - Tab / Shift+Tab cycle the panel's fields and wrap at the ends, so focus
   *   never leaks out to the sidebar or board (Esc is how you leave).
   * - j / k move the selected option of a focused `<select>` (Status,
   *   Priority); the native arrow keys already do this too.
   */
  function handlePanelKeydown(event: KeyboardEvent) {
    if (event.key === "Tab") {
      const focusable = focusableEls();
      if (focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      const active = document.activeElement as HTMLElement | null;
      // Always keep Tab handling inside the panel.
      event.stopPropagation();
      if (event.shiftKey) {
        if (active === first || !panelEl?.contains(active)) {
          event.preventDefault();
          last.focus();
        }
      } else if (active === last || !panelEl?.contains(active)) {
        event.preventDefault();
        first.focus();
      }
      return;
    }

    if (
      (event.key === "j" || event.key === "k") &&
      !event.ctrlKey &&
      !event.altKey &&
      !event.metaKey &&
      document.activeElement instanceof HTMLSelectElement
    ) {
      const select = document.activeElement;
      const next = select.selectedIndex + (event.key === "j" ? 1 : -1);
      if (next >= 0 && next < select.options.length) {
        event.preventDefault();
        event.stopPropagation();
        select.selectedIndex = next;
        select.dispatchEvent(new Event("change", { bubbles: true }));
      }
    }
  }

  /** Renders one inline node's text — bold/italic carry a plain string. */
  function inlineText(node: Inline): string {
    return node.value;
  }
</script>

<aside
  bind:this={panelEl}
  class="detail-panel"
  class:open
  aria-label="Task details"
  aria-hidden={!open}
  onkeydown={handlePanelKeydown}
>
  {#if task}
    <header class="panel-header">
      <div class="header-top">
        <div class="save-indicator" aria-live="polite">
          {#if saveStatus === "saving"}Saving…{:else if saveStatus === "saved"}Saved ✓{/if}
        </div>
        <button type="button" class="close-button" onclick={onClose} aria-label="Close details" title="Close (Esc)">
          ×
        </button>
      </div>

      <input
        bind:this={titleInputEl}
        class="title-input"
        bind:value={draftTitle}
        onblur={commit}
        aria-label="Task title"
        placeholder="Untitled task"
      />

      <div class="header-meta">
        <select class="status-pill" bind:value={draftStatus} onchange={commit} aria-label="Status">
          {#each statuses as status (status.id)}
            <option value={status.id}>{status.label}</option>
          {/each}
        </select>

        {#if projectChain.length > 0}
          <nav class="breadcrumb" aria-label="Project">
            {#each projectChain as p, i (p.id)}
              {#if i > 0}<span class="crumb-sep">›</span>{/if}
              <button type="button" class="crumb" onclick={() => navigateToProject(p.id)}>{p.name}</button>
            {/each}
          </nav>
        {/if}
      </div>
    </header>

    <div class="panel-body">
      <!-- Metadata -->
      <section class="meta-grid" aria-label="Task metadata">
        <label class="meta-field">
          <span class="meta-label">Priority</span>
          <select bind:value={draftPriority} onchange={commit}>
            {#each sortedPriorities(priorities) as level (level.id)}
              <option value={level.id}>{level.label}</option>
            {/each}
          </select>
        </label>

        <label class="meta-field">
          <span class="meta-label">Estimate</span>
          <span class="estimate-inputs">
            <input
              type="number"
              min="0"
              step="1"
              placeholder="0"
              bind:value={draftEstimatedHours}
              onblur={normalizeEstimateDraft}
              aria-label="Estimated hours"
            />h
            <input
              type="number"
              min="0"
              step="1"
              placeholder="0"
              bind:value={draftEstimatedMinutes}
              onblur={normalizeEstimateDraft}
              aria-label="Estimated minutes"
            />m
          </span>
        </label>

        <label class="meta-field">
          <span class="meta-label">Scheduled</span>
          {#if parentTask}
            <span class="locked-value">{draftScheduled || "—"}</span>
          {:else}
            <span class="date-row">
              <input type="text" bind:value={draftScheduled} placeholder="YYYY-MM-DD" onblur={commit} />
              <DatePickerPopover
                selected={draftScheduled || undefined}
                triggerLabel="Pick scheduled date"
                clearLabel="Clear"
                onSelect={(iso) => {
                  draftScheduled = iso;
                  commit();
                }}
                onClear={() => {
                  draftScheduled = "";
                  commit();
                }}
              />
            </span>
          {/if}
        </label>

        <label class="meta-field">
          <span class="meta-label">Due</span>
          {#if parentTask}
            <span class="locked-value">{draftDue || "—"}</span>
          {:else}
            <span class="date-row">
              <input type="text" bind:value={draftDue} placeholder="YYYY-MM-DD" onblur={commit} />
              <DatePickerPopover
                selected={draftDue || undefined}
                triggerLabel="Pick due date"
                clearLabel="Never"
                onSelect={(iso) => {
                  draftDue = iso;
                  commit();
                }}
                onClear={() => {
                  draftDue = "";
                  commit();
                }}
              />
            </span>
          {/if}
        </label>

        <label class="meta-field meta-wide">
          <span class="meta-label">Tags</span>
          {#if parentTask}
            <span class="locked-value">{draftTags || "—"}</span>
          {:else}
            <div class="field-with-suggestions">
              <input
                type="text"
                bind:value={draftTags}
                placeholder="comma, separated"
                role="combobox"
                aria-expanded={tagSuggestions.length > 0}
                aria-controls="detail-tags-suggestions"
                aria-autocomplete="list"
                oninput={updateTagSuggestions}
                onkeydown={handleTagsKeydown}
                onblur={() => {
                  tagSuggestions = [];
                  commit();
                }}
              />
              <Autocomplete
                id="detail-tags-suggestions"
                items={tagSuggestions}
                activeIndex={tagSuggestionIndex}
                onSelect={selectTagSuggestion}
                onHover={(index) => (tagSuggestionIndex = index)}
                prefix="#"
              />
            </div>
          {/if}
        </label>

        {#if !parentTask}
          <label class="meta-field meta-wide">
            <span class="meta-label">Project</span>
            <div class="field-with-suggestions">
              <input
                type="text"
                bind:value={draftProject}
                placeholder="e.g. Inbox/Personal"
                role="combobox"
                aria-expanded={projectSuggestions.length > 0}
                aria-controls="detail-project-suggestions"
                aria-autocomplete="list"
                oninput={updateProjectSuggestions}
                onkeydown={handleProjectKeydown}
                onblur={() => {
                  projectSuggestions = [];
                  commit();
                }}
              />
              <Autocomplete
                id="detail-project-suggestions"
                items={projectSuggestions}
                activeIndex={projectSuggestionIndex}
                onSelect={selectProjectSuggestion}
                onHover={(index) => (projectSuggestionIndex = index)}
              />
            </div>
          </label>
        {/if}

        <div class="meta-field meta-wide">
          <span class="meta-label">Tracked time</span>
          <div class="tracked-row">
            <span class="tracked-value">
              {#if isTracking}
                <span class="tracked-ticker">
                  {formatHms(
                    liveDisplaySecondsFor(
                      task,
                      settingsState.current?.card_tracked_time_display ?? "total",
                    ) ?? 0,
                  )}
                </span>
              {:else}
                {task.tracked_minutes} min
              {/if}
            </span>
            <button
              type="button"
              class="track-button"
              class:tracking-active={isTracking}
              onclick={handleTrackingToggle}
              disabled={isTrackingPending}
              aria-label={isTracking ? "Stop tracking" : "Start tracking"}
              title={isTracking ? "Stop tracking" : "Start tracking"}
            >
              {#if isTracking}■{:else}▶{/if}
            </button>
          </div>
        </div>

        {#if task.series_id !== undefined}
          <div class="meta-field meta-wide">
            <span class="meta-label">Series</span>
            {#if seriesInfo}
              <span class="series-badge">
                {formatRecurrenceFrequency(seriesInfo.frequency)}{seriesInfo.end_date
                  ? ` until ${seriesInfo.end_date}`
                  : ""} · Due: {formatDueRule(seriesInfo.due_rule)}
                {#if !seriesInfo.active}(removed){:else if isThisTaskSubtask}(from parent){/if}
              </span>
              {#if seriesInfo.active && !isThisTaskSubtask}
                <div class="series-actions">
                  <RecurrenceBuilderDialog
                    value={recurrenceBuilderValue}
                    triggerLabel="Edit recurrence"
                    onApply={handleRecurrenceBuilderApply}
                    onClear={() => {}}
                  />
                  <button
                    type="button"
                    class="link-button"
                    onclick={() => (showRemoveRecurrenceConfirm = true)}
                  >
                    Remove recurrence
                  </button>
                </div>
              {/if}
            {:else if seriesLoadError}
              <span class="series-badge series-error">{seriesLoadError}</span>
            {:else}
              <span class="series-badge">Loading…</span>
            {/if}
          </div>
        {/if}
      </section>

      {#if editError}
        <p class="panel-error" role="alert">{editError}</p>
      {/if}

      <!-- Notes -->
      <section class="panel-section" aria-label="Notes">
        <h3 class="section-title">Notes</h3>
        {#if notesEditing}
          <textarea
            class="notes-textarea"
            bind:value={draftNotes}
            onblur={commitNotes}
            rows="5"
            placeholder="Supports **bold**, *italic*, # headings, - bullets, - [ ] checkboxes"
          ></textarea>
        {:else if draftNotes.trim() === ""}
          <button type="button" class="notes-placeholder" onclick={startEditingNotes}>
            No notes yet — click to add…
          </button>
        {:else}
          <div
            class="notes-rendered"
            role="button"
            tabindex="0"
            onclick={startEditingNotes}
            onkeydown={(e) => {
              if (e.key === "Enter") startEditingNotes();
            }}
          >
            {#each notesBlocks as block}
              {#if block.type === "heading"}
                <svelte:element this={`h${block.level}`} class="md-heading">
                  {#each block.content as node}<span class:md-bold={node.type === "bold"} class:md-italic={node.type === "italic"}>{inlineText(node)}</span>{/each}
                </svelte:element>
              {:else if block.type === "paragraph"}
                <p class="md-paragraph">
                  {#each block.content as node}<span class:md-bold={node.type === "bold"} class:md-italic={node.type === "italic"}>{inlineText(node)}</span>{/each}
                </p>
              {:else if block.type === "bullet_list"}
                <ul class="md-bullets">
                  {#each block.items as item}
                    <li>{#each item as node}<span class:md-bold={node.type === "bold"} class:md-italic={node.type === "italic"}>{inlineText(node)}</span>{/each}</li>
                  {/each}
                </ul>
              {:else if block.type === "checklist"}
                <ul class="md-checklist">
                  {#each block.items as item}
                    <li>
                      <input
                        type="checkbox"
                        checked={item.checked}
                        onclick={(e) => {
                          e.stopPropagation();
                          handleCheckboxToggle(item.checkboxIndex);
                        }}
                        aria-label={item.content.map(inlineText).join("")}
                      />
                      <span class:md-checked={item.checked}>{#each item.content as node}<span class:md-bold={node.type === "bold"} class:md-italic={node.type === "italic"}>{inlineText(node)}</span>{/each}</span>
                    </li>
                  {/each}
                </ul>
              {/if}
            {/each}
          </div>
        {/if}
      </section>

      <!-- Subtasks -->
      {#if taskSubtasks.length > 0 || (!isThisTaskSubtask && onCreateSubtask)}
        <section class="panel-section" aria-label="Subtasks">
          <h3 class="section-title">
            Subtasks
            {#if taskSubtasks.length > 0}<span class="muted">{subtasksDone}/{taskSubtasks.length}</span>{/if}
          </h3>
          {#if taskSubtasks.length > 0}
            <div class="subtask-progress" role="progressbar" aria-valuemin={0} aria-valuemax={taskSubtasks.length} aria-valuenow={subtasksDone}>
              <div class="subtask-progress-fill" style="transform: scaleX({taskSubtasks.length > 0 ? subtasksDone / taskSubtasks.length : 0})"></div>
            </div>
            <ul class="subtask-list">
              {#each taskSubtasks as subtask (subtask.id)}
                <li>
                  <input
                    type="checkbox"
                    checked={subtask.status === (settingsState.current?.done_status ?? "done")}
                    onchange={() => toggleSubtask(subtask)}
                    aria-label={subtask.title}
                  />
                  <span class:md-checked={subtask.status === (settingsState.current?.done_status ?? "done")}>{subtask.title}</span>
                </li>
              {/each}
            </ul>
          {/if}
          {#if !isThisTaskSubtask && onCreateSubtask}
            <button type="button" class="link-button" onclick={() => onCreateSubtask(task)}>+ Create subtask</button>
          {/if}
        </section>
      {/if}

      <!-- History -->
      {#if timelineRows.length > 0}
        <section class="panel-section" aria-label="Status history">
          <h3 class="section-title">History</h3>
          <ol class="timeline">
            {#each timelineRows as row (row.changedAt)}
              <li class="timeline-row" class:is-seed={row.isSeed} class:is-current={row.isCurrent}>
                <span class="timeline-dot"></span>
                <span class="timeline-status">{row.status}</span>
                <span class="timeline-date">
                  {row.isSeed ? "~" : ""}{new Date(row.changedAt).toLocaleDateString()}
                </span>
                <span class="timeline-duration">{formatStatusDuration(row.durationMs)}</span>
              </li>
            {/each}
          </ol>
        </section>
      {/if}

      <!-- Time entries -->
      <section class="panel-section" aria-label="Time entries">
        <TimeLogSection {task} />
      </section>

      <!-- Danger zone -->
      <section class="panel-section danger-zone">
        <button type="button" class="delete-button" onclick={handleDelete}>Delete task</button>
      </section>
    </div>
  {/if}
</aside>

<ConfirmDialog
  open={showDeleteConfirm}
  title="Delete task"
  message={task
    ? taskSubtasks.length > 0
      ? `Delete "${task.title}"? This will also delete ${taskSubtasks.length} subtask${taskSubtasks.length === 1 ? "" : "s"}. This can't be undone.`
      : `Delete "${task.title}"? This can't be undone.`
    : ""}
  confirmLabel="Delete"
  onConfirm={confirmDelete}
  onCancel={() => (showDeleteConfirm = false)}
/>

<ConfirmDialog
  open={showRemoveRecurrenceConfirm}
  title="Remove recurrence?"
  message="This stops the series from generating any more occurrences. Tasks already created (past and future) are left as they are."
  confirmLabel="Remove recurrence"
  onConfirm={confirmRemoveRecurrence}
  onCancel={() => (showRemoveRecurrenceConfirm = false)}
/>

<SeriesScopeDialog
  open={scopeDialogOpen}
  title={scopeDialogTitle}
  message={scopeDialogMessage}
  onThis={handleScopeThis}
  onFuture={handleScopeFuture}
  onCancel={cancelScopeDialog}
/>

<style>
  .detail-panel {
    position: fixed;
    top: 0;
    right: 0;
    height: 100vh;
    width: min(34rem, 94vw);
    background: var(--color-surface-raised);
    color: var(--color-ink);
    border-left: 1px solid var(--color-border);
    box-shadow: var(--shadow-lg);
    display: flex;
    flex-direction: column;
    transform: translateX(100%);
    transition: transform var(--duration-normal, 300ms) var(--ease-out-expo, cubic-bezier(0.16, 1, 0.3, 1));
    z-index: 40;
    overflow: hidden;
  }

  .detail-panel.open {
    transform: translateX(0);
  }

  .panel-header {
    padding: var(--space-md) var(--space-md) var(--space-sm);
    border-bottom: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    background: var(--color-surface-raised);
  }

  .header-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 1.25rem;
  }

  .save-indicator {
    font-size: var(--text-xs);
    color: var(--color-accent);
    font-weight: 600;
  }

  .close-button {
    border: none;
    background: transparent;
    color: var(--color-ink-muted);
    font-size: var(--text-xl);
    line-height: 1;
    cursor: pointer;
    padding: 0 var(--space-2xs);
    border-radius: var(--radius-sm);
  }
  .close-button:hover {
    color: var(--color-ink);
    background: var(--color-canvas);
  }

  .title-input {
    border: 1px solid transparent;
    background: transparent;
    color: var(--color-ink);
    font-size: var(--text-lg);
    font-weight: 600;
    letter-spacing: var(--tracking-tight);
    padding: var(--space-3xs) var(--space-2xs);
    border-radius: var(--radius-sm);
    width: 100%;
  }
  .title-input:hover {
    border-color: var(--color-border);
  }
  .title-input:focus-visible {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-soft);
    outline: none;
  }

  .header-meta {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }

  .status-pill {
    font-size: var(--text-xs);
    font-weight: 600;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill, 999px);
    background: var(--color-surface);
    color: var(--color-ink);
    padding: var(--space-3xs) var(--space-xs);
  }

  .breadcrumb {
    display: flex;
    align-items: center;
    gap: var(--space-3xs);
    flex-wrap: wrap;
    font-size: var(--text-xs);
    color: var(--color-ink-muted);
  }
  .crumb {
    border: none;
    background: transparent;
    color: var(--color-ink-muted);
    cursor: pointer;
    padding: 0;
    font: inherit;
  }
  .crumb:hover {
    color: var(--color-accent);
    text-decoration: underline;
  }
  .crumb-sep {
    opacity: 0.5;
  }

  .panel-body {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: var(--space-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
  }

  .meta-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-sm);
  }
  .meta-field {
    display: flex;
    flex-direction: column;
    gap: var(--space-3xs);
    font-size: var(--text-xs);
    /* Grid items default to min-width:auto and won't shrink below their
       content, which forced the panel to scroll horizontally. */
    min-width: 0;
  }
  .meta-wide {
    grid-column: 1 / -1;
  }
  .meta-label {
    font-weight: 600;
    color: var(--color-ink-muted);
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    font-size: var(--text-2xs, 0.6875rem);
  }
  .meta-field select,
  .meta-field input[type="text"],
  .meta-field input[type="number"] {
    box-sizing: border-box;
    max-width: 100%;
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
    border-radius: var(--radius-sm);
    padding: var(--space-3xs) var(--space-2xs);
    font: inherit;
    font-size: var(--text-sm);
  }
  .meta-field input:focus-visible,
  .meta-field select:focus-visible {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-soft);
    outline: none;
  }
  .estimate-inputs {
    display: flex;
    align-items: center;
    gap: var(--space-3xs);
  }
  .estimate-inputs input {
    width: 3rem;
  }
  .date-row {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
  }
  .date-row input {
    flex: 1;
    min-width: 0;
  }
  .field-with-suggestions {
    position: relative;
  }
  .field-with-suggestions input {
    width: 100%;
    box-sizing: border-box;
  }
  .locked-value {
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
    padding: var(--space-3xs) 0;
  }

  .tracked-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-sm);
  }
  .tracked-value {
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
  }
  .tracked-ticker {
    color: var(--color-accent);
    font-weight: 700;
  }
  .track-button {
    width: 1.75rem;
    height: 1.75rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    color: var(--color-ink-muted);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .track-button.tracking-active {
    border-color: color-mix(in oklch, var(--color-accent) 50%, var(--color-border));
    background: var(--color-accent-soft);
    color: var(--color-accent);
  }

  .series-badge {
    font-size: var(--text-xs);
    color: var(--color-ink-muted);
  }
  .series-error {
    color: var(--color-danger);
  }
  .series-actions {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    margin-top: var(--space-3xs);
  }

  .section-title {
    margin: 0 0 var(--space-xs);
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
    display: flex;
    gap: var(--space-xs);
    align-items: baseline;
  }
  .muted {
    color: var(--color-ink-muted);
    font-weight: 500;
  }

  .notes-textarea {
    width: 100%;
    box-sizing: border-box;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-ink);
    padding: var(--space-xs) var(--space-sm);
    font: inherit;
    font-size: var(--text-sm);
    line-height: 1.5;
    resize: vertical;
  }
  .notes-textarea:focus-visible {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-soft);
    outline: none;
  }
  .notes-placeholder {
    border: 1px dashed var(--color-border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-ink-muted);
    padding: var(--space-sm);
    width: 100%;
    text-align: left;
    cursor: text;
    font: inherit;
    font-size: var(--text-sm);
  }
  .notes-rendered {
    cursor: text;
    font-size: var(--text-sm);
    line-height: 1.55;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    padding: var(--space-2xs);
  }
  .notes-rendered:hover {
    border-color: var(--color-border);
  }
  .md-heading {
    margin: var(--space-xs) 0 var(--space-3xs);
    font-weight: 700;
    line-height: 1.3;
  }
  .md-paragraph {
    margin: 0 0 var(--space-xs);
  }
  .md-bullets,
  .md-checklist {
    margin: 0 0 var(--space-xs);
    padding-left: var(--space-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-3xs);
  }
  .md-checklist {
    list-style: none;
    padding-left: 0;
  }
  .md-checklist li {
    display: flex;
    align-items: baseline;
    gap: var(--space-2xs);
  }
  .md-bold {
    font-weight: 700;
  }
  .md-italic {
    font-style: italic;
  }
  .md-checked {
    text-decoration: line-through;
    color: var(--color-ink-muted);
  }

  .subtask-progress {
    height: 4px;
    background: var(--color-canvas);
    border-radius: 999px;
    overflow: hidden;
    margin-bottom: var(--space-xs);
  }
  .subtask-progress-fill {
    height: 100%;
    width: 100%;
    transform-origin: left;
    background: var(--color-accent);
    transition: transform var(--duration-normal) var(--ease-out-expo);
  }
  .subtask-list {
    list-style: none;
    margin: 0 0 var(--space-xs);
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-3xs);
    font-size: var(--text-sm);
  }
  .subtask-list li {
    display: flex;
    align-items: baseline;
    gap: var(--space-2xs);
  }

  .timeline {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
  }
  .timeline-row {
    display: grid;
    grid-template-columns: auto 1fr auto auto;
    align-items: center;
    gap: var(--space-xs);
    font-size: var(--text-xs);
    padding: var(--space-3xs) var(--space-2xs);
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
  }
  .timeline-row.is-seed {
    border-style: dashed;
    border-color: var(--color-border);
    opacity: 0.8;
  }
  .timeline-row.is-current {
    background: var(--color-accent-soft);
  }
  .timeline-dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--color-accent);
  }
  .timeline-status {
    font-weight: 600;
  }
  .timeline-date {
    color: var(--color-ink-muted);
  }
  .timeline-duration {
    color: var(--color-ink-muted);
    font-variant-numeric: tabular-nums;
  }

  .panel-error {
    margin: 0;
    color: var(--color-danger);
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .link-button {
    border: none;
    background: transparent;
    color: var(--color-accent);
    cursor: pointer;
    font: inherit;
    font-size: var(--text-sm);
    padding: 0;
    text-align: left;
  }
  .link-button:hover {
    text-decoration: underline;
  }

  .danger-zone {
    margin-top: auto;
    padding-top: var(--space-md);
    border-top: 1px solid var(--color-border);
  }
  .delete-button {
    border: 1px solid var(--color-danger);
    background: transparent;
    color: var(--color-danger);
    border-radius: var(--radius-md);
    padding: var(--space-2xs) var(--space-sm);
    cursor: pointer;
    font: inherit;
    font-size: var(--text-sm);
  }
  .delete-button:hover {
    background: var(--color-danger-soft);
  }

  @media (prefers-reduced-motion: reduce) {
    .detail-panel {
      transition: none;
    }
  }
</style>
