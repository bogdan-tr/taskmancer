<script lang="ts">
  import type { DashboardDateRange } from "$lib/api";
  import { getDashboardStatusDistribution } from "$lib/api";
  import type { DashboardStatusCount } from "$lib/types";
  import { settingsState } from "$lib/settings.svelte";

  interface Props {
    projectId: string | null;
    dateRange: DashboardDateRange;
  }
  let { projectId, dateRange }: Props = $props();

  let data = $state<DashboardStatusCount[]>([]);
  let loading = $state(true);
  let error = $state(false);

  $effect(() => {
    loading = true;
    error = false;
    getDashboardStatusDistribution(projectId, dateRange)
      .then((d) => {
        data = d;
        loading = false;
      })
      .catch(() => {
        error = true;
        loading = false;
      });
  });

  function getStatusDef(statusId: string) {
    return settingsState.current?.statuses.find((s) => s.id === statusId);
  }

  function getColor(statusId: string): string {
    return getStatusDef(statusId)?.color ?? "var(--color-border)";
  }

  function getLabel(statusId: string): string {
    return getStatusDef(statusId)?.label ?? statusId;
  }

  let total = $derived(data.reduce((s, x) => s + x.count, 0));
</script>

<div class="widget">
  <h3 class="widget-title">Status Distribution</h3>

  {#if loading}
    <div class="widget-empty">Loading…</div>
  {:else if error}
    <div class="widget-empty widget-error">Failed to load data.</div>
  {:else if data.length === 0}
    <div class="widget-empty">No data for this period.</div>
  {:else}
    <div class="chart-container">
      <div class="status-list">
        {#each data as item (item.status_id)}
          {@const pct = total > 0 ? Math.round((item.count / total) * 100) : 0}
          {@const color = getColor(item.status_id)}
          <div class="status-row">
            <span class="status-dot" style="background: {color}"></span>
            <span class="status-name">{getLabel(item.status_id)}</span>
            <div class="status-bar-track">
              <div
                class="status-bar-fill"
                style="width: {pct}%; background: {color}"
              ></div>
            </div>
            <span class="status-count">{item.count}</span>
            <span class="status-pct">{pct}%</span>
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

  .widget-title {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
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

  .status-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .status-row {
    display: grid;
    grid-template-columns: 0.75rem 6rem 1fr 2.5rem 2.5rem;
    align-items: center;
    gap: var(--space-sm);
  }

  .status-dot {
    width: 0.625rem;
    height: 0.625rem;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .status-name {
    font-size: var(--text-sm);
    color: var(--color-ink-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status-bar-track {
    height: 0.5rem;
    background: color-mix(in srgb, var(--color-border) 50%, transparent);
    border-radius: var(--radius-full, 9999px);
    overflow: hidden;
  }

  .status-bar-fill {
    height: 100%;
    border-radius: inherit;
    opacity: 0.75;
    transition: width 0.3s ease;
  }

  .status-count {
    font-size: var(--text-xs);
    color: var(--color-ink-muted);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .status-pct {
    font-size: var(--text-xs);
    color: var(--color-ink-faint);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
