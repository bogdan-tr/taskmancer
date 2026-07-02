<script lang="ts">
  import { getDashboardCompletionsByProject } from "$lib/api";
  import type { DashboardDateRange } from "$lib/api";
  import type { DashboardProjectCompletions } from "$lib/types";
  import WidgetHeader from "./WidgetHeader.svelte";

  interface Props {
    dateRange: DashboardDateRange;
  }
  let { dateRange }: Props = $props();

  let data = $state<DashboardProjectCompletions[]>([]);
  let loading = $state(true);
  let error = $state(false);

  $effect(() => {
    loading = true;
    error = false;
    getDashboardCompletionsByProject(dateRange)
      .then((result) => {
        data = result;
        loading = false;
      })
      .catch(() => {
        error = true;
        loading = false;
      });
  });

  // SVG layout — responsive width bound to container
  let chartAreaW = $state(600);
  let SVG_W = $derived(Math.max(320, chartAreaW));

  // Label zone: 0 → LABEL_W (≈ 21% of SVG_W)
  let LABEL_W = $derived(Math.round(SVG_W * 0.21));
  let DOT_X = $derived(LABEL_W - 5);

  // Bar zone: LABEL_W+9 → SVG_W, center halfway across bar zone
  let BAR_ZONE_START = $derived(LABEL_W + 9);
  let BAR_ZONE_W = $derived(SVG_W - BAR_ZONE_START);
  let CENTER_X = $derived(BAR_ZONE_START + Math.round(BAR_ZONE_W / 2));
  let BAR_MAX_W = $derived(Math.round(BAR_ZONE_W / 2) - 14);

  let maxVal = $derived(
    data.length === 0
      ? 1
      : Math.max(1, ...data.map((d) => Math.max(d.completed, d.cancelled))),
  );

  function barW(count: number): number {
    return (count / maxVal) * BAR_MAX_W;
  }

  // Rows fill the full container height; bar height is capped so they're never huge
  let chartAreaH = $state(240);

  let rowH = $derived(
    data.length === 0
      ? 52
      : Math.max(24, Math.floor(chartAreaH / data.length)),
  );

  // Cap bar height at 32px regardless of how tall each row gets
  let barH = $derived(Math.max(12, Math.min(32, Math.floor(rowH * 0.45))));
  let svgH = $derived(data.length * rowH);
</script>

<div class="widget">
  <WidgetHeader widgetType="completion_overview" pickerRange={dateRange} />

  {#if loading}
    <div class="state-msg">Loading…</div>
  {:else if error}
    <div class="state-msg state-err">Failed to load data</div>
  {:else if data.length === 0}
    <div class="state-msg">No completed tasks in this period</div>
  {:else}
    <div class="chart-area" bind:clientHeight={chartAreaH} bind:clientWidth={chartAreaW}>
      <svg
        width={SVG_W}
        height={svgH}
        viewBox="0 0 {SVG_W} {svgH}"
        aria-label="Completion overview chart"
        role="img"
      >
        <defs>
          <!-- Single gradient defs for all rows -->
          <linearGradient id="cov-comp-grad" x1="0" y1="0" x2="1" y2="0">
            <stop offset="0%" stop-color="#10b981" stop-opacity="0.45" />
            <stop offset="100%" stop-color="#10b981" stop-opacity="1" />
          </linearGradient>
          <linearGradient id="cov-canc-grad" x1="1" y1="0" x2="0" y2="0">
            <stop offset="0%" stop-color="#f59e0b" stop-opacity="0.45" />
            <stop offset="100%" stop-color="#f59e0b" stop-opacity="1" />
          </linearGradient>
        </defs>

        <!-- Center axis line -->
        <line
          x1={CENTER_X}
          y1={0}
          x2={CENTER_X}
          y2={svgH}
          stroke="var(--db-grid-line, rgba(255,255,255,0.1))"
          stroke-width="1"
        />

        {#each data as row, i (row.project_id)}
          {@const y = i * rowH}
          {@const midY = y + rowH / 2}

          <!-- Alternating row tint for readability without gaps -->
          {#if i % 2 === 1}
            <rect x={0} y={y} width={SVG_W} height={rowH} fill="rgba(255,255,255,0.025)" />
          {/if}
          {@const bY = midY - barH / 2}
          {@const cw = barW(row.completed)}
          {@const aw = barW(row.cancelled)}
          {@const fontSize = Math.max(13, Math.min(16, Math.floor(rowH * 0.32)))}

          <!-- Project name — RIGHT-ALIGNED in label zone (never reaches bar zone) -->
          <text
            x={LABEL_W - 14}
            y={midY + fontSize * 0.38}
            font-size={fontSize}
            text-anchor="end"
            fill="var(--db-ink, #e6edf3)"
          >{row.project_name}</text>

          <!-- Color dot — fixed radius so it never overlaps the name -->
          <circle
            cx={DOT_X}
            cy={midY}
            r={5}
            fill={row.project_color}
          />

          <!-- Completion bar (grows RIGHT from center) -->
          {#if row.completed > 0}
            <rect
              x={CENTER_X}
              y={bY}
              width={cw}
              height={barH}
              rx="3"
              fill="url(#cov-comp-grad)"
            />
            <text
              x={CENTER_X + cw + 6}
              y={midY + fontSize * 0.38}
              font-size={Math.max(12, fontSize - 1)}
              fill="#10b981"
            >{row.completed}</text>
          {/if}

          <!-- Cancellation bar (grows LEFT from center) -->
          {#if row.cancelled > 0}
            <rect
              x={CENTER_X - aw}
              y={bY}
              width={aw}
              height={barH}
              rx="3"
              fill="url(#cov-canc-grad)"
            />
            <text
              x={CENTER_X - aw - 6}
              y={midY + fontSize * 0.38}
              font-size={Math.max(12, fontSize - 1)}
              text-anchor="end"
              fill="#f59e0b"
            >{row.cancelled}</text>
          {/if}
        {/each}
      </svg>
    </div>

    <div class="legend-row">
      <span class="legend-item"><span class="dot" style="background:#10b981"></span> Completed</span>
      <span class="legend-item"><span class="dot" style="background:#f59e0b"></span> Cancelled</span>
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

  .chart-area {
    flex: 1;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
  }

  .chart-area svg {
    display: block;
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

  .legend-row {
    display: flex;
    gap: 16px;
    justify-content: center;
    flex-shrink: 0;
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 13px;
    color: var(--db-ink-muted, #8b949e);
  }

  .dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
</style>
