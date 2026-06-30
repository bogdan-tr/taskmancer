<script lang="ts">
  import { projectsState } from "$lib/projects.svelte";
  import { FALLBACK_STATUSES, statusColor, statusLabel } from "$lib/statuses.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import type { SearchResult } from "$lib/types";

  interface Props {
    result: SearchResult;
    query: string;
    focused: boolean;
    onOpen: (result: SearchResult) => void;
    onRestore?: (result: SearchResult) => void;
  }

  let { result, query, focused, onOpen, onRestore }: Props = $props();

  const statuses = $derived(settingsState.current?.statuses ?? FALLBACK_STATUSES);
  const project = $derived(projectsState.items.find((p) => p.id === result.task.project_id));
  const taskStatusColor = $derived(statusColor(statuses, result.task.status));
  const taskStatusLabel = $derived(statusLabel(statuses, result.task.status));

  /** Wrap all case-insensitive occurrences of `q` in `text` with <mark>. */
  function highlight(text: string, q: string): string {
    if (!q.trim()) return escapeHtml(text);
    const escaped = escapeHtml(text);
    const escapedQ = escapeHtml(q).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    return escaped.replace(new RegExp(escapedQ, "gi"), (m) => `<mark>${m}</mark>`);
  }

  function escapeHtml(s: string): string {
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }

  const highlightedTitle = $derived(highlight(result.task.title, query));
  const highlightedSnippet = $derived(
    result.notes_snippet ? highlight(result.notes_snippet, query) : null,
  );

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString(undefined, { month: "short", day: "numeric" });
  }

  const displayDate = $derived(
    result.task.archived_at ??
      result.task.completed_at ??
      result.task.cancelled_at ??
      result.task.created,
  );
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="result-card"
  class:focused
  onclick={() => onOpen(result)}
  onkeydown={(e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onOpen(result);
    }
  }}
  role="option"
  aria-selected={focused}
  tabindex="-1"
>
  <div class="card-content">
    <div class="card-top">
      <span class="status-pill" style="--status-color: {taskStatusColor}" title={taskStatusLabel}>
        {taskStatusLabel}
      </span>
      <!-- eslint-disable-next-line svelte/no-at-html-tags -->
      <span class="title">{@html highlightedTitle}</span>
      <span class="card-date">{formatDate(displayDate)}</span>
    </div>

    <div class="card-meta">
      {#if project}
        <span class="project-name">{project.name}</span>
      {/if}
      {#each result.task.tags as tag}
        <span class="tag">#{tag}</span>
      {/each}
      {#if result.is_archived}
        <span class="archived-badge">Archived</span>
      {/if}
    </div>

    {#if highlightedSnippet}
      <div class="notes-snippet">
        <!-- eslint-disable-next-line svelte/no-at-html-tags -->
        {@html highlightedSnippet}
      </div>
    {/if}
  </div>

  {#if result.is_archived && onRestore}
    <button
      type="button"
      class="restore-btn"
      title="Restore task"
      onclick={(e) => {
        e.stopPropagation();
        onRestore?.(result);
      }}
      aria-label="Restore {result.task.title}"
    >
      ↩
    </button>
  {/if}
</div>

<style>
  .result-card {
    display: flex;
    align-items: flex-start;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    cursor: pointer;
    transition: border-color 150ms, background 150ms;
    outline: none;
  }

  .result-card:hover {
    border-color: var(--color-accent);
    background: var(--color-surface-raised);
  }

  .result-card.focused {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-accent) 30%, transparent);
  }

  .card-content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .card-top {
    display: flex;
    align-items: baseline;
    gap: var(--space-xs);
    flex-wrap: wrap;
  }

  .status-pill {
    flex-shrink: 0;
    font-size: 0.7rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    padding: 1px 6px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--status-color) 18%, transparent);
    color: var(--status-color);
    border: 1px solid color-mix(in srgb, var(--status-color) 35%, transparent);
  }

  .title {
    flex: 1;
    font-size: 0.9375rem;
    font-weight: 500;
    color: var(--color-text);
    line-height: 1.4;
  }

  .card-date {
    flex-shrink: 0;
    font-size: 0.75rem;
    color: var(--color-text-muted);
    margin-left: auto;
  }

  .card-meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-xs);
    font-size: 0.75rem;
    color: var(--color-text-muted);
  }

  .project-name {
    color: var(--color-accent);
    font-weight: 500;
  }

  .tag {
    color: var(--color-text-muted);
  }

  .archived-badge {
    padding: 1px 6px;
    border-radius: 4px;
    background: color-mix(in srgb, var(--color-text-muted) 12%, transparent);
    font-size: 0.7rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .notes-snippet {
    font-size: 0.8125rem;
    color: var(--color-text-muted);
    line-height: 1.5;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  :global(.notes-snippet mark),
  :global(.title mark) {
    background: color-mix(in srgb, var(--color-accent) 25%, transparent);
    color: var(--color-accent);
    border-radius: 2px;
    padding: 0 1px;
  }

  .restore-btn {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-muted);
    font-size: 1rem;
    cursor: pointer;
    opacity: 0;
    transition: opacity 150ms, background 150ms, color 150ms;
    margin-top: 2px;
  }

  .result-card:hover .restore-btn,
  .result-card.focused .restore-btn {
    opacity: 1;
  }

  .restore-btn:hover {
    background: color-mix(in srgb, var(--color-accent) 15%, transparent);
    color: var(--color-accent);
    border-color: var(--color-accent);
  }
</style>
