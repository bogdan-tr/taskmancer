<script lang="ts">
  import { tierColor as canonicalTierColor } from "$lib/tierColors";
  import { getDashboardProjectHealth } from "$lib/api";
  import type { DashboardProjectHealth } from "$lib/types";
  import WidgetHeader from "./WidgetHeader.svelte";

  interface Props {
    includeSubprojects: boolean;
  }
  let { includeSubprojects }: Props = $props();

  let data = $state<DashboardProjectHealth[]>([]);
  let loading = $state(true);
  let error = $state(false);

  $effect(() => {
    loading = true;
    error = false;
    getDashboardProjectHealth(includeSubprojects)
      .then((result) => { data = result; loading = false; })
      .catch(() => { error = true; loading = false; });
  });

  const tierColor = canonicalTierColor;

  function fmtTime(mins: number): string {
    if (mins <= 0) return "—";
    const h = Math.floor(mins / 60);
    const m = mins % 60;
    if (h > 0 && m > 0) return `${h}h ${m}m`;
    if (h > 0) return `${h}h`;
    return `${m}m`;
  }

</script>

<div class="widget">
  <WidgetHeader widgetType="project_health" />

  {#if loading}
    <div class="state-msg">Loading…</div>
  {:else if error}
    <div class="state-msg state-err">Failed to load data</div>
  {:else if data.length === 0}
    <div class="state-msg">No projects</div>
  {:else}
    <div class="rows">
      {#each data as row (row.project_id)}
        {@const tc = tierColor(row.tier)}
        <div class="health-row" style="--tier-color:{tc}">
          <!-- Project identity -->
          <div class="project-cell">
            <span class="project-dot" style="background:{row.project_color}"></span>
            <span class="project-name" title={row.project_name}>{row.project_name}</span>
          </div>

          <!-- Metric: due today -->
          <div class="metric-cell">
            <span class="metric-value" style={row.tasks_due_today > 0 ? "color:#f59e0b" : ""}>
              {row.tasks_due_today}
            </span>
            <span class="metric-label">TODAY</span>
          </div>

          <!-- Metric: due tomorrow -->
          <div class="metric-cell">
            <span class="metric-value" style={row.tasks_due_tomorrow > 0 ? "color:#f97316" : ""}>
              {row.tasks_due_tomorrow}
            </span>
            <span class="metric-label">TMRW</span>
          </div>

          <!-- Metric: estimated time left -->
          <div class="metric-cell metric-cell-wide">
            <span class="metric-value metric-time">
              {fmtTime(row.estimated_time_left_minutes)}
            </span>
            <span class="metric-label">LEFT</span>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .widget {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .rows {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow-y: auto;
  }

  .health-row {
    flex: 1;
    display: grid;
    grid-template-columns: 2fr 1fr 1fr 1.2fr;
    align-items: center;
    gap: 4px;
    padding: 4px 6px;
    border-radius: 6px;
    /* Left-to-right fade of the tier color — healthy rows stay just as
       vivid as troubled ones (no dimming, per user request). */
    background: linear-gradient(
      90deg,
      color-mix(in srgb, var(--tier-color) 22%, transparent),
      color-mix(in srgb, var(--tier-color) 10%, transparent)
    );
    min-height: 36px;
  }

  /* Project identity */
  .project-cell {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    overflow: hidden;
  }

  .project-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .project-name {
    font-size: 14px;
    font-weight: 500;
    color: var(--db-ink, #e6edf3);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  /* Metric cells */
  .metric-cell {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
    overflow: hidden;
  }


  .metric-value {
    font-size: 16px;
    font-weight: 700;
    line-height: 1;
    color: var(--db-ink, #e6edf3);
  }

  .metric-time {
    font-size: 13px;
    color: var(--db-ink-muted, #8b949e);
  }

  .metric-label {
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--db-ink-muted, #8b949e);
    white-space: nowrap;
  }

  /* State messages */
  .state-msg {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    color: var(--db-ink-muted, #8b949e);
  }

  .state-err {
    color: #ef4444;
  }
</style>
