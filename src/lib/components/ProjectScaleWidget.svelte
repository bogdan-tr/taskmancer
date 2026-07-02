<script lang="ts">
  import type { DashboardDateRange } from "$lib/api";
  import { getDashboardProjectSummary } from "$lib/api";
  import type { DashboardProjectSummary } from "$lib/types";
  import WidgetHeader from "./WidgetHeader.svelte";

  interface Props {
    dateRange: DashboardDateRange;
  }
  let { dateRange }: Props = $props();

  let data = $state<DashboardProjectSummary[]>([]);
  let loading = $state(true);
  let error = $state(false);

  $effect(() => {
    loading = true;
    error = false;
    getDashboardProjectSummary(dateRange)
      .then((result) => {
        data = result;
        loading = false;
      })
      .catch(() => {
        error = true;
        loading = false;
      });
  });

  // Donut geometry
  const OUTER_R = 60;
  const INNER_R = 40;
  const CX = 70;
  const CY = 70;
  const SVG_SIZE = 140;

  function polarToCartesian(cx: number, cy: number, r: number, angleDeg: number) {
    const rad = ((angleDeg - 90) * Math.PI) / 180;
    return {
      x: cx + r * Math.cos(rad),
      y: cy + r * Math.sin(rad),
    };
  }

  function arcPath(cx: number, cy: number, outerR: number, innerR: number, startDeg: number, endDeg: number): string {
    // Clamp to avoid degenerate arcs
    const sweep = Math.min(endDeg - startDeg, 359.9999);
    const end = startDeg + sweep;
    const largeArc = sweep > 180 ? 1 : 0;

    const o1 = polarToCartesian(cx, cy, outerR, startDeg);
    const o2 = polarToCartesian(cx, cy, outerR, end);
    const i1 = polarToCartesian(cx, cy, innerR, end);
    const i2 = polarToCartesian(cx, cy, innerR, startDeg);

    return [
      `M ${o1.x} ${o1.y}`,
      `A ${outerR} ${outerR} 0 ${largeArc} 1 ${o2.x} ${o2.y}`,
      `L ${i1.x} ${i1.y}`,
      `A ${innerR} ${innerR} 0 ${largeArc} 0 ${i2.x} ${i2.y}`,
      "Z",
    ].join(" ");
  }

  interface DonutSlice {
    color: string;
    path: string;
  }

  function buildSlices(values: number[], colors: string[]): DonutSlice[] {
    const total = values.reduce((s, v) => s + v, 0);
    if (total === 0) return [];
    const slices: DonutSlice[] = [];
    let angle = 0;
    for (let i = 0; i < values.length; i++) {
      const sweep = (values[i] / total) * 360;
      slices.push({ color: colors[i], path: arcPath(CX, CY, OUTER_R, INNER_R, angle, angle + sweep) });
      angle += sweep;
    }
    return slices;
  }

  let timeSlices = $derived(
    buildSlices(
      data.map((d) => d.time_tracked_minutes),
      data.map((d) => d.project_color),
    ),
  );

  let taskSlices = $derived(
    buildSlices(
      data.map((d) => d.task_count),
      data.map((d) => d.project_color),
    ),
  );

  let estimSlices = $derived(
    buildSlices(
      data.map((d) => d.estimated_minutes_total),
      data.map((d) => d.project_color),
    ),
  );

  let totalTime = $derived(data.reduce((s, d) => s + d.time_tracked_minutes, 0));
  let totalTasks = $derived(data.reduce((s, d) => s + d.task_count, 0));
  let totalEstim = $derived(data.reduce((s, d) => s + d.estimated_minutes_total, 0));

  function fmtTime(mins: number): string {
    if (mins === 0) return "0h";
    const h = Math.floor(mins / 60);
    const m = mins % 60;
    return h > 0 ? (m > 0 ? `${h}h ${m}m` : `${h}h`) : `${m}m`;
  }
</script>

<div class="widget">
  <WidgetHeader widgetType="project_scale" pickerRange={dateRange} />

  {#if loading}
    <div class="state-msg">Loading…</div>
  {:else if error}
    <div class="state-msg state-err">Failed to load data</div>
  {:else if data.length === 0}
    <div class="state-msg">No project data yet</div>
  {:else}
    <div class="donuts-row">
      <!-- Donut 1: Estimated -->
      <div class="donut-wrap">
        <svg viewBox="0 0 {SVG_SIZE} {SVG_SIZE}" class="donut-svg" aria-label="Estimated minutes by project">
          {#each estimSlices as slice, i (i)}
            <path d={slice.path} fill={slice.color} />
          {/each}
          {#if estimSlices.length === 0}
            <circle cx={CX} cy={CY} r={OUTER_R} fill="var(--db-grid-line, rgba(255,255,255,0.06))" />
          {/if}
          <text x={CX} y={CY - 6} text-anchor="middle" font-size="15" font-weight="700" fill="var(--db-ink, #e6edf3)">
            {fmtTime(totalEstim)}
          </text>
          <text x={CX} y={CY + 10} text-anchor="middle" font-size="14" fill="var(--db-ink-muted, #8b949e)">estimated</text>
        </svg>
        <span class="donut-title">Estimated</span>
      </div>

      <!-- Donut 2: Time Tracked -->
      <div class="donut-wrap">
        <svg viewBox="0 0 {SVG_SIZE} {SVG_SIZE}" class="donut-svg" aria-label="Time tracked by project">
          {#each timeSlices as slice, i (i)}
            <path d={slice.path} fill={slice.color} />
          {/each}
          {#if timeSlices.length === 0}
            <circle cx={CX} cy={CY} r={OUTER_R} fill="var(--db-grid-line, rgba(255,255,255,0.06))" />
          {/if}
          <text x={CX} y={CY - 6} text-anchor="middle" font-size="15" font-weight="700" fill="var(--db-ink, #e6edf3)">
            {fmtTime(totalTime)}
          </text>
          <text x={CX} y={CY + 10} text-anchor="middle" font-size="14" fill="var(--db-ink-muted, #8b949e)">tracked</text>
        </svg>
        <span class="donut-title">Time Tracked</span>
      </div>

      <!-- Donut 3: Task Count -->
      <div class="donut-wrap">
        <svg viewBox="0 0 {SVG_SIZE} {SVG_SIZE}" class="donut-svg" aria-label="Task count by project">
          {#each taskSlices as slice, i (i)}
            <path d={slice.path} fill={slice.color} />
          {/each}
          {#if taskSlices.length === 0}
            <circle cx={CX} cy={CY} r={OUTER_R} fill="var(--db-grid-line, rgba(255,255,255,0.06))" />
          {/if}
          <text x={CX} y={CY - 6} text-anchor="middle" font-size="15" font-weight="700" fill="var(--db-ink, #e6edf3)">
            {totalTasks}
          </text>
          <text x={CX} y={CY + 10} text-anchor="middle" font-size="14" fill="var(--db-ink-muted, #8b949e)">tasks</text>
        </svg>
        <span class="donut-title">Task Count</span>
      </div>
    </div>

    <!-- Shared legend -->
    <div class="legend">
      {#each data as project (project.project_id)}
        <span class="legend-item">
          <span class="dot" style="background:{project.project_color}"></span>
          {project.project_name}
        </span>
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

  .donuts-row {
    display: flex;
    gap: 8px;
    flex: 1;
    min-height: 0;
    align-items: stretch;
    justify-content: space-around;
  }

  .donut-wrap {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }

  .donut-svg {
    width: 100%;
    height: 100%;
    flex: 1;
    min-height: 0;
    max-width: 200px;
  }

  .donut-title {
    font-size: 13px;
    color: var(--db-ink-muted, #8b949e);
    text-align: center;
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 12px;
    justify-content: center;
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 13px;
    color: var(--db-ink-muted, #8b949e);
    white-space: nowrap;
  }

  .dot {
    display: inline-block;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    flex-shrink: 0;
  }

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
