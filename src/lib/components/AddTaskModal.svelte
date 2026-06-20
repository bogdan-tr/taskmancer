<script lang="ts">
  import { tick } from "svelte";
  import {
    applyTokenSuggestion,
    filterSuggestions,
    findActiveToken,
    MAX_SUGGESTIONS,
    preferredSuggestionText,
    type ActiveToken,
  } from "$lib/autocomplete";
  import { isLightColor } from "$lib/colorPresets";
  import { displayState } from "$lib/displaySettings.svelte";
  import { formatDueDateDisplay } from "$lib/dueDateDisplay";
  import { hoursAndMinutesFromMinutes, minutesFromHoursAndMinutes, normalizeHoursMinutes } from "$lib/estimatedTime";
  import { parseTaskInput, type ParsedTaskInput } from "$lib/naturalLanguage";
  import { FALLBACK_PRIORITIES, priorityColor, priorityLabel, sortedPriorities } from "$lib/priorities.svelte";
  import { resolveProjectColor } from "$lib/projectColor";
  import { projectsState } from "$lib/projects.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import { FALLBACK_STATUSES, sortedStatuses, statusLabel } from "$lib/statuses.svelte";
  import { tagsState } from "$lib/tags.svelte";
  import { resolveTaskPreview } from "$lib/taskPreview";
  import Autocomplete from "./Autocomplete.svelte";
  import DatePickerPopover from "./DatePickerPopover.svelte";

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
  let parsed = $derived(parseTaskInput(title, undefined, knownPriorities, knownStatuses));

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

  function handleDueSelect(iso: string) {
    draftDueOverride = iso;
    dueManuallySet = true;
  }

  function handleDueNever() {
    draftDueOverride = "none";
    dueManuallySet = true;
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

  /** `parsed`, with the explicit estimated-time/due/scheduled controls overriding their quick-add tokens once manually set. */
  let effectiveParsed: ParsedTaskInput = $derived({
    ...parsed,
    due: dueManuallySet ? draftDueOverride : parsed.due,
    scheduled: scheduledManuallySet ? draftScheduledOverride : parsed.scheduled,
    estimatedMinutes: estimateManuallySet ? explicitEstimatedMinutes : parsed.estimatedMinutes,
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

  /** The effective project, priority, status, tags, due, scheduled, and estimated time this task will be created with. */
  let preview = $derived(
    resolveTaskPreview({
      parsed: effectiveParsed,
      projectFilter,
      defaultProjectName,
      globalDefaults,
      projectDefaults: matchedProject?.defaults,
      matchedProjectName: matchedProject?.name,
      priorities,
      statuses,
      projectBoardDefaultStatus: matchedProject?.board.default_status,
    }),
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

  let previewProjectColor = $derived(resolveProjectColor(preview.project, projectsState.items));
  // Falls back to the standard ink color for very light project colors (e.g.
  // a pale cream), which would otherwise be illegible as text — see TaskCard's
  // `projectChipTextColor` for the same fix applied to the board chip.
  let previewProjectTextColor = $derived(
    isLightColor(previewProjectColor) ? "var(--color-ink)" : previewProjectColor,
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
        draftEstimatedHours = undefined;
        draftEstimatedMinutes = undefined;
        estimateManuallySet = false;
        draftDueOverride = undefined;
        dueManuallySet = false;
        draftScheduledOverride = undefined;
        scheduledManuallySet = false;
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
      suggestions = [];
      return;
    }

    if (token.prefix === "#" && tagsState.items.length >= MAX_LISTABLE_TAGS) {
      suggestions = [];
      activeSuggestionIndex = 0;
      return;
    }

    const options =
      token.prefix === "#"
        ? tagsState.items
        : token.prefix === "!"
          ? priorityOptions
          : token.prefix === "@"
            ? statusOptions
            : projectNames;

    // A bare prefix (no text yet) browses every option — `filterSuggestions`
    // itself always returns nothing for an empty prefix, so that case is
    // handled directly here instead.
    suggestions = token.text === "" ? options.slice(0, MAX_SUGGESTIONS) : filterSuggestions(options, token.text);
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
    await onSubmit({ ...effectiveParsed, project: preview.project });
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
        <dd class="filled" style={`color: ${priorityColor(priorities, preview.priorityId)}`}>
          {priorityLabel(priorities, preview.priorityId)}
        </dd>
      </div>
      <div class="field-row">
        <dt>Status</dt>
        <span class="syntax-hint">@status</span>
        <dd class="filled">{statusLabel(statuses, preview.statusId)}</dd>
      </div>
      <div class="field-row">
        <dt>Tags</dt>
        <span class="syntax-hint">#tag</span>
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
        <span class="syntax-hint">sch &lt;phrase&gt;</span>
        <dd class="date-value" class:filled={!!preview.scheduled}>
          <span>{preview.scheduled ?? "—"}</span>
          <DatePickerPopover
            selected={scheduledSelectedForPicker}
            triggerLabel="Pick scheduled date"
            clearLabel="Clear"
            rightAlign
            onSelect={handleScheduledSelect}
            onClear={handleScheduledClear}
          />
        </dd>
      </div>
      <div class="field-row">
        <dt>Due</dt>
        <span class="syntax-hint">due &lt;phrase&gt; / due na</span>
        <dd class="date-value" class:filled={!!preview.due}>
          <span>
            {#if preview.due && preview.due !== "Never"}
              {@const dueDisplay = formatDueDateDisplay(preview.due, new Date(), displayState.nlDueDates)}
              <span class:due-today={dueDisplay?.variant === "today"} class:due-tomorrow={dueDisplay?.variant === "tomorrow"} class:due-overdue={dueDisplay?.variant === "overdue"}>
                {dueDisplay?.label ?? preview.due}
              </span>
            {:else}
              {preview.due ?? "—"}
            {/if}
          </span>
          <DatePickerPopover
            selected={dueSelectedForPicker}
            triggerLabel="Pick due date"
            clearLabel="Never"
            rightAlign
            onSelect={handleDueSelect}
            onClear={handleDueNever}
          />
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
    grid-template-columns: auto 1fr auto;
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
    white-space: nowrap;
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

  .field-row dd.date-value {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-2xs);
  }

  .field-row dd.estimate-editable {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-3xs);
  }

  .field-row dd.estimate-editable input {
    width: 3rem;
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
