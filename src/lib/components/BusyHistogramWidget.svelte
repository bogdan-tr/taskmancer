<script lang="ts">
  import type { DashboardDateRange } from "$lib/api";
  import { getDashboardBusyHistogram } from "$lib/api";
  import type { DashboardBusyHistogram, DashboardBucket } from "$lib/types";

  interface Props {
    projectId: string | null;
    dateRange: DashboardDateRange;
  }
  let { projectId, dateRange }: Props = $props();

  const EMPTY_HISTOGRAM: DashboardBusyHistogram = { days: [], hours: [] };

  let histData = $state<DashboardBusyHistogram>(EMPTY_HISTOGRAM);
  let loading = $state(true);
  let error = $state(false);

  $effect(() => {
    loading = true;
    error = false;
    getDashboardBusyHistogram(projectId, dateRange)
      .then((d) => {
        histData = d;
        loading = false;
      })
      .catch(() => {
        error = true;
        loading = false;
      });
  });

  const DAY_LABELS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

  function hourLabel(index: number): string {
    if (index === 0) return "12am";
    if (index < 12) return `${index}am`;
    if (index === 12) return "12pm";
    return `${index - 12}pm`;
  }

  function maxOf(buckets: DashboardBucket[]): number {
    return buckets.reduce((m, b) => Math.max(m, b.minutes), 0);
  }

  let hasData = $derived(
    histData.days.length > 0 || histData.hours.length > 0,
  );

  let maxDayMinutes = $derived(maxOf(histData.days));
  let maxHourMinutes = $derived(maxOf(histData.hours));
</script>

<div class="widget">
  <h3 class="widget-title">Busiest Times</h3>

  {#if loading}
    <div class="widget-empty">Loading…</div>
  {:else if error}
    <div class="widget-empty widget-error">Failed to load data.</div>
  {:else if !hasData}
    <div class="widget-empty">No data for this period.</div>
  {:else}
    <div class="panels">
      <!-- Day of week panel -->
      <div class="panel">
        <span class="panel-label">By day of week</span>
        <div class="vert-chart">
          {#each histData.days as bucket (bucket.index)}
            {@const pct = maxDayMinutes > 0 ? (bucket.minutes / maxDayMinutes) * 100 : 0}
            <div class="vert-col">
              <div class="vert-bar-wrap">
                <div class="vert-bar" style="height: {pct}%"></div>
              </div>
              <span class="vert-label">{DAY_LABELS[bucket.index] ?? bucket.index}</span>
            </div>
          {/each}
        </div>
      </div>

      <!-- Hour of day panel -->
      <div class="panel">
        <span class="panel-label">By hour of day</span>
        <div class="vert-chart hour-chart">
          {#each histData.hours as bucket (bucket.index)}
            {@const pct = maxHourMinutes > 0 ? (bucket.minutes / maxHourMinutes) * 100 : 0}
            {@const showLabel = bucket.index % 3 === 0}
            <div class="vert-col hour-col">
              <div class="vert-bar-wrap">
                <div class="vert-bar" style="height: {pct}%"></div>
              </div>
              <span class="vert-label" class:hidden={!showLabel}>
                {showLabel ? hourLabel(bucket.index) : ""}
              </span>
            </div>
          {/each}
        </div>
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

  .panels {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 1fr 2fr;
    gap: var(--space-lg);
  }

  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .panel-label {
    font-size: var(--text-xs);
    color: var(--color-ink-faint);
    font-weight: 600;
  }

  .vert-chart {
    display: flex;
    align-items: flex-end;
    gap: 4px;
    height: 8rem;
    padding-bottom: 1.25rem;
  }

  .hour-chart {
    overflow-x: auto;
    gap: 2px;
  }

  .vert-col {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-end;
    gap: 2px;
    flex: 1;
    height: 100%;
  }

  .hour-col {
    min-width: 1rem;
    flex-shrink: 0;
  }

  .vert-bar-wrap {
    width: 100%;
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
  }

  .vert-bar {
    width: 100%;
    min-height: 2px;
    background: var(--color-accent);
    border-radius: var(--radius-sm) var(--radius-sm) 0 0;
    opacity: 0.7;
    transition: height 0.3s ease;
  }

  .vert-label {
    font-size: 0.625rem;
    color: var(--color-ink-faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
    text-align: center;
  }

  .vert-label.hidden {
    visibility: hidden;
  }
</style>
