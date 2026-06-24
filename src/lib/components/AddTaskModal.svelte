<script lang="ts">
  import { tick } from "svelte";
  import {
    applySubtaskTokenSuggestion,
    applyTagsSuggestion,
    applyTokenSuggestion,
    filterSuggestions,
    findActiveSubtaskToken,
    findActiveToken,
    MAX_SUGGESTIONS,
    preferredSuggestionText,
    projectPathSuggestions,
    splitTagsInput,
    type ActiveSubtaskToken,
    type ActiveToken,
  } from "$lib/autocomplete";
  import { ensureSubtaskContainer, getSeries } from "$lib/api";
  import { isLightColor } from "$lib/colorPresets";
  import { displayState } from "$lib/displaySettings.svelte";
  import { formatDueDateDisplay } from "$lib/dueDateDisplay";
  import { hoursAndMinutesFromMinutes, minutesFromHoursAndMinutes, normalizeHoursMinutes } from "$lib/estimatedTime";
  import { parseTaskInput, type ParsedTaskInput } from "$lib/naturalLanguage";
  import { FALLBACK_PRIORITIES, priorityColor, priorityLabel, sortedPriorities } from "$lib/priorities.svelte";
  import { resolveProjectColor } from "$lib/projectColor";
  import { projectsState } from "$lib/projects.svelte";
  import { findProjectByPath, projectPath, selfAndAncestors } from "$lib/projectTree";
  import { containerOwner, isSubtask, subtaskNameSuggestions } from "$lib/subtasks";
  import {
    dueRuleFromDefaultCode,
    formatDueRule,
    formatRecurrenceFrequency,
    resolveNonRecurringDue,
    resolveSeriesDueRule,
    type DueRule,
    type RecurrenceBuilderValue,
    type RecurrenceFrequency,
  } from "$lib/recurrence";
  import { settingsState } from "$lib/settings.svelte";
  import { tasksState } from "$lib/tasks.svelte";
  import { FALLBACK_STATUSES, sortedStatuses, statusLabel } from "$lib/statuses.svelte";
  import { formatTags, parseTags } from "$lib/taskFields";
  import { effectiveDefaultCode, resolveTaskPreview } from "$lib/taskPreview";
  import { tagsState } from "$lib/tags.svelte";
  import type { Series, Task } from "$lib/types";
  import Autocomplete from "./Autocomplete.svelte";
  import DatePickerPopover from "./DatePickerPopover.svelte";
  import RecurrenceBuilderDialog from "./RecurrenceBuilderDialog.svelte";

  interface Props {
    open: boolean;
    onClose: () => void;
    onSubmit: (parsed: ParsedTaskInput) => Promise<void> | void;
    errorMessage?: string;
    /** When set, this dialog was opened from a project-scoped board: new tasks default to this project. */
    projectFilter?: string;
    /**
     * Candidate tasks for resolving `parentTaskId` against — see
     * `matchedParentTask`. `KanbanBoard` passes its own currently-visible
     * task list, not literally every task in the app: a deliberate scoping
     * choice, not a limitation — you can only mark something a subtask of
     * a task you can currently see on the open board (which is the full
     * task list whenever no project filter is active, e.g. the "All Tasks"
     * view).
     */
    allTasks?: Task[];
    /**
     * Set by the "Create Subtask" entry points (a button on the task card,
     * the task edit dialog, or a Week/Calendar bar popover) to mark this
     * new task as a subtask of the task with this id. Deliberately a real
     * id, not text the user could accidentally delete while typing their
     * own title (the title field starts completely blank for this path) —
     * see the Subtasks design spec's revision after the original
     * text-token prefill turned out to both clutter the title and silently
     * lose the link if that text got edited.
     */
    parentTaskId?: string;
  }

  let { open, onClose, onSubmit, errorMessage = "", projectFilter, allTasks = [], parentTaskId }: Props = $props();

  let dialogEl: HTMLDialogElement | undefined = $state();
  let inputEl: HTMLInputElement | undefined = $state();
  let title = $state("");

  let priorities = $derived(settingsState.current?.priorities ?? FALLBACK_PRIORITIES);
  let knownPriorities = $derived(priorities.map(({ id, label }) => ({ id, label })));
  let statuses = $derived(settingsState.current?.statuses ?? FALLBACK_STATUSES);
  let knownStatuses = $derived(statuses.map(({ id, label }) => ({ id, label })));
  // Prefers each level's label for the suggestion list (falling back to its
  // id only when the label has whitespace, since a multi-word value can't
  // round-trip through a single bare token) — otherwise a renamed level
  // would still show its leftover auto-generated id (e.g. "new-status").
  let priorityOptions = $derived(
    sortedPriorities(priorities).map((level) => preferredSuggestionText(level.id, level.label)),
  );
  let statusOptions = $derived(
    sortedStatuses(statuses).map((status) => preferredSuggestionText(status.id, status.label)),
  );

  /**
   * The task that owns the subtask container currently being viewed
   * (`projectFilter` is that container's own project id), if any — i.e.
   * this dialog was opened via the plain "+ Add task" button while
   * already looking at a subtask's own board, not via a "Create Subtask"
   * button or a typed `sub <name>` token. Any task created here is
   * automatically a subtask of this owner, the same as if `sub
   * "<owner>"` had been typed — see `matchedParentTask`'s own doc
   * comment for why this takes priority over a typed `sub` token (which
   * is disabled entirely in this context, just below).
   *
   * Looked up against the global `tasksState`, not the `allTasks` prop —
   * on the container's *own* board, `allTasks` (sourced from that board's
   * `visibleTasks`) is exactly the subtasks themselves; the owning parent
   * task lives in a different project entirely and is never part of it,
   * so this reverse lookup would otherwise never resolve.
   */
  let viewedContainerOwner = $derived(
    projectFilter ? containerOwner(projectFilter, tasksState.items) : undefined,
  );
  let parsed = $derived(
    parseTaskInput(title, undefined, knownPriorities, knownStatuses, {
      disableSubtaskKeyword: viewedContainerOwner !== undefined,
    }),
  );

  // Explicit estimated-time controls. Until the user actually edits one of
  // these inputs, they stay in sync with the *fully resolved* estimate
  // shown elsewhere in this preview (`preview.estimatedMinutes`, declared
  // below) — not just the raw `est`/bare-duration quick-add token — so a
  // configured project/global default shows up here too, the same as it
  // already does in the read-only Due/Scheduled rows. The moment the user
  // touches an input directly, `estimateManuallySet` flips to `true` and
  // the boxes become authoritative instead — further title or default
  // changes no longer overwrite them.
  let draftEstimatedHours: number | undefined = $state(undefined);
  let draftEstimatedMinutes: number | undefined = $state(undefined);
  let estimateManuallySet = $state(false);

  let explicitEstimatedMinutes = $derived(
    draftEstimatedHours === undefined && draftEstimatedMinutes === undefined
      ? undefined
      : minutesFromHoursAndMinutes(draftEstimatedHours ?? 0, draftEstimatedMinutes ?? 0),
  );

  /**
   * Calendar-popup overrides for Due/Scheduled. Unlike the estimated-time
   * boxes above, picking a date here never rewrites the title text — it's a
   * silent override that wins in the preview, exactly the same precedence
   * model as the estimated-time boxes (manual wins over the quick-add token
   * once set), just without a pair of always-visible input boxes to keep in
   * sync. `draftDueOverride` may also be the `"none"` sentinel (the picker's
   * "Never" action), mirroring the `due:na`/`due na` quick-add token.
   */
  let draftDueOverride: string | undefined = $state(undefined);
  let dueManuallySet = $state(false);
  let draftScheduledOverride: string | undefined = $state(undefined);
  let scheduledManuallySet = $state(false);

  /**
   * Priority/Tags manual overrides — the same precedence model as
   * Due/Scheduled above (manual control wins over whatever quick-add
   * token is typed, until cleared), but for fields that previously had no
   * direct control at all (only the `!priority`/`#tag` quick-add tokens).
   * Added so a subtask's inherited attributes (see the parent-inheritance
   * effect below) land somewhere editable without cluttering the title
   * text — the actual fix for "I don't like how the text field gets
   * filled up with everything."
   */
  let draftPriorityOverride: string | undefined = $state(undefined);
  let priorityManuallySet = $state(false);
  let draftTagsInput = $state("");
  let tagsManuallySet = $state(false);
  let tagSuggestions: string[] = $state([]);
  let tagSuggestionIndex = $state(0);

  function handlePriorityChange() {
    priorityManuallySet = true;
  }

  function updateTagSuggestions() {
    const { current } = splitTagsInput(draftTagsInput);
    tagSuggestions = filterSuggestions(tagsState.items, current);
    tagSuggestionIndex = 0;
  }

  function selectTagSuggestion(suggestion: string) {
    const { prefix } = splitTagsInput(draftTagsInput);
    draftTagsInput = applyTagsSuggestion(prefix, suggestion);
    tagsManuallySet = true;
    tagSuggestions = [];
  }

  function handleTagsInput() {
    tagsManuallySet = true;
    updateTagSuggestions();
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

  function handleDueSelect(iso: string) {
    draftDueOverride = iso;
    dueManuallySet = true;
    // Most-recently-used override wins — the calendar-popup pick must beat
    // a stale due-rule choice from the recurrence builder, the same way
    // either one already beats a stale typed phrase.
    dueRuleManuallySet = false;
  }

  function handleDueNever() {
    draftDueOverride = "none";
    dueManuallySet = true;
    dueRuleManuallySet = false;
  }

  function handleScheduledSelect(iso: string) {
    draftScheduledOverride = iso;
    scheduledManuallySet = true;
  }

  /** Scheduled has no "never" concept — clearing just stops overriding and falls back to the token/default/today resolution chain. */
  function handleScheduledClear() {
    draftScheduledOverride = undefined;
    scheduledManuallySet = false;
  }

  /**
   * Recurrence-builder overrides. Mirrors the due/scheduled calendar-popup
   * pattern above: a manual builder selection wins over whatever "every
   * ..." phrase (and, for the due rule specifically, whatever due phrase)
   * is typed in the title, until cleared.
   */
  let draftRecurrenceOverride: { frequency: RecurrenceFrequency; endDate?: string } | undefined =
    $state(undefined);
  let recurrenceManuallySet = $state(false);
  let draftDueRuleOverride: DueRule | undefined = $state(undefined);
  let dueRuleManuallySet = $state(false);

  function handleRecurrenceApply(value: RecurrenceBuilderValue) {
    draftRecurrenceOverride = { frequency: value.frequency, endDate: value.endDate };
    recurrenceManuallySet = true;

    // The builder's due-rule section also sets the due override — most-
    // recently-used wins, the same as the calendar-popup pick. "Use the
    // default" is itself an explicit choice (not "leave whatever's typed
    // alone"): it resolves the configured default *now* and forces it,
    // mirroring create_recurring_task's own DefaultCode/Never fallback, so
    // it actively overrides a due phrase still sitting in the title rather
    // than silently doing nothing.
    draftDueRuleOverride =
      value.dueRule ??
      dueRuleFromDefaultCode(
        effectiveDefaultCode(globalDefaults.due, projectChain.find((p) => p.defaults.due !== undefined)?.defaults.due),
      );
    dueRuleManuallySet = true;
    dueManuallySet = false;
  }

  function handleRecurrenceClear() {
    recurrenceManuallySet = false;
    draftRecurrenceOverride = undefined;
    dueRuleManuallySet = false;
    draftDueRuleOverride = undefined;
  }

  /**
   * The existing task this new task should become a subtask of, in
   * priority order:
   * 1. `parentTaskId` — set directly by a "Create Subtask" button, the
   *    most explicit signal.
   * 2. `viewedContainerOwner` — this dialog is already scoped to a
   *    subtask container's own board (`projectFilter` names it), so any
   *    task created here is implicitly a subtask of its owner. The `sub`
   *    keyword is disabled in this exact case (see `parsed` above) so
   *    there's never a conflicting explicit target to prefer instead —
   *    nesting a subtask under a subtask is the one-level-deep rule's
   *    job to prevent, not a second, different parent to silently swap
   *    in.
   * 3. A typed `sub <name>` quick-add token, matched case-insensitively
   *    against active (non-done, non-cancelled), non-subtask tasks only
   *    (the one-level-deep rule, and the same autocomplete-candidate
   *    filter `subtaskNameSuggestions` already applies), first match
   *    wins.
   * Resolving the actual container project id happens at submit time in
   * `handleSubmit`, not here, since that can require creating one (an
   * async round-trip a `$derived` preview can't make — see the design
   * spec's "preview can't be async" decision); the preview shows a plain
   * `"Subtask of {title}"` label instead.
   */
  let matchedParentTask = $derived.by(() => {
    if (parentTaskId) return allTasks.find((t) => t.id === parentTaskId);
    if (viewedContainerOwner) return viewedContainerOwner;
    if (!parsed.subtaskParentName) return undefined;
    const typed = parsed.subtaskParentName.toLowerCase();
    const doneStatusId = settingsState.current?.done_status;
    const cancelledStatusId = settingsState.current?.cancelled_status;
    return allTasks.find(
      (t) =>
        !t.hidden &&
        t.title.toLowerCase() === typed &&
        t.status !== doneStatusId &&
        t.status !== cancelledStatusId &&
        !isSubtask(t, allTasks),
    );
  });

  /**
   * The recurring `Series` belonging to `matchedParentTask`, if it has
   * one — fetched fresh whenever the matched parent (or its `series_id`)
   * changes, the same way `TaskEditDialog`'s own `loadSeriesInfo` does for
   * an opened task. `Task` only ever carries `series_id`, never the
   * series' own frequency/due rule/end date, so this is the only way to
   * see them. `loadedParentSeriesId` is a plain (non-reactive) tracker, not
   * `$state` — the effect below both reads `matchedParentTask?.series_id`
   * and conditionally writes `parentSeriesInfo`, and tracking "have I
   * already loaded this one" via a *different* state variable than the
   * one being read would risk a self-triggering effect loop.
   */
  let parentSeriesInfo = $state<Series | undefined>(undefined);
  let loadedParentSeriesId: string | undefined = undefined;

  async function loadParentSeriesInfo(seriesId: string) {
    try {
      const result = await getSeries(seriesId);
      if (matchedParentTask?.series_id === seriesId) {
        parentSeriesInfo = result;
      }
    } catch {
      if (matchedParentTask?.series_id === seriesId) {
        parentSeriesInfo = undefined;
      }
    }
  }

  $effect(() => {
    const seriesId = matchedParentTask?.series_id;
    if (seriesId === loadedParentSeriesId) return;
    loadedParentSeriesId = seriesId;
    if (!seriesId) {
      parentSeriesInfo = undefined;
      return;
    }
    void loadParentSeriesInfo(seriesId);
  });

  /**
   * Whether this subtask's recurrence is locked to its (active-recurring)
   * parent's pattern — per the Subtasks design spec's recurrence-
   * inheritance decision, a subtask can never have an independently
   * different pattern than its parent. Drives `effectiveParsed.recurrence`/
   * `.dueRule` below (forced to the parent's values, overriding whatever
   * was typed or built) and the read-only Recurrence/Due rows in the
   * template (no Build button, no date picker).
   */
  let lockedToParentRecurrence = $derived(parentSeriesInfo?.active === true);

  /**
   * `parsed`, with the explicit estimated-time/due/scheduled/recurrence
   * controls overriding their quick-add tokens once manually set. A manual
   * due override (either the calendar-popup's literal date or the
   * recurrence builder's due-rule section) also clears whichever other
   * due override exists — the user's most recent, most explicit action
   * must win, not be silently overridden by a stale one.
   *
   * Once `matchedParentTask` is set, every attribute except Priority and
   * Estimated time is forced straight from the parent's *current* field
   * values instead — Tags/Scheduled always, Due/Recurrence either the
   * parent's own pattern (when `lockedToParentRecurrence`) or none at all
   * (a subtask of a non-recurring parent can't independently start
   * recurring either). This overrides every other source for those
   * fields — manual override, quick-add token, or recurrence builder
   * alike — per the design spec: a subtask's attributes other than
   * priority/estimate can never independently diverge from its parent's.
   *
   * Priority/Estimated time, by contrast, only *default* to the parent's
   * values (the last fallback below, behind a manual box edit and a typed
   * quick-add token) — staying genuinely independently settable, the way
   * the design spec wants. This is a plain fallback chain rather than a
   * one-time copy into the draft boxes (an earlier version of this dialog
   * had `inheritFromParent` write `draftPriorityOverride`/the estimate
   * boxes and flip `priorityManuallySet`/`estimateManuallySet` to `true`
   * the moment a `sub <name>` token resolved — which then made those
   * flags permanently prefer the inherited value over *any* further typed
   * `high`/`est <n>` token in the title, since the flag can't tell "the
   * user directly edited the box" apart from "this dialog auto-filled it
   * on the parent's behalf"). Falling back to `matchedParentTask` here
   * instead — after the typed token, not in place of it — fixes that: the
   * boxes still show the parent's value as a sensible starting point
   * (via the existing `preview.priorityId`/`.estimatedMinutes` mirroring
   * effects below) until a token or a direct edit overrides it.
   */
  let effectiveParsed: ParsedTaskInput = $derived({
    ...parsed,
    priority: priorityManuallySet
      ? draftPriorityOverride
      : parsed.priority ?? matchedParentTask?.priority,
    tags: matchedParentTask ? matchedParentTask.tags : tagsManuallySet ? parseTags(draftTagsInput) : parsed.tags,
    due: matchedParentTask
      ? lockedToParentRecurrence
        ? undefined
        : matchedParentTask.due
      : dueRuleManuallySet
        ? undefined
        : dueManuallySet
          ? draftDueOverride
          : parsed.due,
    dueRule: matchedParentTask
      ? lockedToParentRecurrence
        ? parentSeriesInfo?.due_rule
        : undefined
      : dueRuleManuallySet
        ? draftDueRuleOverride
        : dueManuallySet
          ? undefined
          : parsed.dueRule,
    scheduled: matchedParentTask
      ? matchedParentTask.scheduled
      : scheduledManuallySet
        ? draftScheduledOverride
        : parsed.scheduled,
    estimatedMinutes: estimateManuallySet
      ? explicitEstimatedMinutes
      : parsed.estimatedMinutes ?? matchedParentTask?.estimated_minutes,
    recurrence: matchedParentTask
      ? lockedToParentRecurrence
        ? { frequency: parentSeriesInfo!.frequency, endDate: parentSeriesInfo!.end_date }
        : undefined
      : recurrenceManuallySet
        ? draftRecurrenceOverride
        : parsed.recurrence,
  });

  function handleEstimateInput() {
    estimateManuallySet = true;
  }

  /** Rolls minutes >= 60 over into hours, e.g. typing 90 into "mins" reads back as 1h 30m. */
  function normalizeEstimateDraft() {
    if (draftEstimatedHours === undefined && draftEstimatedMinutes === undefined) return;
    const normalized = normalizeHoursMinutes(draftEstimatedHours ?? 0, draftEstimatedMinutes ?? 0);
    draftEstimatedHours = normalized.hours;
    draftEstimatedMinutes = normalized.minutes;
  }

  let defaultProjectName = $derived(
    projectsState.items.find((p) => p.id === settingsState.current?.default_project_id)?.name ?? "General",
  );
  let globalDefaults = $derived(settingsState.current?.defaults ?? { tags: [] });

  /**
   * The project the task will be created under: the `+Project` quick-add
   * token — a `/`-separated ancestor path (e.g. `Work/Client A`) resolved
   * unambiguously via `findProjectByPath`, else matched by bare name, first
   * match wins (see the NL parser's documented same-name-subproject
   * limitation for the bare-name case specifically) — else this dialog's
   * `projectFilter` (matched by id — `KanbanBoard` passes its own id-based
   * filter straight through), else the configured default project (also
   * matched by id). An unresolved path (e.g. a typo'd segment) falls
   * through to `projectFilter`/the default project rather than attempting
   * any partial/fallback match — see the project-path design spec.
   */
  let matchedProject = $derived.by(() => {
    if (parsed.project) {
      if (parsed.project.includes("/")) {
        const resolved = findProjectByPath(projectsState.items, parsed.project.split("/"));
        if (resolved) return resolved;
      } else {
        const typed = parsed.project.toLowerCase();
        const resolved = projectsState.items.find((project) => project.name.toLowerCase() === typed);
        if (resolved) return resolved;
      }
    }
    if (projectFilter) {
      return projectsState.items.find((project) => project.id === projectFilter);
    }
    return projectsState.items.find((project) => project.id === settingsState.current?.default_project_id);
  });

  /** `matchedProject` itself, then its ancestors nearest-first — settings/defaults inherit through this full chain, mirroring the backend's `self_and_ancestors` walk. Empty when no project is matched yet (e.g. before projects finish loading). */
  let projectChain = $derived(matchedProject ? selfAndAncestors(projectsState.items, matchedProject.id) : []);

  /** Root-first ancestor path for `matchedProject` (e.g. "Work/Client A"), shown in the preview instead of a bare leaf name — disambiguates same-named subprojects under different parents. */
  let matchedProjectPath = $derived(
    matchedProject ? projectPath(projectsState.items, matchedProject.id) : undefined,
  );

  /** The effective project, priority, status, tags, due, scheduled, and estimated time this task will be created with. */
  let preview = $derived(
    resolveTaskPreview({
      parsed: effectiveParsed,
      projectFilter,
      defaultProjectName,
      globalDefaults,
      projectDefaultsChain: projectChain.map((p) => p.defaults),
      matchedProjectName: matchedParentTask ? `Subtask of ${matchedParentTask.title}` : matchedProjectPath,
      priorities,
      statuses,
      projectBoardDefaultStatusChain: projectChain.map((p) => p.board.default_status),
    }),
  );

  /**
   * When recurring, the series' due *rule* (e.g. "Same day as each
   * occurrence", "3 days after each occurrence", "Every Friday") rather
   * than a single resolved date — a single date for occurrence #1 isn't
   * useful for understanding what every future occurrence's due date will
   * be, which is exactly what every occurrence actually gets (see
   * `resolveSeriesDueRule`'s own doc comment). Computed the same way
   * `handleSubmit` computes what's actually sent, so the preview never
   * shows something different from what gets created.
   */
  let seriesDueRule = $derived(
    effectiveParsed.recurrence
      ? resolveSeriesDueRule(effectiveParsed.due, effectiveParsed.dueRule, preview.scheduledDate)
      : undefined,
  );

  /**
   * The date each calendar popup highlights as "selected": the manual
   * override once set, else the fully resolved preview value — never the
   * raw `parsed.due`/`parsed.scheduled`, which is `undefined` whenever no
   * quick-add token was typed even if a project/global default applies (the
   * same `parsed`-vs-`preview` distinction the estimated-time sync effect
   * below has to get right).
   */
  let dueSelectedForPicker = $derived.by(() => {
    if (dueManuallySet) return draftDueOverride !== "none" ? draftDueOverride : undefined;
    return preview.due !== "Never" ? preview.due : undefined;
  });
  let scheduledSelectedForPicker = $derived(scheduledManuallySet ? draftScheduledOverride : preview.scheduledDate);

  /**
   * Mirrors `preview.estimatedMinutes` (quick-add token, else project
   * default, else global default) into the editable boxes, live, as long as
   * the user hasn't manually overridden them — this is what makes a
   * configured default actually show up here instead of leaving the boxes
   * looking empty/zero until you type something.
   */
  $effect(() => {
    if (estimateManuallySet) return;
    const resolved =
      preview.estimatedMinutes !== undefined ? hoursAndMinutesFromMinutes(preview.estimatedMinutes) : undefined;
    draftEstimatedHours = resolved?.hours;
    draftEstimatedMinutes = resolved?.minutes;
  });

  /** Mirrors `preview.priorityId`/`preview.tags` into the editable controls, live, the same way the estimated-time boxes above mirror `preview.estimatedMinutes` — until manually touched. */
  $effect(() => {
    if (priorityManuallySet) return;
    draftPriorityOverride = preview.priorityId;
  });
  $effect(() => {
    if (tagsManuallySet) return;
    draftTagsInput = formatTags(preview.tags);
  });

  let previewProjectColor = $derived(resolveProjectColor(matchedProject?.id, projectsState.items));
  // Falls back to the standard ink color for very light project colors (e.g.
  // a pale cream), which would otherwise be illegible as text — see TaskCard's
  // `projectChipTextColor` for the same fix applied to the board chip.
  let previewProjectTextColor = $derived(
    isLightColor(previewProjectColor) ? "var(--color-ink)" : previewProjectColor,
  );

  let activeToken: ActiveToken | undefined = $state();
  let activeSubtaskToken: ActiveSubtaskToken | undefined = $state();
  let suggestions: string[] = $state([]);
  let activeSuggestionIndex = $state(0);

  $effect(() => {
    if (!dialogEl) return;
    if (open) {
      if (!dialogEl.open) {
        title = "";
        draftPriorityOverride = undefined;
        priorityManuallySet = false;
        draftTagsInput = "";
        tagsManuallySet = false;
        tagSuggestions = [];
        draftEstimatedHours = undefined;
        draftEstimatedMinutes = undefined;
        estimateManuallySet = false;
        draftDueOverride = undefined;
        dueManuallySet = false;
        draftScheduledOverride = undefined;
        scheduledManuallySet = false;
        draftRecurrenceOverride = undefined;
        recurrenceManuallySet = false;
        draftDueRuleOverride = undefined;
        dueRuleManuallySet = false;
        parentSeriesInfo = undefined;
        loadedParentSeriesId = undefined;
        suggestions = [];
        activeToken = undefined;
        activeSubtaskToken = undefined;
        dialogEl.showModal();
        inputEl?.focus();
        inputEl?.setSelectionRange(title.length, title.length);
      }
    } else if (dialogEl.open) {
      dialogEl.close();
    }
  });

  // Past this many tags, the dropdown stops helping at all (no browse-all,
  // no filtering either) — the user types the tag's full name from memory.
  // A long tag list is unwieldy to browse, unlike the much shorter
  // priority/status/project lists this threshold doesn't apply to.
  const MAX_LISTABLE_TAGS = 10;

  /** Recomputes the active token and its suggestions from the input's current value and cursor. */
  function updateSuggestions() {
    if (!inputEl) return;

    const value = inputEl.value;
    const cursor = inputEl.selectionStart ?? value.length;
    const token = findActiveToken(value, cursor);
    activeToken = token;

    if (!token) {
      // `findActiveToken`'s `#+!@` patterns and `findActiveSubtaskToken`'s
      // word-based `sub ` pattern never both match the same cursor
      // position, so checking this one only when the other found nothing
      // is unambiguous, not a priority choice between two real matches.
      activeSubtaskToken = findActiveSubtaskToken(value, cursor);
      suggestions = activeSubtaskToken
        ? subtaskNameSuggestions(
            allTasks,
            activeSubtaskToken.text,
            settingsState.current?.done_status,
            settingsState.current?.cancelled_status,
          )
        : [];
      activeSuggestionIndex = 0;
      return;
    }
    activeSubtaskToken = undefined;

    if (token.prefix === "#" && tagsState.items.length >= MAX_LISTABLE_TAGS) {
      suggestions = [];
      activeSuggestionIndex = 0;
      return;
    }

    if (token.prefix === "+") {
      // Handles its own collision-disambiguation and "/"-drill-down — see
      // `projectPathSuggestions`'s own doc comment. Already behaves
      // correctly for an empty `token.text` (browses every project), so no
      // separate bare-prefix branch is needed here unlike the other prefixes.
      suggestions = projectPathSuggestions(projectsState.items, token.text);
      activeSuggestionIndex = 0;
      return;
    }

    const options = token.prefix === "#" ? tagsState.items : token.prefix === "!" ? priorityOptions : statusOptions;

    // A bare prefix (no text yet) browses every option — `filterSuggestions`
    // itself always returns nothing for an empty prefix, so that case is
    // handled directly here instead.
    suggestions = token.text === "" ? options.slice(0, MAX_SUGGESTIONS) : filterSuggestions(options, token.text);
    activeSuggestionIndex = 0;
  }

  async function selectSuggestion(suggestion: string) {
    if (activeSubtaskToken) {
      const result = applySubtaskTokenSuggestion(title, activeSubtaskToken, suggestion);
      title = result.value;
      suggestions = [];
      activeSubtaskToken = undefined;

      await tick();
      inputEl?.setSelectionRange(result.cursor, result.cursor);
      inputEl?.focus();
      return;
    }

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
        activeSubtaskToken = undefined;
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
    // `matchedParentTask` takes precedence: ensuring its subtask container
    // exists (creating it on first use) is the one piece of resolution
    // that can't happen in the live preview (see `matchedParentTask`'s own
    // doc comment) — only here, right before actually submitting.
    // Otherwise `matchedProject` (see its own `$derived` above) already
    // resolves the effective project — by +Project name, by this dialog's
    // id-based projectFilter, or by the default project — reused here
    // directly as the actual submission's id, rather than re-deriving it.
    const projectId = matchedParentTask
      ? (await ensureSubtaskContainer(matchedParentTask.id)).id
      : matchedProject?.id;
    if (effectiveParsed.recurrence) {
      await onSubmit({
        ...effectiveParsed,
        project: preview.project,
        projectId,
        dueRule: resolveSeriesDueRule(effectiveParsed.due, effectiveParsed.dueRule, preview.scheduledDate),
      });
    } else {
      await onSubmit({
        ...effectiveParsed,
        project: preview.project,
        projectId,
        due: resolveNonRecurringDue(effectiveParsed.due, effectiveParsed.dueRule, preview.scheduledDate),
        dueRule: undefined,
      });
    }
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
        placeholder="Task title"
        aria-label="New task title"
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
      <details class="syntax-help">
        <summary aria-label="Quick-add syntax help" title="Quick-add syntax help">?</summary>
        <div class="syntax-help-content" role="note">
          <p class="syntax-help-title">Quick-add syntax</p>
          <ul>
            <li><code>#tag</code> — add a tag</li>
            <li><code>+Project</code> — set the project</li>
            <li><code>high</code> / <code>medium</code> / <code>low</code> (or <code>!h</code> / <code>!m</code> / <code>!l</code>) — set priority</li>
            <li>
              <code>due …</code> / <code>sch …</code> — due / scheduled date: <code>today</code>,
              <code>tomorrow</code>, <code>YYYY-MM-DD</code>, a weekday, "next weekday", or "month day[ year]"
            </li>
            <li><code>due na</code> — never due</li>
            <li><code>@status</code> — set the status, e.g. <code>@do</code></li>
            <li>
              <code>est &lt;n&gt;h &lt;n&gt;m</code> — estimated time, e.g. <code>est 1h 30m</code>
              (<code>est</code> is optional; <code>m</code> is optional once <code>h</code> is present)
            </li>
          </ul>
          <p class="syntax-help-note">
            "next weekday" skips the upcoming one — e.g. "next monday" is a week later than "monday".
          </p>
        </div>
      </details>
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
        <span class="syntax-hint">+Project</span>
        <dd class="filled" style={`color: ${previewProjectTextColor}`}>
          {preview.project}
        </dd>
      </div>
      <div class="field-row">
        <dt>Priority</dt>
        <span class="syntax-hint">high / medium / low</span>
        <dd class="priority-editable">
          <select
            aria-label="Priority"
            value={draftPriorityOverride ?? preview.priorityId}
            onchange={(event) => {
              draftPriorityOverride = event.currentTarget.value;
              handlePriorityChange();
            }}
            style={`color: ${priorityColor(priorities, draftPriorityOverride ?? preview.priorityId)}`}
          >
            {#each sortedPriorities(priorities) as level (level.id)}
              <option value={level.id}>{level.label}</option>
            {/each}
          </select>
        </dd>
      </div>
      <div class="field-row">
        <dt>Status</dt>
        <span class="syntax-hint">@status</span>
        <dd class="filled">{statusLabel(statuses, preview.statusId)}</dd>
      </div>
      <div class="field-row">
        <dt>Tags</dt>
        <span class="syntax-hint">{matchedParentTask ? "locked to parent" : "#tag"}</span>
        {#if matchedParentTask}
          <dd class="filled">{formatTags(matchedParentTask.tags) || "—"}</dd>
        {:else}
          <dd class="tags-editable">
            <input
              type="text"
              value={draftTagsInput}
              placeholder="comma, separated"
              role="combobox"
              aria-label="Tags"
              aria-expanded={tagSuggestions.length > 0}
              aria-controls="add-task-tags-suggestions"
              aria-autocomplete="list"
              aria-activedescendant={tagSuggestions.length > 0
                ? `add-task-tags-suggestions-option-${tagSuggestionIndex}`
                : undefined}
              oninput={(event) => {
                draftTagsInput = event.currentTarget.value;
                handleTagsInput();
              }}
              onkeydown={handleTagsKeydown}
              onblur={() => (tagSuggestions = [])}
            />
            <Autocomplete
              id="add-task-tags-suggestions"
              items={tagSuggestions}
              activeIndex={tagSuggestionIndex}
              onSelect={selectTagSuggestion}
              onHover={(index) => (tagSuggestionIndex = index)}
              prefix="#"
            />
          </dd>
        {/if}
      </div>
      <div class="field-row">
        <dt>Scheduled</dt>
        <span class="syntax-hint">{matchedParentTask ? "locked to parent" : "sch <phrase>"}</span>
        <dd class="date-value" class:filled={!!preview.scheduled}>
          <span>{preview.scheduled ?? "—"}</span>
          {#if !matchedParentTask}
            <DatePickerPopover
              selected={scheduledSelectedForPicker}
              triggerLabel="Pick scheduled date"
              clearLabel="Clear"
              onSelect={handleScheduledSelect}
              onClear={handleScheduledClear}
            />
          {/if}
        </dd>
      </div>
      <div class="field-row">
        <dt>Due</dt>
        <span class="syntax-hint">
          {matchedParentTask
            ? "locked to parent"
            : effectiveParsed.recurrence
              ? "due <phrase> / due in <n> days / due <weekday>s"
              : "due <phrase> / due na"}
        </span>
        <dd class="date-value" class:filled={!!preview.due}>
          <span>
            {#if seriesDueRule}
              {formatDueRule(seriesDueRule)}
            {:else if preview.due && preview.due !== "Never"}
              {@const dueDisplay = formatDueDateDisplay(preview.due, new Date(), displayState.nlDueDates)}
              <span class:due-today={dueDisplay?.variant === "today"} class:due-tomorrow={dueDisplay?.variant === "tomorrow"} class:due-overdue={dueDisplay?.variant === "overdue"}>
                {dueDisplay?.label ?? preview.due}
              </span>
            {:else}
              {preview.due ?? "—"}
            {/if}
          </span>
          {#if !matchedParentTask}
            <DatePickerPopover
              selected={dueSelectedForPicker}
              triggerLabel="Pick due date"
              clearLabel="Never"
              onSelect={handleDueSelect}
              onClear={handleDueNever}
            />
          {/if}
        </dd>
      </div>
      <div class="field-row">
        <dt>Estimated time</dt>
        <span class="syntax-hint">est &lt;n&gt;h &lt;n&gt;m</span>
        <dd class="estimate-editable">
          <input
            type="number"
            min="0"
            step="1"
            placeholder="0"
            bind:value={draftEstimatedHours}
            oninput={handleEstimateInput}
            onblur={normalizeEstimateDraft}
            aria-label="Estimated hours"
          />
          h
          <input
            type="number"
            min="0"
            step="1"
            placeholder="0"
            bind:value={draftEstimatedMinutes}
            oninput={handleEstimateInput}
            onblur={normalizeEstimateDraft}
            aria-label="Estimated minutes"
          />
          m
        </dd>
      </div>
      <div class="field-row">
        <dt>Recurrence</dt>
        <span class="syntax-hint">
          {matchedParentTask ? "locked to parent" : "every <phrase> [until <phrase>]"}
        </span>
        <dd class="date-value" class:filled={!!effectiveParsed.recurrence}>
          <span>
            {#if effectiveParsed.recurrence}
              {formatRecurrenceFrequency(effectiveParsed.recurrence.frequency)}{effectiveParsed.recurrence.endDate
                ? ` until ${effectiveParsed.recurrence.endDate}`
                : ""}
            {:else}
              —
            {/if}
          </span>
          {#if !matchedParentTask}
            <RecurrenceBuilderDialog
              value={effectiveParsed.recurrence
                ? { frequency: effectiveParsed.recurrence.frequency, endDate: effectiveParsed.recurrence.endDate, dueRule: seriesDueRule }
                : undefined}
              triggerLabel="Build"
              onApply={handleRecurrenceApply}
              onClear={handleRecurrenceClear}
            />
          {/if}
        </dd>
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
    padding-right: calc(var(--space-md) + 1.75rem);
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

  .syntax-help {
    position: absolute;
    top: 50%;
    right: var(--space-sm);
    transform: translateY(-50%);
  }

  .syntax-help summary {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.5rem;
    height: 1.5rem;
    border-radius: var(--radius-pill);
    border: 1px solid var(--color-border);
    background: var(--color-canvas);
    color: var(--color-ink-muted);
    font-size: var(--text-xs);
    font-weight: 700;
    cursor: pointer;
    list-style: none;
    transition:
      color var(--duration-fast) var(--ease-out-expo),
      border-color var(--duration-fast) var(--ease-out-expo);
  }

  .syntax-help summary::-webkit-details-marker {
    display: none;
  }

  .syntax-help summary::marker {
    content: "";
  }

  .syntax-help summary:hover,
  .syntax-help[open] summary {
    color: var(--color-ink);
    border-color: var(--color-accent);
  }

  .syntax-help summary:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .syntax-help-content {
    position: absolute;
    top: calc(100% + var(--space-2xs));
    right: 0;
    z-index: 20;
    width: 19rem;
    max-width: calc(100vw - 2 * var(--space-lg));
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface-raised);
    box-shadow: var(--shadow-lg);
    font-size: var(--text-xs);
    color: var(--color-ink-muted);
  }

  .syntax-help-content ul {
    margin: var(--space-2xs) 0;
    padding-left: var(--space-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-3xs);
  }

  .syntax-help-title {
    margin: 0;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink);
  }

  .syntax-help-note {
    margin: var(--space-2xs) 0 0;
  }

  .syntax-help-content code {
    padding: 0 0.2em;
    border-radius: var(--radius-sm);
    background: var(--color-canvas);
    color: var(--color-ink);
    font-size: 0.9em;
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
    display: grid;
    /* `minmax(0, 1fr)` on both flexible columns, not bare `1fr`/`auto` — a
       bare `1fr` won't shrink below its content's min-content size, and a
       bare `auto` claims as much space as its content needs with no
       upper bound. Either one can starve the other: a long syntax hint
       (e.g. the recurring-task variant) used to force the grid wider than
       the dialog, and a long value (e.g. a multi-weekday recurrence with
       an end date) used to squeeze the hint down to ~1 character instead
       of sharing space with it. `minmax(0, 1fr)` on both lets each one
       wrap onto multiple lines and shrink to fit instead. */
    grid-template-columns: auto minmax(0, 1fr) minmax(0, 1fr);
    align-items: baseline;
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

  .syntax-hint {
    font-family: monospace;
    font-size: var(--text-xs);
    color: var(--color-ink-faint);
    text-align: center;
    /* No `white-space: nowrap`: most hints comfortably fit on one line
       within their `minmax(0, 1fr)` column anyway, so this has no visual
       effect for them — but the longer recurring-task due-hint now wraps
       onto a second line instead of overflowing. */
    overflow-wrap: break-word;
  }

  .field-row dd {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--color-ink-faint);
    text-align: right;
    /* A deep project path (e.g. "Work/ClientA/Phase1/Deep1") has no
       whitespace anywhere for the browser to wrap at by default, so
       without this it overflows its `minmax(0, 1fr)` column and runs
       offscreen instead of wrapping — same fix as `.syntax-hint` above. */
    overflow-wrap: break-word;
  }

  .field-row dd.filled {
    color: var(--color-ink);
    font-weight: 600;
  }

  .field-row dd.date-value {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-2xs);
  }

  /* A flex item's default min-width is `auto` (its content's min-content
     size), the same shrink-resistance issue as a bare grid `1fr`/`auto`
     track — without this, the text span here could refuse to shrink/wrap
     and overflow its now-constrained `.date-value` column instead. */
  .field-row dd.date-value > span {
    min-width: 0;
  }

  .field-row dd.estimate-editable {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-3xs);
  }

  .field-row dd.estimate-editable input {
    /* Wide enough for a comfortable 3-digit value (e.g. "100" hours)
       without the text crowding the field's edge. */
    width: 4rem;
    padding: var(--space-3xs) var(--space-2xs);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
    font: inherit;
    font-size: var(--text-sm);
    text-align: right;
  }

  .field-row dd.estimate-editable input:focus-visible {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-soft);
    outline: none;
  }

  .field-row dd.priority-editable {
    display: flex;
    justify-content: flex-end;
  }

  .field-row dd.priority-editable select {
    padding: var(--space-3xs) var(--space-2xs);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    font: inherit;
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .field-row dd.priority-editable select:focus-visible {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-soft);
    outline: none;
  }

  .field-row dd.tags-editable {
    position: relative;
    text-align: left;
  }

  .field-row dd.tags-editable input {
    width: 100%;
    padding: var(--space-3xs) var(--space-2xs);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
    font: inherit;
    font-size: var(--text-sm);
  }

  .field-row dd.tags-editable input:focus-visible {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-soft);
    outline: none;
  }

  .due-today {
    color: var(--color-urgent);
  }

  .due-overdue {
    color: var(--color-overdue);
  }

  .due-tomorrow {
    color: var(--color-soon);
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
