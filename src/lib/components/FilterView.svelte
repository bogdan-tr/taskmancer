<script lang="ts">
  import FilterDrawer from "$lib/components/FilterDrawer.svelte";
  import { filterViewState } from "$lib/filterViewState.svelte";
  import { savedViewsState, refreshSavedViews } from "$lib/savedViewsState.svelte";
  import { isFilterActive } from "$lib/filterLogic";
  import { createSavedView, updateSavedView } from "$lib/api";
  import type { SortKey, SortDirection } from "$lib/types";

  interface Props {
    taskCount: number;
  }
  const { taskCount }: Props = $props();

  // ── Sort controls ─────────────────────────────────────────────────────────────

  const SORT_KEY_LABELS: Record<SortKey, string> = {
    due: "Due date",
    scheduled: "Scheduled",
    priority: "Priority",
    created: "Created",
    title: "Title",
    estimated_time: "Estimate",
    tracked_time: "Tracked",
    status: "Status",
  };

  const SORT_KEYS: SortKey[] = [
    "due", "scheduled", "priority", "created", "title",
    "estimated_time", "tracked_time", "status",
  ];

  function addSortLevel() {
    if (filterViewState.sort.levels.length >= 3) return;
    const usedKeys = new Set(filterViewState.sort.levels.map((l) => l.key));
    const key = SORT_KEYS.find((k) => !usedKeys.has(k)) ?? "due";
    filterViewState.sort = {
      levels: [...filterViewState.sort.levels, { key, direction: "asc" }],
    };
    filterViewState.activeViewId = null;
  }

  function removeSortLevel(idx: number) {
    filterViewState.sort = {
      levels: filterViewState.sort.levels.filter((_, i) => i !== idx),
    };
    filterViewState.activeViewId = null;
  }

  function updateSortKey(idx: number, key: SortKey) {
    const levels = filterViewState.sort.levels.map((l, i) =>
      i === idx ? { ...l, key } : l,
    );
    filterViewState.sort = { levels };
    filterViewState.activeViewId = null;
  }

  function updateSortDir(idx: number, direction: SortDirection) {
    const levels = filterViewState.sort.levels.map((l, i) =>
      i === idx ? { ...l, direction } : l,
    );
    filterViewState.sort = { levels };
    filterViewState.activeViewId = null;
  }

  // ── Save view dialog ───────────────────────────────────────────────────────────

  let saveDialogOpen = $state(false);
  let saveAsNew = $state(false);
  let saveName = $state("");
  let saveColor = $state("#3b82f6");
  let saveIcon = $state("star");
  let saveError = $state("");

  const PRESET_COLORS = [
    "#3b82f6", "#8b5cf6", "#ec4899", "#ef4444",
    "#f97316", "#eab308", "#22c55e", "#06b6d4",
  ];

  const PRESET_ICONS = ["star", "bolt", "heart", "flag", "target", "filter", "tag", "clock"];

  const ICON_GLYPHS: Record<string, string> = {
    star: "★", bolt: "⚡", heart: "♥", flag: "⚑",
    target: "◎", filter: "⊟", tag: "⌖", clock: "◷",
  };

  function openSaveDialog(asNew: boolean) {
    saveAsNew = asNew;
    if (!asNew && filterViewState.activeViewId) {
      const view = savedViewsState.items.find((v) => v.id === filterViewState.activeViewId);
      saveName = view?.name ?? "";
      saveColor = view?.color ?? "#3b82f6";
      saveIcon = view?.icon ?? "star";
    } else {
      saveName = "";
      saveColor = "#3b82f6";
      saveIcon = "star";
    }
    saveDialogOpen = true;
  }

  async function confirmSave() {
    if (!saveName.trim()) return;
    saveError = "";
    const fc = JSON.stringify(filterViewState.config);
    const sc = JSON.stringify(filterViewState.sort);
    try {
      if (!saveAsNew && filterViewState.activeViewId) {
        await updateSavedView(filterViewState.activeViewId, saveName.trim(), saveColor, saveIcon, fc, sc);
      } else {
        const created = await createSavedView(saveName.trim(), saveColor, saveIcon, fc, sc);
        filterViewState.activeViewId = created.id;
      }
      await refreshSavedViews();
      saveDialogOpen = false;
    } catch (err) {
      saveError = err instanceof Error ? err.message : String(err);
    }
  }

  // ── Derived state ─────────────────────────────────────────────────────────────

  const filterActive = $derived(isFilterActive(filterViewState.config));
  const activeViewName = $derived(
    filterViewState.activeViewId
      ? savedViewsState.items.find((v) => v.id === filterViewState.activeViewId)?.name
      : undefined,
  );
  const isDirty = $derived(filterViewState.activeViewId === null && filterActive);
  const canUpdate = $derived(filterViewState.activeViewId !== null);
</script>

<div class="filter-view">
  <!-- Toolbar -->
  <div class="filter-toolbar">
    <div class="toolbar-left">
      {#if activeViewName}
        <span class="view-name">{activeViewName}</span>
        <span class="view-divider">·</span>
      {/if}
      <span class="task-count">{taskCount} task{taskCount === 1 ? "" : "s"}</span>

      <!-- Sort levels -->
      <div class="sort-levels" aria-label="Sort controls">
        {#each filterViewState.sort.levels as level, idx (idx)}
          <div class="sort-level">
            {#if idx === 0}
              <span class="sort-label">Sort by</span>
            {:else}
              <span class="sort-label">then</span>
            {/if}
            <select
              class="sort-select"
              value={level.key}
              onchange={(e) => updateSortKey(idx, (e.currentTarget as HTMLSelectElement).value as SortKey)}
              aria-label="Sort key {idx + 1}"
            >
              {#each SORT_KEYS as key (key)}
                <option value={key}>{SORT_KEY_LABELS[key]}</option>
              {/each}
            </select>
            <select
              class="sort-dir-select"
              value={level.direction}
              onchange={(e) => updateSortDir(idx, (e.currentTarget as HTMLSelectElement).value as SortDirection)}
              aria-label="Sort direction {idx + 1}"
            >
              <option value="asc">↑ Asc</option>
              <option value="desc">↓ Desc</option>
            </select>
            <button
              type="button"
              class="sort-remove"
              onclick={() => removeSortLevel(idx)}
              aria-label="Remove sort level"
            >×</button>
          </div>
        {/each}
        {#if filterViewState.sort.levels.length < 3}
          <button type="button" class="add-sort-btn" onclick={addSortLevel}>
            + Sort
          </button>
        {/if}
      </div>
    </div>

    <div class="toolbar-right">
      <!-- Filter button -->
      <button
        type="button"
        class="filter-btn"
        class:filter-btn-active={filterActive}
        class:drawer-open={filterViewState.drawerOpen}
        onclick={() => (filterViewState.drawerOpen = !filterViewState.drawerOpen)}
        aria-pressed={filterViewState.drawerOpen}
      >
        <span class="filter-icon">⊟</span>
        Filter
        {#if filterActive}
          <span class="filter-badge" aria-label="Filters active"></span>
        {/if}
      </button>

      <!-- View save actions -->
      {#if canUpdate}
        <button type="button" class="save-btn save-btn-secondary" onclick={() => openSaveDialog(false)}>
          Update view
        </button>
        <button type="button" class="save-btn save-btn-ghost" onclick={() => openSaveDialog(true)}>
          Save as new
        </button>
      {:else if isDirty}
        <button type="button" class="save-btn save-btn-primary" onclick={() => openSaveDialog(false)}>
          Save view
        </button>
      {/if}
    </div>
  </div>

  <!-- Filter drawer -->
  {#if filterViewState.drawerOpen}
    <FilterDrawer />
  {/if}
</div>

<!-- Save view dialog -->
{#if saveDialogOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="dialog-backdrop" onclick={() => (saveDialogOpen = false)}></div>
  <div class="save-dialog" role="dialog" aria-label="Save view" aria-modal="true">
    <h3 class="dialog-title">{saveAsNew || !filterViewState.activeViewId ? "Save view" : "Update view"}</h3>

    <label class="dialog-field">
      <span class="field-label">Name</span>
      <input
        type="text"
        class="dialog-input"
        bind:value={saveName}
        placeholder="My filter…"
        autofocus
        onkeydown={(e) => { if (e.key === "Enter") void confirmSave(); if (e.key === "Escape") saveDialogOpen = false; }}
      />
    </label>

    <div class="dialog-field">
      <span class="field-label">Color</span>
      <div class="color-swatches">
        {#each PRESET_COLORS as color (color)}
          <button
            type="button"
            class="color-swatch"
            class:swatch-active={saveColor === color}
            style="background: {color}"
            onclick={() => (saveColor = color)}
            aria-label="Color {color}"
            aria-pressed={saveColor === color}
          ></button>
        {/each}
      </div>
    </div>

    <div class="dialog-field">
      <span class="field-label">Icon</span>
      <div class="icon-options">
        {#each PRESET_ICONS as icon (icon)}
          <button
            type="button"
            class="icon-option"
            class:icon-active={saveIcon === icon}
            style={saveIcon === icon ? `color: ${saveColor}` : ""}
            onclick={() => (saveIcon = icon)}
            aria-label="Icon {icon}"
            aria-pressed={saveIcon === icon}
          >
            {ICON_GLYPHS[icon] ?? "★"}
          </button>
        {/each}
      </div>
    </div>

    {#if saveError}
      <p class="save-error">{saveError}</p>
    {/if}

    <div class="dialog-actions">
      <button type="button" class="dialog-cancel" onclick={() => (saveDialogOpen = false)}>
        Cancel
      </button>
      <button
        type="button"
        class="dialog-confirm"
        style="background: {saveColor}"
        disabled={!saveName.trim()}
        onclick={() => void confirmSave()}
      >
        Save
      </button>
    </div>
  </div>
{/if}

<style>
  .filter-view {
    display: contents;
  }

  /* ── Toolbar ──────────────────────────────────────────────────────────────── */

  .filter-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md);
    padding: var(--space-xs) var(--space-lg);
    border-bottom: 1px solid var(--color-border);
    background: var(--color-surface);
    flex-shrink: 0;
    flex-wrap: wrap;
    min-height: 44px;
  }

  .toolbar-left {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }

  .view-name {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--color-text);
  }

  .view-divider {
    color: var(--color-text-muted);
  }

  .task-count {
    font-size: 0.8125rem;
    color: var(--color-text-muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .sort-levels {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    flex-wrap: wrap;
  }

  .sort-level {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .sort-label {
    font-size: 0.75rem;
    color: var(--color-text-muted);
    white-space: nowrap;
  }

  .sort-select,
  .sort-dir-select {
    font-size: 0.8125rem;
    padding: 2px 6px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    cursor: pointer;
  }

  .sort-dir-select {
    width: 72px;
  }

  .sort-remove {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    background: none;
    border: none;
    color: var(--color-text-muted);
    font-size: 1rem;
    cursor: pointer;
    border-radius: var(--radius-sm);
    transition: color 150ms, background 150ms;
    padding: 0;
  }

  .sort-remove:hover {
    color: var(--color-text);
    background: var(--color-surface-raised);
  }

  .add-sort-btn {
    font-size: 0.75rem;
    color: var(--color-text-muted);
    background: none;
    border: 1px dashed var(--color-border);
    border-radius: var(--radius-sm);
    padding: 2px 8px;
    cursor: pointer;
    white-space: nowrap;
    transition: color 150ms, border-color 150ms;
  }

  .add-sort-btn:hover {
    color: var(--color-text);
    border-color: var(--color-text-muted);
  }

  .toolbar-right {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  /* ── Filter button ─────────────────────────────────────────────────────────── */

  .filter-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 4px 12px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text-muted);
    font-size: 0.8125rem;
    cursor: pointer;
    position: relative;
    transition: color 150ms, border-color 150ms, background 150ms;
  }

  .filter-btn:hover {
    color: var(--color-text);
    border-color: var(--color-text-muted);
  }

  .filter-btn-active {
    border-color: var(--color-accent);
    color: var(--color-accent);
    background: color-mix(in srgb, var(--color-accent) 8%, transparent);
  }

  .drawer-open {
    background: color-mix(in srgb, var(--color-accent) 12%, transparent);
  }

  .filter-icon {
    font-size: 0.875rem;
  }

  .filter-badge {
    position: absolute;
    top: 3px;
    right: 3px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--color-accent);
    border: 1.5px solid var(--color-surface);
  }

  /* ── Save buttons ──────────────────────────────────────────────────────────── */

  .save-btn {
    font-size: 0.8125rem;
    padding: 4px 12px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    white-space: nowrap;
    transition: opacity 150ms, background 150ms;
  }

  .save-btn-primary {
    background: var(--color-accent);
    color: #fff;
    border: none;
  }

  .save-btn-primary:hover {
    opacity: 0.88;
  }

  .save-btn-secondary {
    background: none;
    border: 1px solid var(--color-accent);
    color: var(--color-accent);
  }

  .save-btn-secondary:hover {
    background: color-mix(in srgb, var(--color-accent) 10%, transparent);
  }

  .save-btn-ghost {
    background: none;
    border: 1px solid var(--color-border);
    color: var(--color-text-muted);
  }

  .save-btn-ghost:hover {
    border-color: var(--color-text-muted);
    color: var(--color-text);
  }

  /* ── Save dialog ───────────────────────────────────────────────────────────── */

  .dialog-backdrop {
    position: fixed;
    inset: 0;
    background: color-mix(in srgb, var(--color-text) 30%, transparent);
    z-index: 300;
  }

  .save-dialog {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 301;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-lg);
    width: 320px;
    box-shadow: 0 8px 32px color-mix(in srgb, var(--color-text) 18%, transparent);
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .dialog-title {
    font-size: 1rem;
    font-weight: 700;
    color: var(--color-text);
    margin: 0;
  }

  .dialog-field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .field-label {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-text-muted);
  }

  .dialog-input {
    padding: var(--space-xs) var(--space-sm);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    font-size: 0.9375rem;
  }

  .dialog-input:focus {
    outline: none;
    border-color: var(--color-accent);
  }

  .color-swatches {
    display: flex;
    gap: var(--space-xs);
    flex-wrap: wrap;
  }

  .color-swatch {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    transition: transform 150ms, border-color 150ms;
  }

  .swatch-active {
    border-color: var(--color-text);
    transform: scale(1.15);
  }

  .color-swatch:hover:not(.swatch-active) {
    transform: scale(1.08);
  }

  .icon-options {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-xs);
  }

  .icon-option {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1.5px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: none;
    font-size: 1rem;
    cursor: pointer;
    color: var(--color-text-muted);
    transition: border-color 150ms, color 150ms, background 150ms;
  }

  .icon-active {
    border-color: currentColor;
    background: color-mix(in srgb, currentColor 10%, transparent);
  }

  .icon-option:hover:not(.icon-active) {
    border-color: var(--color-text-muted);
    color: var(--color-text);
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-sm);
    margin-top: var(--space-xs);
  }

  .dialog-cancel {
    padding: 6px 16px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: none;
    color: var(--color-text-muted);
    font-size: 0.875rem;
    cursor: pointer;
  }

  .dialog-cancel:hover {
    color: var(--color-text);
    border-color: var(--color-text-muted);
  }

  .dialog-confirm {
    padding: 6px 16px;
    border: none;
    border-radius: var(--radius-sm);
    color: #fff;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 150ms;
  }

  .dialog-confirm:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .dialog-confirm:hover:not(:disabled) {
    opacity: 0.88;
  }

  .save-error {
    margin: 0;
    font-size: 0.8125rem;
    color: var(--color-danger, #ef4444);
    padding: var(--space-xs) var(--space-sm);
    background: color-mix(in srgb, #ef4444 8%, transparent);
    border-radius: var(--radius-sm);
  }
</style>
