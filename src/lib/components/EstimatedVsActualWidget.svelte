<script lang="ts">
  import type { DashboardDateRange } from "$lib/api";
  import { getDashboardEstimatedVsActual } from "$lib/api";
  import type { DashboardEstVsActual } from "$lib/types";
  import { formatMinutes } from "$lib/estimatedTime";

  interface Props {
    projectId: string | null;
    dateRange: DashboardDateRange;
  }
  let { projectId, dateRange }: Props = $props();

  let data = $state<DashboardEstVsActual[]>([]);
  let loading = $state(true);
  let error = $state(false);

  $effect(() => {
    loading = true;
    error = false;
    getDashboardEstimatedVsActual(projectId, dateRange)
      .then((d) => {
        data = d;
        loading = false;
      })
      .catch(() => {
        error = true;
        loading = false;
      });
  });

  let topItems = $derived([...data].slice(0, 6));

  let maxMinutes = $derived(
    topItems.reduce(
      (m, item) => Math.max(m, item.estimated_minutes, item.actual_minutes),
      0,
    ),
  );
</script>

<div class="widget">
  <h3 class="widget-title">Estimated vs Actual</h3>

  {#if loading}
    <div class="widget-empty">Loading…</div>
  {:else if error}
    <div class="widget-empty widget-error">Failed to load data.</div>
  {:else if topItems.length === 0}
    <div class="widget-empty">No data for this period.</div>
  {:else}
    <div class="chart-container">
      <div class="legend">
        <span class="legend-item">
          <span class="legend-dot estimated"></span>Estimated
        </span>
        <span class="legend-item">
          <span class="legend-dot actual"></span>Actual
        </span>
      </div>
      <div class="bar-list">
        {#each topItems as item (item.project_name)}
          {@const estPct = maxMinutes > 0 ? (item.estimated_minutes / maxMinutes) * 100 : 0}
          {@const actPct = maxMinutes > 0 ? (item.actual_minutes / maxMinutes) * 100 : 0}
          <div class="project-group">
            <span class="project-label" title={item.project_name}>{item.project_name}</span>
            <div class="dual-bars">
              <div class="bar-row">
                <div class="bar-track">
                  <div class="bar-fill estimated" style="width: {estPct}%"></div>
                </div>
                <span class="bar-value">{formatMinutes(item.estimated_minutes)}</span>
              </div>
              <div class="bar-row">
                <div class="bar-track">
                  <div class="bar-fill actual" style="width: {actPct}%"></div>
                </div>
                <span class="bar-value">{formatMinutes(item.actual_minutes)}</span>
              </div>
            </div>
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
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    overflow-y: auto;
  }

  .legend {
    display: flex;
    gap: var(--space-md);
    font-size: var(--text-xs);
    color: var(--color-ink-muted);
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
  }

  .legend-dot {
    width: 0.625rem;
    height: 0.625rem;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .legend-dot.estimated {
    background: var(--color-accent);
    opacity: 0.5;
  }

  .legend-dot.actual {
    background: var(--color-accent);
  }

  .bar-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .project-group {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .project-label {
    font-size: var(--text-xs);
    color: var(--color-ink-muted);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dual-bars {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .bar-row {
    display: grid;
    grid-template-columns: 1fr 4rem;
    align-items: center;
    gap: var(--space-sm);
  }

  .bar-track {
    height: 0.6rem;
    background: color-mix(in srgb, var(--color-border) 50%, transparent);
    border-radius: var(--radius-full, 9999px);
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    border-radius: inherit;
    transition: width 0.3s ease;
  }

  .bar-fill.estimated {
    background: var(--color-accent);
    opacity: 0.5;
  }

  .bar-fill.actual {
    background: var(--color-accent);
  }

  .bar-value {
    font-size: var(--text-xs);
    color: var(--color-ink-muted);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
