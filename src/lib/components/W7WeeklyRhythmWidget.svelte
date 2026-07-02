<script lang="ts">
  import { getProjectWeeklyRhythm } from "$lib/api";
  import type { DashboardDateRange } from "$lib/api";
  import type { ProjectWeeklyRhythm } from "$lib/types";
  import WidgetHeader from "./WidgetHeader.svelte";

  interface Props {
    projectId: string;
    projectColor: string;
    dateRange: DashboardDateRange;
  }
  let { projectId, projectColor, dateRange }: Props = $props();

  let data = $state<ProjectWeeklyRhythm | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    if (!projectId) return;
    loading = true;
    error = null;
    getProjectWeeklyRhythm(projectId, dateRange)
      .then((d) => { data = d; })
      .catch((e) => { error = e instanceof Error ? e.message : String(e); })
      .finally(() => { loading = false; });
  });

  const DAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
  const PAD_L = 36;
  const PAD_R = 8;
  const PAD_T = 8;
  const PAD_B = 28;

  let chartW = $state(0);
  let chartH = $state(0);
  let SVG_W = $derived(Math.max(200, chartW));
  // Height tracks the card so the bars fill it — a fixed height left a dead
  // band under the chart at taller card sizes.
  let SVG_H = $derived(Math.max(110, chartH));

  let maxHours = $derived(
    data ? Math.max(...data.weekday_hours, 0.5) : 1,
  );

  function barX(i: number): number {
    const w = SVG_W - PAD_L - PAD_R;
    const barW = w / 7;
    return PAD_L + i * barW + barW * 0.15;
  }

  function barW(): number {
    const w = SVG_W - PAD_L - PAD_R;
    return (w / 7) * 0.7;
  }

  function barH(h: number): number {
    return ((SVG_H - PAD_T - PAD_B) * h) / maxHours;
  }

  function barY(h: number): number {
    return SVG_H - PAD_B - barH(h);
  }

  function gridY(val: number): number {
    return SVG_H - PAD_B - ((SVG_H - PAD_T - PAD_B) * val) / maxHours;
  }

  let gridLines = $derived((): number[] => {
    if (!data) return [];
    const step = maxHours <= 2 ? 0.5 : maxHours <= 5 ? 1 : 2;
    const lines: number[] = [];
    let v = step;
    while (v <= maxHours) {
      lines.push(v);
      v += step;
    }
    return lines;
  });
</script>

<div class="w7" style="--project-accent: {projectColor}">
  <WidgetHeader widgetType="p_weekly_rhythm" pickerRange={dateRange} />
  {#if loading}
    <div class="state-msg">Loading…</div>
  {:else if error}
    <div class="state-msg state-err">{error}</div>
  {:else if data}
    {@const hours = data.weekday_hours}
    {@const todayWd = data.today_weekday}
    <div class="chart-area" bind:clientWidth={chartW} bind:clientHeight={chartH}>
      <svg width={SVG_W} height={SVG_H} viewBox="0 0 {SVG_W} {SVG_H}">
        <!-- Y-axis grid lines -->
        {#each gridLines() as gv}
          <line
            x1={PAD_L}
            x2={SVG_W - PAD_R}
            y1={gridY(gv)}
            y2={gridY(gv)}
            stroke="var(--db-grid-line, #ffffff14)"
            stroke-width="1"
          />
          <text
            x={PAD_L - 4}
            y={gridY(gv) + 4}
            text-anchor="end"
            font-size="9"
            fill="var(--db-ink-muted)"
          >{gv.toFixed(gv % 1 === 0 ? 0 : 1)}h</text>
        {/each}

        <!-- Bars -->
        {#each hours as h, i}
          {@const isToday = i === todayWd}
          <rect
            x={barX(i)}
            y={barY(h)}
            width={barW()}
            height={barH(h)}
            rx="3"
            fill={isToday ? "var(--project-accent)" : "color-mix(in srgb, var(--project-accent) 40%, transparent)"}
          />
          <!-- Day label -->
          <text
            x={barX(i) + barW() / 2}
            y={SVG_H - PAD_B + 14}
            text-anchor="middle"
            font-size="10"
            font-weight={isToday ? "700" : "400"}
            fill={isToday ? "var(--project-accent)" : "var(--db-ink-muted)"}
          >{DAYS[i]}</text>
          <!-- Value label on bar if tall enough -->
          {#if h > 0 && barH(h) > 14}
            <text
              x={barX(i) + barW() / 2}
              y={barY(h) - 3}
              text-anchor="middle"
              font-size="8"
              fill={isToday ? "var(--project-accent)" : "var(--db-ink-muted)"}
            >{h.toFixed(1)}</text>
          {/if}
        {/each}

        <!-- Y-axis line -->
        <line
          x1={PAD_L}
          x2={PAD_L}
          y1={PAD_T}
          y2={SVG_H - PAD_B}
          stroke="var(--db-border)"
          stroke-width="1"
        />
      </svg>
    </div>
    <div class="footer-note">avg tracked hrs/weekday</div>
  {:else}
    <div class="state-msg">No data</div>
  {/if}
</div>

<style>
  .w7 {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .chart-area {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .chart-area svg {
    display: block;
  }
  .footer-note {
    font-size: 10px;
    color: var(--db-ink-muted);
    text-align: center;
    flex-shrink: 0;
  }
  .state-msg {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    color: var(--db-ink-muted);
  }
  .state-err { color: #ef4444; }
</style>
