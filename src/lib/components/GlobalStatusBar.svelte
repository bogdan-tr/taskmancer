<script lang="ts">
  import { getGlobalStatusStats } from "$lib/api";
  import { displayState } from "$lib/displaySettings.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import { formatMinutes } from "$lib/estimatedTime";
  import type { GlobalStatusStats } from "$lib/types";

  let stats = $state<GlobalStatusStats | null>(null);
  let loadError = $state(false);

  async function load() {
    try {
      stats = await getGlobalStatusStats(displayState.weekStartsOn);
      loadError = false;
    } catch {
      loadError = true;
    }
  }

  $effect(() => {
    void load();
  });

  let statuses = $derived(settingsState.current?.statuses ?? []);

  /** Looks up the display label for a status id from the global settings list. */
  function statusLabel(id: string): string {
    return statuses.find((s) => s.id === id)?.label ?? id;
  }
</script>

{#if stats && !loadError && (settingsState.current?.status_bar_enabled ?? true)}
  <div class="global-status-bar">
    <div class="global-status-bar-inner">
      <div class="stat-group">
        <span class="group-label">Tasks</span>
        <div class="status-counts">
          {#each stats.tasks_by_status as [statusId, count] (statusId)}
            <span class="status-count">
              <span class="status-count-label">{statusLabel(statusId)}</span>
              <span class="status-count-value">{count}</span>
            </span>
          {/each}
          {#if stats.tasks_by_status.length === 0}
            <span class="empty">No tasks</span>
          {/if}
        </div>
      </div>

      <div class="stat-divider" aria-hidden="true"></div>

      <div class="stat-item">
        <span class="stat-label">Projects</span>
        <span class="stat-value">{stats.total_projects}</span>
      </div>

      <div class="stat-divider" aria-hidden="true"></div>

      <div class="stat-item">
        <span class="stat-label">Today</span>
        <span class="stat-value">{formatMinutes(stats.time_tracked_today_minutes)}</span>
      </div>

      <div class="stat-item">
        <span class="stat-label">This week</span>
        <span class="stat-value">{formatMinutes(stats.time_tracked_this_week_minutes)}</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .global-status-bar {
    margin-bottom: var(--space-lg);
    padding: var(--space-sm) var(--space-lg);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
  }

  .global-status-bar-inner {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-md);
  }

  .stat-group {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    min-width: 0;
  }

  .group-label {
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
    flex-shrink: 0;
  }

  .status-counts {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-xs);
  }

  .status-count {
    display: inline-flex;
    align-items: center;
    gap: var(--space-3xs);
    padding: var(--space-4xs) var(--space-xs);
    border-radius: var(--radius-pill);
    border: 1px solid var(--color-border);
    background: var(--color-canvas);
    font-size: var(--text-xs);
  }

  .status-count-label {
    color: var(--color-ink-muted);
  }

  .status-count-value {
    font-weight: 700;
    color: var(--color-ink);
  }

  .empty {
    font-size: var(--text-xs);
    color: var(--color-ink-faint);
  }

  .stat-divider {
    width: 1px;
    height: 1.5rem;
    background: var(--color-border);
    flex-shrink: 0;
  }

  .stat-item {
    display: flex;
    flex-direction: column;
    gap: var(--space-4xs);
    flex-shrink: 0;
  }

  .stat-label {
    font-size: var(--text-xs);
    color: var(--color-ink-muted);
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
  }

  .stat-value {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-ink);
  }
</style>
