<script lang="ts">
  import type { DashboardDateRange } from "$lib/api";
  import { getDashboardTimeByProject, getDashboardTimeByTag } from "$lib/api";
  import type { DashboardTimeEntry } from "$lib/types";
  import { formatMinutes } from "$lib/estimatedTime";

  interface Props {
    projectId: string | null;
    dateRange: DashboardDateRange;
  }
  let { projectId, dateRange }: Props = $props();

  type Mode = "project" | "tag";
  let mode = $state<Mode>("project");

  let data = $state<DashboardTimeEntry[]>([]);
  let loading = $state(true);
  let error = $state(false);

  $effect(() => {
    loading = true;
    error = false;
    const fn = mode === "project" ? getDashboardTimeByProject : getDashboardTimeByTag;
    fn(projectId, dateRange)
      .then((d) => {
        data = d;
        loading = false;
      })
      .catch(() => {
        error = true;
        loading = false;
      });
  });

  let topItems = $derived(
    [...data].sort((a, b) => b.minutes - a.minutes).slice(0, 8),
  );

  let maxMinutes = $derived(topItems[0]?.minutes ?? 0);
</script>

<div class="widget">
  <div class="widget-header">
    <h3 class="widget-title">Time Tracked</h3>
    <div class="toggle-group">
      <button
        class="toggle-btn"
        class:active={mode === "project"}
        onclick={() => (mode = "project")}
      >
        By project
      </button>
      <button
        class="toggle-btn"
        class:active={mode === "tag"}
        onclick={() => (mode = "tag")}
      >
        By tag
      </button>
    </div>
  </div>

  {#if loading}
    <div class="widget-empty">Loading…</div>
  {:else if error}
    <div class="widget-empty widget-error">Failed to load data.</div>
  {:else if topItems.length === 0}
    <div class="widget-empty">No data for this period.</div>
  {:else}
    <div class="chart-container">
      <div class="bar-list">
        {#each topItems as item (item.label)}
          {@const pct = maxMinutes > 0 ? (item.minutes / maxMinutes) * 100 : 0}
          <div class="bar-row">
            <span class="bar-label" title={item.label}>{item.label}</span>
            <div class="bar-track">
              <div class="bar-fill" style="width: {pct}%"></div>
            </div>
            <span class="bar-value">{formatMinutes(item.minutes)}</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .widget {
    height: 100%;
    min-height: 16rem;
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    padding: var(--space-md) var(--space-lg);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
  }

  .widget-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }

  .widget-title {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
  }

  .toggle-group {
    display: flex;
    gap: 2px;
    background: var(--color-surface-raised, var(--color-border));
    border-radius: var(--radius-sm);
    padding: 2px;
  }

  .toggle-btn {
    padding: 2px var(--space-sm);
    font-size: var(--text-xs);
    font-weight: 600;
    border: none;
    border-radius: calc(var(--radius-sm) - 2px);
    background: transparent;
    color: var(--color-ink-muted);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .toggle-btn.active {
    background: var(--color-surface);
    color: var(--color-ink);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12);
  }

  .widget-empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-ink-faint);
    font-size: var(--text-sm);
  }

  .widget-error {
    color: var(--color-danger);
  }

  .chart-container {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  .bar-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .bar-row {
    display: grid;
    grid-template-columns: 6rem 1fr 4rem;
    align-items: center;
    gap: var(--space-sm);
  }

  .bar-label {
    font-size: var(--text-sm);
    color: var(--color-ink-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .bar-track {
    height: 0.75rem;
    background: var(--color-surface-raised, color-mix(in srgb, var(--color-border) 50%, transparent));
    border-radius: var(--radius-full, 9999px);
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    background: var(--color-accent);
    border-radius: inherit;
    transition: width 0.3s ease;
  }

  .bar-value {
    font-size: var(--text-xs);
    color: var(--color-ink-muted);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
