<script lang="ts">
  import { updateProject } from "$lib/api";
  import {
    legibleInkColor,
    NEON_CARD_CHROMA_BOOST,
    WEEK_BAR_CHROMA_BOOST,
    neonCardColor,
    type InkMode,
  } from "$lib/colorPresets";
  import { getErrorMessage } from "$lib/errors";
  import { boardsEqual, effectiveBoardStatuses } from "$lib/projectBoardSettings";
  import { projectsState, refreshProjects } from "$lib/projects.svelte";
  import { ancestorsOf, descendantsOf, selfAndAncestors } from "$lib/projectTree";
  import { settingsState } from "$lib/settings.svelte";
  import { FALLBACK_STATUSES, sortedStatuses, statusColor, statusLabel } from "$lib/statuses.svelte";
  import type { Project } from "$lib/types";

  interface Props {
    project: Project;
  }

  let { project }: Props = $props();

  /** Every project this one could be moved under, without creating a cycle: everything except itself and its own descendants. */
  let parentCandidates = $derived(
    projectsState.items.filter(
      (p) => p.id !== project.id && !descendantsOf(projectsState.items, project.id).some((d) => d.id === p.id),
    ),
  );

  /** This project's own board, then its ancestors' boards, nearest-first — the chain `effectiveBoardStatuses`/etc. resolve through. */
  let boardChain = $derived(selfAndAncestors(projectsState.items, project.id).map((p) => p.board));
  /** The nearest *ancestor* (not including this project itself) whose board customizes its status subset, if any — used to label an inherited (not self-set) value. */
  let statusesInheritedFrom = $derived(
    project.board.statuses.length === 0
      ? ancestorsOf(projectsState.items, project.id).find((p) => p.board.statuses.length > 0)?.name
      : undefined,
  );
  let cardLightnessInheritedFrom = $derived(
    project.board.card_lightness === undefined
      ? ancestorsOf(projectsState.items, project.id).find((p) => p.board.card_lightness !== undefined)?.name
      : undefined,
  );
  let barLightnessInheritedFrom = $derived(
    project.board.bar_lightness === undefined
      ? ancestorsOf(projectsState.items, project.id).find((p) => p.board.bar_lightness !== undefined)?.name
      : undefined,
  );
  let inkModeInheritedFrom = $derived(
    project.board.ink_mode === undefined
      ? ancestorsOf(projectsState.items, project.id).find((p) => p.board.ink_mode !== undefined)?.name
      : undefined,
  );

  let statuses = $derived(sortedStatuses(settingsState.current?.statuses ?? FALLBACK_STATUSES));
  let allStatusIds = $derived(statuses.map((status) => status.id));

  let baselineStatuses = $derived(effectiveBoardStatuses(boardChain, allStatusIds));
  let baselineDefault = $derived(project.board.default_status ?? "");
  let draftParentId = $state("");
  let baselineParentId = $derived(project.parent_id ?? "");
  /** "" = inherit the global default, "true"/"false" = explicit override. */
  let baselineShowPreviousWeeks = $derived(
    project.board.show_previous_weeks === undefined ? "" : String(project.board.show_previous_weeks),
  );
  /** "" = inherit the global default, "true"/"false" = explicit override. */
  let baselineShowSubprojectTasks = $derived(
    project.board.show_subproject_tasks === undefined ? "" : String(project.board.show_subproject_tasks),
  );
  let baselineCardLightnessOverride = $derived(project.board.card_lightness !== undefined);
  let baselineCardLightness = $derived(
    Math.round((project.board.card_lightness ?? settingsState.current?.card_lightness ?? 0.5) * 100),
  );
  let baselineBarLightnessOverride = $derived(project.board.bar_lightness !== undefined);
  let baselineBarLightness = $derived(
    Math.round((project.board.bar_lightness ?? settingsState.current?.bar_lightness ?? 0.38) * 100),
  );
  let baselineInkModeOverride = $derived(project.board.ink_mode !== undefined);
  let baselineInkMode = $derived(project.board.ink_mode ?? settingsState.current?.ink_mode ?? "auto");

  let draftStatuses = $state<string[]>([]);
  let draftDefault = $state("");
  let draftShowPreviousWeeks = $state("");
  let draftShowSubprojectTasks = $state("");
  let draftCardLightnessOverride = $state(false);
  let draftCardLightness = $state(50);
  let draftBarLightnessOverride = $state(false);
  let draftBarLightness = $state(38);
  let draftInkModeOverride = $state(false);
  let draftInkMode: InkMode = $state("auto");
  let initialized = $state(false);
  let errorMessage = $state("");
  let isSaving = $state(false);

  /** Seeds the draft from the project's board once; later edits live only in the draft. */
  $effect(() => {
    if (settingsState.current && !initialized) {
      draftParentId = baselineParentId;
      draftStatuses = [...baselineStatuses];
      draftDefault = baselineDefault;
      draftShowPreviousWeeks = baselineShowPreviousWeeks;
      draftShowSubprojectTasks = baselineShowSubprojectTasks;
      draftCardLightnessOverride = baselineCardLightnessOverride;
      draftCardLightness = baselineCardLightness;
      draftBarLightnessOverride = baselineBarLightnessOverride;
      draftBarLightness = baselineBarLightness;
      draftInkModeOverride = baselineInkModeOverride;
      draftInkMode = baselineInkMode;
      initialized = true;
    }
  });

  let isDirty = $derived(
    draftParentId !== baselineParentId ||
    !boardsEqual(
      { statuses: draftStatuses, default_status: draftDefault || undefined },
      { statuses: baselineStatuses, default_status: project.board.default_status },
    ) ||
      draftShowPreviousWeeks !== baselineShowPreviousWeeks ||
      draftShowSubprojectTasks !== baselineShowSubprojectTasks ||
      draftCardLightnessOverride !== baselineCardLightnessOverride ||
      (draftCardLightnessOverride && draftCardLightness !== baselineCardLightness) ||
      draftBarLightnessOverride !== baselineBarLightnessOverride ||
      (draftBarLightnessOverride && draftBarLightness !== baselineBarLightness) ||
      draftInkModeOverride !== baselineInkModeOverride ||
      (draftInkModeOverride && draftInkMode !== baselineInkMode),
  );

  /** The ink mode that would actually apply if saved right now, for the preview swatches below. */
  let effectiveInkMode = $derived(
    draftInkModeOverride ? draftInkMode : settingsState.current?.ink_mode ?? "auto",
  );
  let cardPreviewBg = $derived(neonCardColor(project.color, draftCardLightness / 100, NEON_CARD_CHROMA_BOOST));
  let cardPreviewText = $derived(legibleInkColor(cardPreviewBg, effectiveInkMode));
  let barPreviewBg = $derived(neonCardColor(project.color, draftBarLightness / 100, WEEK_BAR_CHROMA_BOOST));
  let barPreviewText = $derived(legibleInkColor(barPreviewBg, effectiveInkMode));

  let availableStatusIds = $derived(allStatusIds.filter((id) => !draftStatuses.includes(id)));

  function moveUp(index: number) {
    if (index === 0) return;
    const next = [...draftStatuses];
    [next[index - 1], next[index]] = [next[index], next[index - 1]];
    draftStatuses = next;
  }

  function moveDown(index: number) {
    if (index === draftStatuses.length - 1) return;
    moveUp(index + 1);
  }

  function removeStatus(id: string) {
    if (draftStatuses.length <= 1) return;
    draftStatuses = draftStatuses.filter((statusId) => statusId !== id);
    if (draftDefault === id) draftDefault = "";
  }

  function addStatus(id: string) {
    draftStatuses = [...draftStatuses, id];
  }

  function discardChanges() {
    draftParentId = baselineParentId;
    draftStatuses = [...baselineStatuses];
    draftDefault = baselineDefault;
    draftShowPreviousWeeks = baselineShowPreviousWeeks;
    draftShowSubprojectTasks = baselineShowSubprojectTasks;
    draftCardLightnessOverride = baselineCardLightnessOverride;
    draftCardLightness = baselineCardLightness;
    draftBarLightnessOverride = baselineBarLightnessOverride;
    draftBarLightness = baselineBarLightness;
    draftInkModeOverride = baselineInkModeOverride;
    draftInkMode = baselineInkMode;
    errorMessage = "";
  }

  async function save() {
    isSaving = true;
    try {
      await updateProject({
        ...project,
        parent_id: draftParentId || undefined,
        board: {
          statuses: draftStatuses,
          default_status: draftDefault || undefined,
          show_previous_weeks: draftShowPreviousWeeks === "" ? undefined : draftShowPreviousWeeks === "true",
          show_subproject_tasks:
            draftShowSubprojectTasks === "" ? undefined : draftShowSubprojectTasks === "true",
          card_lightness: draftCardLightnessOverride ? draftCardLightness / 100 : undefined,
          bar_lightness: draftBarLightnessOverride ? draftBarLightness / 100 : undefined,
          ink_mode: draftInkModeOverride ? draftInkMode : undefined,
        },
      });
      await refreshProjects();
      errorMessage = "";
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to save board settings");
    } finally {
      isSaving = false;
    }
  }
</script>

<section aria-labelledby="parent-heading">
  <div class="section-header">
    <h2 id="parent-heading">Parent project</h2>
  </div>
  <p class="description">Move this project under a different parent, or leave it at the top level.</p>
  <div class="field">
    <label for="parent-project">Parent</label>
    <select id="parent-project" bind:value={draftParentId}>
      <option value="">No parent (top level)</option>
      {#each parentCandidates as candidate (candidate.id)}
        <option value={candidate.id}>{candidate.name}</option>
      {/each}
    </select>
  </div>
</section>

<section aria-labelledby="board-heading">
  <div class="section-header">
    <h2 id="board-heading">Board columns</h2>
  </div>
  <p class="description">
    These statuses appear as columns on this project's board, in this order. Tasks with any other
    status appear in a trailing "Other" column.
  </p>

  <ul class="status-list">
    {#each draftStatuses as id, index (id)}
      <li class="status-row">
        <div class="rank-controls">
          <button
            type="button"
            disabled={index === 0}
            onclick={() => moveUp(index)}
            aria-label={`Move ${statusLabel(statuses, id)} up`}
          >
            ▲
          </button>
          <button
            type="button"
            disabled={index === draftStatuses.length - 1}
            onclick={() => moveDown(index)}
            aria-label={`Move ${statusLabel(statuses, id)} down`}
          >
            ▼
          </button>
        </div>

        <span class="swatch" style="background: {statusColor(statuses, id)}" aria-hidden="true"
        ></span>
        <span class="status-name">{statusLabel(statuses, id)}</span>

        <button
          type="button"
          class="secondary"
          disabled={draftStatuses.length <= 1}
          title={draftStatuses.length <= 1
            ? "At least one column is required"
            : `Move ${statusLabel(statuses, id)} to Other statuses`}
          aria-label={`Remove ${statusLabel(statuses, id)} from this board`}
          onclick={() => removeStatus(id)}
        >
          Remove
        </button>
      </li>
    {/each}
  </ul>

  {#if availableStatusIds.length > 0}
    <h3 id="other-statuses-heading">Other statuses</h3>
    <ul class="status-list">
      {#each availableStatusIds as id (id)}
        <li class="status-row">
          <span class="swatch" style="background: {statusColor(statuses, id)}" aria-hidden="true"
          ></span>
          <span class="status-name">{statusLabel(statuses, id)}</span>
          <button
            type="button"
            class="secondary"
            aria-label={`Add ${statusLabel(statuses, id)} to this board`}
            onclick={() => addStatus(id)}
          >
            Add to board
          </button>
        </li>
      {/each}
    </ul>
  {/if}
  {#if statusesInheritedFrom}
    <p class="inherited-note">Inherited from {statusesInheritedFrom}.</p>
  {/if}

  <div class="field">
    <label for="default-status">Default status for new tasks in this project</label>
    <select id="default-status" bind:value={draftDefault}>
      <option value="">Use global default</option>
      {#each draftStatuses as id (id)}
        <option value={id}>{statusLabel(statuses, id)}</option>
      {/each}
    </select>
  </div>

  <div class="field">
    <label for="show-previous-weeks">Week view "Previous" column</label>
    <select id="show-previous-weeks" bind:value={draftShowPreviousWeeks}>
      <option value="">Use global default</option>
      <option value="true">Always show</option>
      <option value="false">Always hide</option>
    </select>
    <p class="hint">
      Overrides the global Display setting for this project's Week view only.
    </p>
  </div>

  <div class="field">
    <label for="show-subproject-tasks">Show subprojects' tasks</label>
    <select id="show-subproject-tasks" bind:value={draftShowSubprojectTasks}>
      <option value="">Use global default</option>
      <option value="true">Roll up subprojects' tasks here</option>
      <option value="false">Only this project's own tasks</option>
    </select>
    <p class="hint">
      When on, viewing this project's board/week/calendar also shows every descendant subproject's
      tasks (origin-labeled). Off by default — a subtask's own glued nested row on its parent
      task's card is unaffected either way.
    </p>
  </div>

  <div class="field lightness-field">
    <label class="checkbox-row" for="card-lightness-override">
      <input id="card-lightness-override" type="checkbox" bind:checked={draftCardLightnessOverride} />
      Override Kanban card lightness for this project
    </label>
    {#if draftCardLightnessOverride}
      <div class="control-with-preview">
        <input
          type="range"
          min="0"
          max="100"
          step="1"
          bind:value={draftCardLightness}
          aria-label="Kanban card lightness"
        />
        <span class="control-value">{draftCardLightness}%</span>
        <span
          class="preview-swatch"
          style="background: {cardPreviewBg}; color: {cardPreviewText}"
          aria-hidden="true"
        >
          Sample
        </span>
      </div>
    {/if}
  </div>
  {#if cardLightnessInheritedFrom}
    <p class="inherited-note">Inherited from {cardLightnessInheritedFrom}.</p>
  {/if}

  <div class="field lightness-field">
    <label class="checkbox-row" for="bar-lightness-override">
      <input id="bar-lightness-override" type="checkbox" bind:checked={draftBarLightnessOverride} />
      Override week/calendar bar lightness for this project
    </label>
    {#if draftBarLightnessOverride}
      <div class="control-with-preview">
        <input
          type="range"
          min="0"
          max="100"
          step="1"
          bind:value={draftBarLightness}
          aria-label="Week/calendar bar lightness"
        />
        <span class="control-value">{draftBarLightness}%</span>
        <span
          class="preview-swatch"
          style="background: {barPreviewBg}; color: {barPreviewText}"
          aria-hidden="true"
        >
          Sample
        </span>
      </div>
    {/if}
  </div>
  {#if barLightnessInheritedFrom}
    <p class="inherited-note">Inherited from {barLightnessInheritedFrom}.</p>
  {/if}

  <div class="field lightness-field">
    <label class="checkbox-row" for="ink-mode-override">
      <input id="ink-mode-override" type="checkbox" bind:checked={draftInkModeOverride} />
      Override card &amp; bar text color for this project
    </label>
    {#if draftInkModeOverride}
      <select bind:value={draftInkMode} aria-label="Card & bar text color">
        <option value="auto">Auto</option>
        <option value="white">White</option>
        <option value="black">Black</option>
      </select>
    {/if}
  </div>
  {#if inkModeInheritedFrom}
    <p class="inherited-note">Inherited from {inkModeInheritedFrom}.</p>
  {/if}

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

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-md);
  }

  .section-header h2 {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
  }

  .description {
    margin: 0 0 var(--space-md);
    font-size: var(--text-sm);
    color: var(--color-ink-muted);
  }

  h3 {
    margin: var(--space-lg) 0 var(--space-md);
    font-size: var(--text-sm);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
  }

  .status-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    list-style: none;
    margin: 0 0 var(--space-md);
    padding: 0;
  }

  .status-row {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
  }

  .rank-controls {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex-shrink: 0;
  }

  .rank-controls button {
    width: 1.5rem;
    height: 1.25rem;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink-muted);
    font-size: var(--text-xs);
    line-height: 1;
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out-expo);
  }

  .rank-controls button:hover:not(:disabled) {
    background: var(--color-canvas);
    color: var(--color-ink);
  }

  .rank-controls button:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .swatch {
    width: 1.25rem;
    height: 1.25rem;
    border-radius: var(--radius-pill);
    border: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .status-name {
    flex: 1;
    min-width: 0;
    font-size: var(--text-sm);
  }

  button.secondary {
    flex-shrink: 0;
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
    font-weight: 600;
    font-size: var(--text-xs);
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out-expo);
  }

  button.secondary:hover:not(:disabled) {
    background: var(--color-canvas);
  }

  button.secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
    margin: var(--space-lg) 0;
    max-width: 20rem;
  }

  .field label {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-ink-muted);
  }

  .field select {
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
    font-size: var(--text-sm);
    box-shadow: var(--shadow-sm);
  }

  .field select:focus-visible {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-soft);
    outline: none;
  }

  .lightness-field {
    max-width: 28rem;
  }

  .checkbox-row {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    cursor: pointer;
  }

  .checkbox-row input[type="checkbox"] {
    width: 1.1rem;
    height: 1.1rem;
    accent-color: var(--color-accent);
    cursor: pointer;
  }

  .control-with-preview {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    margin-top: var(--space-2xs);
  }

  .control-with-preview input[type="range"] {
    flex: 1;
    accent-color: var(--color-accent);
  }

  .control-value {
    flex-shrink: 0;
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
    color: var(--color-ink-muted);
  }

  .preview-swatch {
    flex-shrink: 0;
    width: 4.5rem;
    padding: var(--space-2xs) 0;
    border-radius: var(--radius-md);
    text-align: center;
    font-size: var(--text-xs);
    font-weight: 600;
  }

  .hint {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--color-ink-faint);
  }

  .inherited-note {
    margin: calc(-1 * var(--space-2xs)) 0 0;
    font-size: var(--text-xs);
    color: var(--color-ink-muted);
    font-style: italic;
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
    padding: var(--space-sm) var(--space-lg);
    font-size: var(--text-base);
  }

  .actions button.secondary:hover:not(:disabled) {
    background: var(--color-canvas);
  }
</style>
