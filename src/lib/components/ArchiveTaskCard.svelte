<script lang="ts">
  import { formatMinutes } from "$lib/estimatedTime";
  import { projectsState } from "$lib/projects.svelte";
  import { FALLBACK_STATUSES, statusColor, statusLabel } from "$lib/statuses.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import type { Task } from "$lib/types";

  interface Props {
    task: Task;
    focused?: boolean;
    onOpenDetail: (task: Task) => void;
    onRestore: (task: Task) => void;
  }

  let { task, focused = false, onOpenDetail, onRestore }: Props = $props();

  const statuses = $derived(settingsState.current?.statuses ?? FALLBACK_STATUSES);
  const project = $derived(projectsState.items.find((p) => p.id === task.project_id));

  function formatArchivedAt(task: Task): string {
    const ts = task.archived_at ?? task.completed_at ?? task.cancelled_at;
    if (!ts) return "";
    const d = new Date(ts);
    return d.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" }) +
      " " + d.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
  }

  const archivedLabel = $derived(formatArchivedAt(task));
  const taskStatusColor = $derived(statusColor(statuses, task.status));
  const taskStatusLabel = $derived(statusLabel(statuses, task.status));
</script>

<div
  class="archive-card"
  class:focused
  onclick={() => onOpenDetail(task)}
  role="button"
  tabindex="0"
  onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onOpenDetail(task); } }}
>
  <div class="card-main">
    <div class="card-top">
      <span class="status-pill" style="--status-color: {taskStatusColor}" title={taskStatusLabel}>
        {taskStatusLabel}
      </span>
      <span class="title">{task.title}</span>
    </div>
    <div class="card-meta">
      {#if project}
        <span class="project-name">{project.name}</span>
        {#if task.tags.length > 0}
          <span class="meta-sep">·</span>
        {/if}
      {/if}
      {#each task.tags as tag, i}
        <span class="tag">#{tag}</span>{#if i < task.tags.length - 1}<span class="meta-sep"> </span>{/if}
      {/each}
      {#if task.estimated_minutes || task.tracked_minutes > 0}
        <span class="meta-sep">·</span>
        <span class="time-info">
          {#if task.tracked_minutes > 0}{formatMinutes(task.tracked_minutes)} tracked{/if}
          {#if task.estimated_minutes && task.tracked_minutes > 0} / {/if}
          {#if task.estimated_minutes}{formatMinutes(task.estimated_minutes)} est{/if}
        </span>
      {/if}
    </div>
    {#if archivedLabel}
      <div class="archived-date">Archived {archivedLabel}</div>
    {/if}
  </div>

  <button
    type="button"
    class="restore-btn"
    title="Restore task"
    onclick={(e) => { e.stopPropagation(); onRestore(task); }}
    aria-label="Restore {task.title}"
  >
    ↩
  </button>
</div>

<style>
  .archive-card {
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

  .archive-card:hover {
    border-color: var(--color-accent);
    background: var(--color-surface-raised);
  }

  .archive-card.focused {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-accent) 30%, transparent);
  }

  .card-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
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
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--color-text);
    line-height: 1.4;
  }

  .card-meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 2px;
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

  .meta-sep {
    color: var(--color-border-strong);
    margin: 0 2px;
  }

  .time-info {
    color: var(--color-text-muted);
  }

  .archived-date {
    font-size: 0.7rem;
    color: var(--color-text-subtle, var(--color-text-muted));
    margin-top: 1px;
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

  .archive-card:hover .restore-btn,
  .archive-card.focused .restore-btn {
    opacity: 1;
  }

  .restore-btn:hover {
    background: color-mix(in srgb, var(--color-accent) 15%, transparent);
    color: var(--color-accent);
    border-color: var(--color-accent);
  }
</style>
