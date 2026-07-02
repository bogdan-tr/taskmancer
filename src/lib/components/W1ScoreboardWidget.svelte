<script lang="ts">
  import { getProjectScoreboard } from "$lib/api";
  import type { DashboardDateRange } from "$lib/api";
  import type { ProjectScoreboard } from "$lib/types";
  import WidgetHeader from "./WidgetHeader.svelte";

  interface Props {
    projectId: string;
    projectColor: string;
    dateRange: DashboardDateRange;
  }
  let { projectId, projectColor, dateRange }: Props = $props();

  let data = $state<ProjectScoreboard | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    if (!projectId) return;
    loading = true;
    error = null;
    getProjectScoreboard(projectId, dateRange)
      .then((d) => { data = d; })
      .catch((e) => { error = e instanceof Error ? e.message : String(e); })
      .finally(() => { loading = false; });
  });

  function fmtMins(mins: number): string {
    if (mins <= 0) return "0m";
    const h = Math.floor(mins / 60);
    const m = mins % 60;
    if (h === 0) return `${m}m`;
    if (m === 0) return `${h}h`;
    return `${h}h ${m}m`;
  }
</script>

<div class="scoreboard" style="--project-accent: {projectColor}">
  <WidgetHeader widgetType="p_scoreboard" pickerRange={dateRange} />
  {#if loading}
    <div class="skeleton-grid">
      {#each [0,1,2,3] as i (i)}
        <div class="skeleton-cell">
          <div class="skeleton-num"></div>
          <div class="skeleton-label"></div>
        </div>
      {/each}
    </div>
  {:else if error}
    <p class="err">{error}</p>
  {:else if data}
    <div class="kpi-grid">
      <div class="kpi-cell">
        <span class="kpi-num">{data.tasks_done}</span>
        <span class="kpi-label">DONE</span>
        <span class="accent-line"></span>
      </div>
      <div class="kpi-cell">
        <span class="kpi-num">{data.tasks_remaining}</span>
        <span class="kpi-label">LEFT</span>
        <span class="accent-line"></span>
      </div>
      <div class="kpi-cell">
        <span class="kpi-num">{fmtMins(data.total_time_tracked_mins)}</span>
        <span class="kpi-label">TRACKED</span>
        <span class="accent-line"></span>
      </div>
      <div class="kpi-cell">
        <span class="kpi-num">{fmtMins(data.estimated_time_left_mins)}</span>
        <span class="kpi-label">REMAINING</span>
        <span class="accent-line"></span>
      </div>
    </div>
  {:else}
    <p class="empty">No task data.</p>
  {/if}
</div>

<style>
  .scoreboard {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 8px;
    /* Font sizing below is relative to the CARD, not the viewport, so the
       numbers scale with the widget when it's resized in the grid. */
    container-type: inline-size;
  }

  .kpi-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
    gap: 8px;
    flex: 1;
    min-height: 0;
  }

  .kpi-cell {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    padding: 8px 4px 10px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--db-border);
    position: relative;
    min-width: 0;
    overflow: hidden;
  }

  .kpi-num {
    font-size: clamp(1.1rem, 9cqi, 2.4rem);
    font-weight: 800;
    color: var(--project-accent);
    line-height: 1;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.02em;
    white-space: nowrap;
    max-width: 100%;
  }

  .kpi-label {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.1em;
    color: var(--db-ink-muted);
    text-transform: uppercase;
  }

  .accent-line {
    position: absolute;
    bottom: 0;
    left: 20%;
    right: 20%;
    height: 2px;
    background: var(--project-accent);
    border-radius: 999px;
    opacity: 0.6;
  }

  /* Skeleton */
  .skeleton-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
    gap: 8px;
    flex: 1;
    min-height: 0;
  }

  .skeleton-cell {
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--db-border);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 12px;
  }

  .skeleton-num {
    width: 60%;
    height: 28px;
    border-radius: 6px;
    background: var(--db-border);
    animation: pulse 1.4s ease-in-out infinite;
  }

  .skeleton-label {
    width: 40%;
    height: 10px;
    border-radius: 4px;
    background: var(--db-border);
    animation: pulse 1.4s ease-in-out infinite 0.2s;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 0.8; }
  }

  .err, .empty {
    margin: 0;
    font-size: 12px;
    color: var(--db-ink-muted);
    text-align: center;
    padding: 16px;
  }
  .err { color: #ef4444; }
</style>
