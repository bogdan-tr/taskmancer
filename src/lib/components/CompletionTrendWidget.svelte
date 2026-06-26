<script lang="ts">
  import type { DashboardDateRange } from "$lib/api";
  import { getDashboardCompletionTrend } from "$lib/api";
  import type { WeekStartsOn } from "$lib/displaySettings.svelte";
  import type { DashboardCompletionWeek } from "$lib/types";

  interface Props {
    projectId: string | null;
    dateRange: DashboardDateRange;
    weekStartsOn: WeekStartsOn;
  }
  let { projectId, dateRange, weekStartsOn }: Props = $props();

  let weeks = $state<DashboardCompletionWeek[]>([]);
  let loading = $state(true);
  let error = $state(false);

  $effect(() => {
    loading = true;
    error = false;
    getDashboardCompletionTrend(projectId, dateRange, weekStartsOn)
      .then((d) => {
        weeks = d;
        loading = false;
      })
      .catch(() => {
        error = true;
        loading = false;
      });
  });

  let maxTotal = $derived(
    weeks.reduce((m, w) => {
      const total = w.series.reduce((s, x) => s + x.count, 0);
      return Math.max(m, total);
    }, 0),
  );
</script>

<div class="widget">
  <h3 class="widget-title">Completion Trend</h3>

  {#if loading}
    <div class="widget-empty">Loading…</div>
  {:else if error}
    <div class="widget-empty widget-error">Failed to load data.</div>
  {:else if weeks.length === 0}
    <div class="widget-empty">No data for this period.</div>
  {:else}
    <div class="chart-container">
      <div class="trend-chart">
        {#each weeks as week (week.week_label)}
          {@const total = week.series.reduce((s, x) => s + x.count, 0)}
          {@const pct = maxTotal > 0 ? (total / maxTotal) * 100 : 0}
          <div class="trend-col">
            <div class="trend-bar-wrap">
              {#if total > 0}
                <span class="trend-count">{total}</span>
              {/if}
              <div class="trend-bar" style="height: {pct}%"></div>
            </div>
            <span class="trend-label">{week.week_label}</span>
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
    overflow-x: auto;
  }

  .trend-chart {
    display: flex;
    align-items: flex-end;
    gap: var(--space-xs);
    height: 10rem;
    min-width: max-content;
    padding-bottom: var(--space-lg);
  }

  .trend-col {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    flex: 1;
    min-width: 2.5rem;
    height: 100%;
    justify-content: flex-end;
  }

  .trend-bar-wrap {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-end;
    width: 100%;
    flex: 1;
    gap: 2px;
  }

  .trend-count {
    font-size: var(--text-xs);
    color: var(--color-ink-muted);
    font-variant-numeric: tabular-nums;
  }

  .trend-bar {
    width: 100%;
    min-height: 2px;
    background: var(--color-accent);
    border-radius: var(--radius-sm) var(--radius-sm) 0 0;
    transition: height 0.3s ease;
  }

  .trend-label {
    font-size: 0.625rem;
    color: var(--color-ink-faint);
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }
</style>
