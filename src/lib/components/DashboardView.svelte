<script lang="ts">
  import type { DashboardDateRange } from "$lib/api";
  import { listStatusLayouts } from "$lib/api";
  import { displayState } from "$lib/displaySettings.svelte";
  import { projectsState } from "$lib/projects.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import type { StatLayout } from "$lib/types";
  import TimeByProjectTagWidget from "./TimeByProjectTagWidget.svelte";
  import EstimatedVsActualWidget from "./EstimatedVsActualWidget.svelte";
  import CompletionTrendWidget from "./CompletionTrendWidget.svelte";
  import StatusDistributionWidget from "./StatusDistributionWidget.svelte";
  import BusyHistogramWidget from "./BusyHistogramWidget.svelte";

  interface Props {
    projectId: string | null;
  }
  let { projectId }: Props = $props();

  // ── Date range ──────────────────────────────────────────────────────────────
  let dateRange = $state<DashboardDateRange>("last_30_days");

  const DATE_RANGE_LABELS: Record<DashboardDateRange, string> = {
    last_7_days: "Last 7 days",
    last_30_days: "Last 30 days",
    this_month: "This month",
    last_3_months: "Last 3 months",
    all_time: "All time",
  };

  // ── Layout resolution ───────────────────────────────────────────────────────
  let project = $derived(
    projectId ? projectsState.items.find((p) => p.id === projectId) : undefined,
  );

  let resolvedLayoutId = $derived(
    (project?.board.dashboard_layout_id ?? settingsState.current?.default_dashboard_layout_id) ??
      "",
  );

  // ── Layout list ─────────────────────────────────────────────────────────────
  let layouts = $state<StatLayout[]>([]);

  $effect(() => {
    listStatusLayouts()
      .then((all) => {
        layouts = all.filter((l) => l.kind === "dashboard");
      })
      .catch(() => {});
  });

  let resolvedLayout = $derived(
    layouts.find((l) => l.id === resolvedLayoutId) ?? layouts[0] ?? null,
  );

  // ── Widget configuration ────────────────────────────────────────────────────
  const DEFAULT_WIDGET_IDS = [
    "time_by_project_tag",
    "estimated_vs_actual",
    "completion_trend",
    "status_distribution",
    "busy_histogram",
  ];

  let widgetIds = $derived(resolvedLayout?.stat_ids ?? DEFAULT_WIDGET_IDS);
  let widgetWidths = $derived(resolvedLayout?.widget_widths ?? {});

  function widgetWidth(id: string): "half" | "full" {
    return widgetWidths[id] ?? "half";
  }
</script>

<div class="dashboard">
  <div class="dashboard-header">
    <h1 class="dashboard-title">{projectId ? "Project Dashboard" : "Dashboard"}</h1>
    <div class="dashboard-controls">
      <label class="range-label" for="date-range">Date range</label>
      <select id="date-range" class="range-select" bind:value={dateRange}>
        {#each Object.entries(DATE_RANGE_LABELS) as [value, label] (value)}
          <option {value}>{label}</option>
        {/each}
      </select>
    </div>
  </div>

  <div class="widget-grid">
    {#each widgetIds as widgetId (widgetId)}
      <div
        class="widget-slot"
        class:full-width={widgetWidth(widgetId) === "full"}
      >
        {#if widgetId === "time_by_project_tag"}
          <TimeByProjectTagWidget {projectId} {dateRange} />
        {:else if widgetId === "estimated_vs_actual"}
          <EstimatedVsActualWidget {projectId} {dateRange} />
        {:else if widgetId === "completion_trend"}
          <CompletionTrendWidget
            {projectId}
            {dateRange}
            weekStartsOn={displayState.weekStartsOn}
          />
        {:else if widgetId === "status_distribution"}
          <StatusDistributionWidget {projectId} {dateRange} />
        {:else if widgetId === "busy_histogram"}
          <BusyHistogramWidget {projectId} {dateRange} />
        {/if}
      </div>
    {/each}
    {#if widgetIds.length === 0}
      <p class="empty">No widgets configured. Edit the dashboard layout in Settings.</p>
    {/if}
  </div>
</div>

<style>
  .dashboard {
    max-width: 1200px;
    margin: 0 auto;
    padding: var(--space-xl) var(--space-lg) var(--space-2xl);
  }

  .dashboard-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-xl);
    gap: var(--space-md);
    flex-wrap: wrap;
  }

  .dashboard-title {
    margin: 0;
    font-size: var(--text-xl);
    font-weight: 700;
    color: var(--color-ink);
  }

  .dashboard-controls {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .range-label {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-ink-muted);
  }

  .range-select {
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .range-select:focus-visible {
    border-color: var(--color-accent);
    outline: none;
    box-shadow: 0 0 0 3px var(--color-accent-soft);
  }

  .widget-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-lg);
  }

  .widget-slot {
    min-height: 16rem;
  }

  .widget-slot.full-width {
    grid-column: 1 / -1;
  }

  .empty {
    grid-column: 1 / -1;
    text-align: center;
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
    padding: var(--space-2xl);
  }
</style>
