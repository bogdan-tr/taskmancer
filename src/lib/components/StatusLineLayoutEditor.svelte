<script lang="ts">
  import { dndzone, type DndEvent } from "svelte-dnd-action";
  import {
    createStatusLayout,
    deleteStatusLayout,
    duplicateStatusLayout,
    listProjects,
    listStatusLayouts,
    updateProject,
    updateStatusLayout,
  } from "$lib/api";
  import type { WeekStartsOn } from "$lib/displaySettings.svelte";
  import { getErrorMessage } from "$lib/errors";
  import { refreshProjects } from "$lib/projects.svelte";
  import { refreshProjectStatusStats } from "$lib/statusLine.svelte";
  import { isKnownStatusLineStatId, statLabel } from "$lib/statusLineDisplay";
  import { ALL_STATUS_LINE_STAT_IDS, reorderStatIds, toggleStatId } from "$lib/statusLineLayoutEditor";
  import type { StatLayout } from "$lib/types";

  const FLIP_DURATION_MS = 150;

  interface Props {
    /**
     * The project whose status line this editor customizes — the picked/duplicated/forked layout
     * always becomes *this* project's own `board.status_line_layout_id` override when set.
     * Omit when opened from the global settings page; in that case, provide `onPickLayout` instead.
     */
    projectId?: string;
    /** The layout id currently rendered (`stats.effective_layout_id` for per-project; `settings.default_status_line_layout_id` for global). */
    currentLayoutId: string | undefined;
    weekStartsOn: WeekStartsOn;
    /**
     * Called when the user picks or creates a layout that should become "active". Overrides the
     * default per-project `board.status_line_layout_id` update — supply this from the global
     * settings page to update the global default instead.
     */
    onPickLayout?: (layoutId: string) => Promise<void>;
  }

  let { projectId, currentLayoutId, weekStartsOn, onPickLayout }: Props = $props();

  let open = $state(false);
  let dialogEl: HTMLDialogElement | undefined = $state();
  let layouts = $state<StatLayout[]>([]);
  let isLoading = $state(false);
  let isSaving = $state(false);
  let errorMessage = $state("");

  /** Draft `stat_ids` for the currently-applied layout, edited locally via toggle/drag and only sent to the backend on an explicit action below (`updateStatusLayout`/`createStatusLayout`) — accumulating edits client-side avoids a network round-trip per drag tick or checkbox click. */
  let draftStatIds = $state<{ id: string }[]>([]);
  let newLayoutNameDraft = $state("");
  let showSaveAsNewInput = $state(false);
  let showDuplicateInput = $state(false);
  let showDeleteConfirm = $state(false);

  let currentLayout = $derived(layouts.find((l) => l.id === currentLayoutId));

  async function openEditor() {
    open = true;
    isLoading = true;
    errorMessage = "";
    showSaveAsNewInput = false;
    showDuplicateInput = false;
    showDeleteConfirm = false;
    newLayoutNameDraft = "";
    try {
      layouts = await listStatusLayouts();
      const applied = layouts.find((l) => l.id === currentLayoutId);
      draftStatIds = (applied?.stat_ids ?? []).map((id) => ({ id }));
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to load layouts");
    } finally {
      isLoading = false;
    }
  }

  /** Whether the draft has diverged from the currently-applied layout's own saved `stat_ids` — gates "Save changes"/"Save as new layout" so they're not offered with nothing to commit. */
  let isDraftDirty = $derived(
    !currentLayout ||
      draftStatIds.length !== currentLayout.stat_ids.length ||
      draftStatIds.some((item, index) => item.id !== currentLayout.stat_ids[index]),
  );

  $effect(() => {
    if (!dialogEl) return;
    if (open) {
      if (!dialogEl.open) dialogEl.showModal();
    } else if (dialogEl.open) {
      dialogEl.close();
    }
  });

  function handleBackdropClick(event: MouseEvent) {
    if (!dialogEl || event.target !== dialogEl) return;
    const rect = dialogEl.getBoundingClientRect();
    const insideContent =
      event.clientX >= rect.left &&
      event.clientX <= rect.right &&
      event.clientY >= rect.top &&
      event.clientY <= rect.bottom;
    if (!insideContent) dialogEl.close();
  }

  async function refreshStats() {
    if (projectId) await refreshProjectStatusStats(projectId, weekStartsOn);
  }

  /** Applies an existing saved layout — if `onPickLayout` is provided (global settings context), delegates to it; otherwise sets the project's own `board.status_line_layout_id` override. */
  async function applyExistingLayout(layoutId: string) {
    isSaving = true;
    errorMessage = "";
    try {
      await pickLayout(layoutId);
      await refreshStats();
      open = false;
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to apply layout");
    } finally {
      isSaving = false;
    }
  }

  /** Picks `layoutId` as the active layout, delegating to `onPickLayout` if provided or falling back to the per-project `board.status_line_layout_id` update. */
  async function pickLayout(layoutId: string) {
    if (onPickLayout) {
      await onPickLayout(layoutId);
      return;
    }
    await updateProjectLayoutOverride(layoutId);
  }

  /**
   * Sets `layoutId` as this project's own `board.status_line_layout_id`
   * override. `updateProject` takes a full `Project`, but this component is
   * only ever handed a `projectId` (not the full record — see this
   * component's own props doc), so it fetches the current project list
   * fresh rather than threading the whole `Project` down as a prop just for
   * this one write.
   */
  async function updateProjectLayoutOverride(layoutId: string) {
    if (!projectId) throw new Error("No project in scope");
    const projects = await listProjects();
    const project = projects.find((p) => p.id === projectId);
    if (!project) throw new Error("Project not found");
    await updateProject({ ...project, board: { ...project.board, status_line_layout_id: layoutId } });
    await refreshProjects();
  }

  function handleToggleStat(statId: string, enabled: boolean) {
    draftStatIds = toggleStatId(draftStatIds.map((item) => item.id), statId, enabled).map((id) => ({ id }));
  }

  function handleConsider(event: CustomEvent<DndEvent<{ id: string }>>) {
    draftStatIds = event.detail.items;
  }

  function handleFinalize(event: CustomEvent<DndEvent<{ id: string }>>) {
    draftStatIds = event.detail.items;
  }

  /** Display label for a `stat_ids` entry that may be a stale/unrecognized id (a layout edited by a future version, or a since-removed stat) — degrades to the raw id rather than throwing, mirroring `ProjectStatusLine.svelte`'s own `isKnownStatusLineStatId` filtering. `"status_badge"` is handled first since `statLabel` only accepts the other 5 stat ids (see `StatusLineStatId`'s own doc comment). */
  function displayLabelFor(statId: string): string {
    if (statId === "status_badge") return "Status badge";
    if (!isKnownStatusLineStatId(statId)) return statId;
    return statLabel(statId as Exclude<typeof statId, "status_badge">);
  }

  /** Commits the draft back to the currently-applied layout *in place* — every other project or the global default pointing at this same layout id sees the change immediately, per `StatLayout`'s shared-edit semantics. */
  async function saveInPlace() {
    if (!currentLayout) return;
    isSaving = true;
    errorMessage = "";
    try {
      await updateStatusLayout({ ...currentLayout, stat_ids: reorderStatIds(draftStatIds) });
      await refreshStats();
      open = false;
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to save layout");
    } finally {
      isSaving = false;
    }
  }

  /** Forks the *currently-applied* layout immediately, before any in-popover edits are taken into account — a fast "branch off this one" action, distinct from "Save as new layout" (which commits in-progress draft edits to a new layout instead). */
  async function confirmDuplicate() {
    if (!currentLayout || newLayoutNameDraft.trim() === "") return;
    isSaving = true;
    errorMessage = "";
    try {
      const forked = await duplicateStatusLayout(currentLayout.id, newLayoutNameDraft.trim());
      await pickLayout(forked.id);
      await refreshStats();
      open = false;
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to duplicate layout");
    } finally {
      isSaving = false;
    }
  }

  /** Commits the current draft's `stat_ids` to a brand-new layout instead of saving them back in place on the original — used after live-editing (reordering/toggling) the currently-applied layout in this popover. */
  async function confirmSaveAsNew() {
    if (newLayoutNameDraft.trim() === "") return;
    isSaving = true;
    errorMessage = "";
    try {
      const created = await createStatusLayout(newLayoutNameDraft.trim(), reorderStatIds(draftStatIds));
      await pickLayout(created.id);
      await refreshStats();
      open = false;
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to save new layout");
    } finally {
      isSaving = false;
    }
  }

  /** Permanently deletes the currently-applied layout. The backend rejects if any project or the global default still references it. */
  async function confirmDelete() {
    if (!currentLayout) return;
    isSaving = true;
    errorMessage = "";
    try {
      await deleteStatusLayout(currentLayout.id);
      open = false;
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to delete layout");
      showDeleteConfirm = false;
    } finally {
      isSaving = false;
    }
  }
</script>

<button type="button" class="trigger" onclick={openEditor} aria-label="Customize status line" aria-haspopup="dialog">
  Customize
</button>

<dialog bind:this={dialogEl} class="layout-editor-dialog" aria-label="Customize status line" onclose={() => (open = false)} onclick={handleBackdropClick}>
  <div class="dialog-header">
    <p class="dialog-title">Customize status line</p>
    <button type="button" class="close-button" onclick={() => dialogEl?.close()} aria-label="Close" title="Close">
      <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
        <path d="M3 3l10 10M13 3L3 13" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" />
      </svg>
    </button>
  </div>

  {#if isLoading}
    <p class="loading">Loading…</p>
  {:else}
    <div class="field">
      <label for="existing-layout-select">Apply a different layout</label>
      <select
        id="existing-layout-select"
        value=""
        disabled={isSaving}
        onchange={(event) => {
          const value = (event.currentTarget as HTMLSelectElement).value;
          if (value) applyExistingLayout(value);
        }}
      >
        <option value="" disabled selected>Choose a saved layout…</option>
        {#each layouts.filter((l) => l.id !== currentLayoutId) as layout (layout.id)}
          <option value={layout.id}>{layout.name}</option>
        {/each}
      </select>
    </div>

    <h3>Edit "{currentLayout?.name ?? "current layout"}"</h3>
    <p class="hint">
      Drag to reorder, toggle a stat off to remove it. Saving here edits this layout in place for
      everyone pointing at it.
    </p>

    <ul
      class="stat-list"
      aria-label="Stats shown on this status line, drag to reorder"
      use:dndzone={{ items: draftStatIds, flipDurationMs: FLIP_DURATION_MS, dropTargetStyle: {} }}
      onconsider={handleConsider}
      onfinalize={handleFinalize}
    >
      {#each draftStatIds as item (item.id)}
        <li class="stat-row" aria-label={`Reorder ${displayLabelFor(item.id)}`}>
          <label class="stat-toggle">
            <input
              type="checkbox"
              checked={true}
              onchange={(event) => handleToggleStat(item.id, (event.currentTarget as HTMLInputElement).checked)}
            />
            {displayLabelFor(item.id)}
          </label>
        </li>
      {/each}
    </ul>

    <h3 id="other-stats-heading">Other stats</h3>
    <ul class="stat-list">
      {#each ALL_STATUS_LINE_STAT_IDS.filter((id) => !draftStatIds.some((item) => item.id === id)) as statId (statId)}
        <li class="stat-row">
          <label class="stat-toggle">
            <input type="checkbox" checked={false} onchange={() => handleToggleStat(statId, true)} />
            {displayLabelFor(statId)}
          </label>
        </li>
      {/each}
    </ul>

    {#if errorMessage}
      <p class="error" role="alert">{errorMessage}</p>
    {/if}

    <div class="actions">
      {#if showDuplicateInput}
        <input
          type="text"
          placeholder="New layout name"
          bind:value={newLayoutNameDraft}
          aria-label="New layout name for duplicate"
        />
        <button type="button" class="secondary" onclick={() => (showDuplicateInput = false)}>Cancel</button>
        <button type="button" disabled={isSaving || newLayoutNameDraft.trim() === ""} onclick={confirmDuplicate}>
          Confirm duplicate
        </button>
      {:else if showSaveAsNewInput}
        <input
          type="text"
          placeholder="New layout name"
          bind:value={newLayoutNameDraft}
          aria-label="New layout name for save as new"
        />
        <button type="button" class="secondary" onclick={() => (showSaveAsNewInput = false)}>Cancel</button>
        <button type="button" disabled={isSaving || newLayoutNameDraft.trim() === ""} onclick={confirmSaveAsNew}>
          Confirm save as new
        </button>
      {:else if showDeleteConfirm}
        <span class="delete-confirm-text">Delete "{currentLayout?.name}"? This cannot be undone.</span>
        <button type="button" class="secondary" onclick={() => (showDeleteConfirm = false)}>Cancel</button>
        <button type="button" class="danger" disabled={isSaving} onclick={confirmDelete}>
          {isSaving ? "Deleting…" : "Confirm delete"}
        </button>
      {:else}
        <button
          type="button"
          class="secondary danger-hover"
          disabled={isSaving || !currentLayout}
          onclick={() => (showDeleteConfirm = true)}
        >
          Delete
        </button>
        <button
          type="button"
          class="secondary"
          disabled={isSaving || !currentLayout}
          onclick={() => {
            newLayoutNameDraft = "";
            showDuplicateInput = true;
          }}
        >
          Duplicate
        </button>
        <button
          type="button"
          class="secondary"
          disabled={isSaving || !isDraftDirty}
          onclick={() => {
            newLayoutNameDraft = "";
            showSaveAsNewInput = true;
          }}
        >
          Save as new layout
        </button>
        <button type="button" disabled={isSaving || !isDraftDirty} onclick={saveInPlace}>
          {isSaving ? "Saving…" : "Save changes"}
        </button>
      {/if}
    </div>
  {/if}
</dialog>

<style>
  .trigger {
    flex-shrink: 0;
    padding: var(--space-4xs) var(--space-sm);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: transparent;
    color: var(--color-ink-muted);
    font-size: var(--text-xs);
    cursor: pointer;
    transition:
      color var(--duration-fast) var(--ease-out-expo),
      border-color var(--duration-fast) var(--ease-out-expo);
  }

  .trigger:hover {
    color: var(--color-ink);
    border-color: var(--color-accent);
  }

  .trigger:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .layout-editor-dialog {
    padding: var(--space-lg);
    border: none;
    border-radius: var(--radius-lg);
    background: var(--color-surface-raised);
    color: var(--color-ink);
    box-shadow: var(--shadow-lg);
    width: min(28rem, calc(100vw - 2 * var(--space-lg)));
  }

  .layout-editor-dialog:not([open]) {
    display: none;
  }

  .layout-editor-dialog::backdrop {
    background: oklch(20% 0.02 50 / 0.45);
  }

  .dialog-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-sm);
    margin-bottom: var(--space-md);
  }

  .dialog-title {
    margin: 0;
    font-size: var(--text-base);
    font-weight: 600;
    letter-spacing: var(--tracking-tight);
  }

  .close-button {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 1.75rem;
    height: 1.75rem;
    border-radius: var(--radius-md);
    border: none;
    background: transparent;
    color: var(--color-ink-muted);
    cursor: pointer;
  }

  .close-button:hover {
    color: var(--color-ink);
    background: var(--color-canvas);
  }

  .loading {
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
    margin-bottom: var(--space-md);
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
  }

  h3 {
    margin: var(--space-md) 0 var(--space-2xs);
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
  }

  .hint {
    margin: 0 0 var(--space-sm);
    font-size: var(--text-xs);
    color: var(--color-ink-faint);
  }

  .stat-list {
    list-style: none;
    margin: 0 0 var(--space-md);
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
    min-height: 2.5rem;
  }

  .stat-row {
    display: flex;
    align-items: center;
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    cursor: grab;
  }

  .stat-toggle {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: var(--text-sm);
    cursor: pointer;
    width: 100%;
  }

  .stat-toggle input[type="checkbox"] {
    width: 1.1rem;
    height: 1.1rem;
    accent-color: var(--color-accent);
    cursor: pointer;
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
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: var(--space-xs);
  }

  .actions input[type="text"] {
    flex: 1;
    min-width: 10rem;
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
    font-size: var(--text-sm);
  }

  .actions button {
    padding: var(--space-2xs) var(--space-md);
    border-radius: var(--radius-md);
    border: none;
    font-weight: 600;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .actions button:not(.secondary) {
    background: var(--color-accent);
    color: var(--color-accent-ink);
  }

  .actions button:disabled {
    background: var(--color-border);
    color: var(--color-ink-muted);
    cursor: not-allowed;
  }

  .actions button.secondary {
    background: var(--color-surface);
    color: var(--color-ink);
    border: 1px solid var(--color-border);
  }

  .actions button.secondary:disabled {
    opacity: 0.5;
  }

  .actions button.danger {
    background: var(--color-danger, oklch(55% 0.19 25));
    color: white;
  }

  .actions button.danger:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .actions button.danger-hover:hover:not(:disabled) {
    background: var(--color-danger-soft, oklch(95% 0.04 25));
    color: var(--color-danger, oklch(55% 0.19 25));
    border-color: var(--color-danger, oklch(55% 0.19 25));
  }

  .delete-confirm-text {
    flex: 1;
    font-size: var(--text-sm);
    color: var(--color-danger, oklch(55% 0.19 25));
    font-weight: 600;
  }
</style>
