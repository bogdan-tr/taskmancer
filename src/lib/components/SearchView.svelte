<script lang="ts">
  import { onMount } from "svelte";
  import { searchTasks } from "$lib/api";
  import { parseTaskInput } from "$lib/naturalLanguage";
  import { projectsState } from "$lib/projects.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import { FALLBACK_PRIORITIES, sortedPriorities } from "$lib/priorities.svelte";
  import { FALLBACK_STATUSES, sortedStatuses } from "$lib/statuses.svelte";
  import type { SearchResult, Task } from "$lib/types";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import SearchResultCard from "./SearchResultCard.svelte";

  interface Props {
    onOpenDetail: (task: Task) => void;
    onOpenArchivedDetail: (task: Task) => void;
    onRestore: (task: Task) => void;
  }

  let { onOpenDetail, onOpenArchivedDetail, onRestore }: Props = $props();

  // ─── State ───────────────────────────────────────────────────────────────────

  let rawQuery = $state("");
  let includeArchived = $state(false);
  let results: SearchResult[] = $state([]);
  let searching = $state(false);
  let focusedIndex = $state(-1);
  let searchInputEl: HTMLInputElement | undefined = $state();
  let confirmRestoreResult: SearchResult | undefined = $state(undefined);

  onMount(() => { searchInputEl?.focus(); });

  // ─── Debounced search ────────────────────────────────────────────────────────

  let debounceTimer: ReturnType<typeof setTimeout> | undefined;

  function getParsed(query: string) {
    const knownPriorities = sortedPriorities(
      settingsState.current?.priorities ?? FALLBACK_PRIORITIES,
    ).map(({ id, label }) => ({ id, label }));
    const knownStatuses = sortedStatuses(
      settingsState.current?.statuses ?? FALLBACK_STATUSES,
    ).map(({ id, label }) => ({ id, label }));
    return parseTaskInput(query, undefined, knownPriorities, knownStatuses);
  }

  async function runSearch(query: string, archived: boolean) {
    const parsed = getParsed(query);
    const titleText = parsed.title.trim();
    const notesText = parsed.notes?.trim() ?? null;

    if (!titleText && !notesText) {
      results = [];
      return;
    }

    searching = true;
    try {
      let raw = await searchTasks(titleText, notesText || null, archived);

      // Frontend filters from NL tokens
      if (parsed.tags.length > 0) {
        raw = raw.filter((r) => parsed.tags.every((tag) => r.task.tags.includes(tag)));
      }
      if (parsed.priority) {
        raw = raw.filter((r) => r.task.priority === parsed.priority);
      }
      if (parsed.projectId) {
        raw = raw.filter((r) => r.task.project_id === parsed.projectId);
      } else if (parsed.project) {
        const matched = projectsState.items.find(
          (p) => p.name.toLowerCase() === parsed.project?.toLowerCase(),
        );
        if (matched) raw = raw.filter((r) => r.task.project_id === matched.id);
      }
      if (parsed.status) {
        raw = raw.filter((r) => r.task.status === parsed.status);
      }

      results = raw;
      if (focusedIndex >= results.length) focusedIndex = results.length - 1;
    } catch {
      results = [];
    } finally {
      searching = false;
    }
  }

  function scheduleSearch() {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      void runSearch(rawQuery, includeArchived);
    }, 150);
  }

  $effect(() => {
    void rawQuery;
    void includeArchived;
    scheduleSearch();
  });

  // ─── The text shown in the result cards' highlight helper ─────────────────────

  const highlightQuery = $derived(getParsed(rawQuery).title.trim());

  // ─── Keyboard navigation ─────────────────────────────────────────────────────

  function handleKeydown(e: KeyboardEvent) {
    const target = e.target as HTMLElement;
    const inInput = target?.matches("input, textarea");

    if (inInput) {
      if (e.key === "Escape") {
        const el = target as HTMLInputElement;
        if (el.value) {
          el.value = "";
          rawQuery = "";
          e.stopPropagation();
        }
        // If already empty, let Escape bubble up to KanbanBoard to close SearchView
      }
      return;
    }

    if (e.key === "j" || e.key === "ArrowDown") {
      e.preventDefault();
      focusedIndex = Math.min(focusedIndex + 1, results.length - 1);
    } else if (e.key === "k" || e.key === "ArrowUp") {
      e.preventDefault();
      focusedIndex = Math.max(focusedIndex - 1, 0);
    } else if ((e.key === "e" || e.key === "Enter") && focusedIndex >= 0) {
      e.preventDefault();
      openResult(results[focusedIndex]);
    } else if (e.key === "r" && focusedIndex >= 0) {
      const r = results[focusedIndex];
      if (r?.is_archived) {
        e.preventDefault();
        confirmRestoreResult = r;
      }
    } else if (e.key === "a") {
      e.preventDefault();
      includeArchived = !includeArchived;
    } else if (e.key === "/" || e.key === "f") {
      e.preventDefault();
      searchInputEl?.focus();
    }
  }

  function openResult(result: SearchResult | undefined) {
    if (!result) return;
    if (result.is_archived) {
      onOpenArchivedDetail(result.task);
    } else {
      onOpenDetail(result.task);
    }
  }

  async function confirmRestore() {
    const r = confirmRestoreResult;
    confirmRestoreResult = undefined;
    if (!r) return;
    // Remove from results immediately
    results = results.filter((x) => x.task.id !== r.task.id);
    onRestore(r.task);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="search-view" onkeydown={handleKeydown}>
  <!-- Search input bar -->
  <div class="search-bar">
    <div class="input-wrap">
      <span class="search-icon" aria-hidden="true">⌕</span>
      <input
        bind:this={searchInputEl}
        type="search"
        class="search-input"
        placeholder="Search tasks… or use #tag +project high ;; notes text"
        bind:value={rawQuery}
        aria-label="Search tasks"
      />
      {#if rawQuery}
        <button
          type="button"
          class="clear-btn"
          onclick={() => { rawQuery = ""; searchInputEl?.focus(); }}
          aria-label="Clear search"
        >
          ×
        </button>
      {/if}
    </div>

    <label class="archived-toggle">
      <input type="checkbox" bind:checked={includeArchived} />
      Include archived
    </label>
  </div>

  <!-- Hint row -->
  <div class="hint-row">
    <span class="hint">Use <kbd>#tag</kbd> <kbd>+project</kbd> <kbd>high</kbd> <kbd>due:friday</kbd> to filter · <kbd>;;</kbd> to search notes · <kbd>j</kbd>/<kbd>k</kbd> navigate · <kbd>e</kbd> open · <kbd>a</kbd> toggle archived</span>
  </div>

  <!-- Results -->
  {#if searching}
    <div class="status-line">Searching…</div>
  {:else if rawQuery && results.length === 0}
    <div class="status-line empty">No tasks match — try broader terms.</div>
  {:else if results.length > 0}
    <div class="results-count">{results.length} result{results.length !== 1 ? "s" : ""}{includeArchived ? " (including archived)" : ""}</div>
    <div class="results" role="listbox" aria-label="Search results">
      {#each results as result (result.task.id)}
        {@const idx = results.indexOf(result)}
        <SearchResultCard
          {result}
          query={highlightQuery}
          focused={focusedIndex === idx}
          onOpen={(r) => { focusedIndex = idx; openResult(r); }}
          onRestore={(r) => (confirmRestoreResult = r)}
        />
      {/each}
    </div>
  {:else}
    <div class="empty-state">
      <span class="empty-icon">🔍</span>
      <p>Start typing to search your tasks.</p>
    </div>
  {/if}
</div>

<ConfirmDialog
  open={confirmRestoreResult !== undefined}
  title="Restore task"
  message={`Restore "${confirmRestoreResult?.task.title}" back to your board?`}
  confirmLabel="Restore"
  onConfirm={confirmRestore}
  onCancel={() => (confirmRestoreResult = undefined)}
/>

<style>
  .search-view {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    outline: none;
    height: 100%;
  }

  /* ── Search bar ──────────────────────────────────────────────────────────── */

  .search-bar {
    display: flex;
    align-items: center;
    gap: var(--space-md);
  }

  .input-wrap {
    position: relative;
    flex: 1;
  }

  .search-icon {
    position: absolute;
    left: var(--space-md);
    top: 50%;
    transform: translateY(-50%);
    color: var(--color-text-muted);
    font-size: 1.25rem;
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    padding: var(--space-sm) var(--space-xl) var(--space-sm) calc(var(--space-md) + 1.5rem);
    border: 1.5px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    color: var(--color-text);
    font-size: 1rem;
    box-sizing: border-box;
    transition: border-color 150ms;
  }

  .search-input:focus {
    outline: none;
    border-color: var(--color-accent);
  }

  .clear-btn {
    position: absolute;
    right: var(--space-sm);
    top: 50%;
    transform: translateY(-50%);
    background: none;
    border: none;
    color: var(--color-text-muted);
    font-size: 1.25rem;
    line-height: 1;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: var(--radius-sm);
  }

  .clear-btn:hover {
    color: var(--color-text);
    background: var(--color-surface-raised);
  }

  .archived-toggle {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: 0.8125rem;
    color: var(--color-text-muted);
    white-space: nowrap;
    cursor: pointer;
    user-select: none;
  }

  /* ── Hint row ────────────────────────────────────────────────────────────── */

  .hint-row {
    min-height: 1.25rem;
  }

  .hint {
    font-size: 0.75rem;
    color: var(--color-text-muted);
  }

  kbd {
    display: inline-block;
    padding: 0 4px;
    border: 1px solid var(--color-border);
    border-radius: 3px;
    font-size: 0.7rem;
    font-family: monospace;
    background: var(--color-surface-raised);
    color: var(--color-text-muted);
  }

  /* ── Results ─────────────────────────────────────────────────────────────── */

  .results-count {
    font-size: 0.75rem;
    color: var(--color-text-muted);
  }

  .results {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    overflow-y: auto;
    flex: 1;
    padding-bottom: var(--space-md);
  }

  .status-line {
    font-size: 0.875rem;
    color: var(--color-text-muted);
    padding: var(--space-sm) 0;
  }

  .status-line.empty {
    font-style: italic;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    color: var(--color-text-muted);
    text-align: center;
    gap: var(--space-sm);
    padding: var(--space-2xl) 0;
  }

  .empty-icon {
    font-size: 2.5rem;
    line-height: 1;
  }
</style>
