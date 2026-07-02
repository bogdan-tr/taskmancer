<script lang="ts">
  import { getProjectCompletionTrend } from "$lib/api";
  import type { DashboardDateRange } from "$lib/api";
  import type { ProjectCompletionWeek } from "$lib/types";
  import WidgetHeader from "./WidgetHeader.svelte";

  interface Props {
    projectId: string;
    projectColor: string;
    dateRange: DashboardDateRange;
  }
  let { projectId, projectColor, dateRange }: Props = $props();

  let data = $state<ProjectCompletionWeek[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    if (!projectId) return;
    loading = true;
    error = null;
    getProjectCompletionTrend(projectId, dateRange)
      .then((d) => { data = d; })
      .catch((e) => { error = e instanceof Error ? e.message : String(e); })
      .finally(() => { loading = false; });
  });

  const PAD_L = 28;
  const PAD_R = 4;
  const PAD_T = 8;
  const PAD_B = 30;

  let chartW = $state(0);
  let chartH = $state(130);
  let SVG_W = $derived(Math.max(200, chartW));
  let SVG_H = $derived(Math.max(90, chartH));
  let CW = $derived(SVG_W - PAD_L - PAD_R);
  let CH = $derived(SVG_H - PAD_T - PAD_B);

  let maxCount = $derived(Math.max(...data.map((w) => w.count), 1));

  function barX(i: number): number {
    const slot = CW / data.length;
    return PAD_L + i * slot + slot * 0.12;
  }

  function bW(): number {
    return Math.max(4, (CW / data.length) * 0.76);
  }

  function barH(count: number): number {
    return (count / maxCount) * CH;
  }

  function barY(count: number): number {
    return PAD_T + CH - barH(count);
  }

  let hasAnyData = $derived(data.some((w) => w.count > 0));

  // 4-week rolling average line
  let avgPoints = $derived((): string => {
    if (data.length === 0) return "";
    return data
      .map((w, i) => {
        const slice = data.slice(Math.max(0, i - 3), i + 1);
        const avg = slice.reduce((s, x) => s + x.count, 0) / slice.length;
        const slot = CW / data.length;
        const cx = PAD_L + i * slot + slot / 2;
        const cy = PAD_T + CH - (avg / maxCount) * CH;
        return `${cx},${cy}`;
      })
      .join(" ");
  });
</script>

<div class="w14" style="--project-accent: {projectColor}">
  <WidgetHeader widgetType="p_completion_trend" pickerRange={dateRange} />
  {#if loading}
    <div class="state-msg">Loading…</div>
  {:else if error}
    <div class="state-msg state-err">{error}</div>
  {:else if data.length > 0}
    <div class="chart-area" bind:clientWidth={chartW} bind:clientHeight={chartH}>
      <svg width={SVG_W} height={SVG_H} viewBox="0 0 {SVG_W} {SVG_H}">
        <!-- Y grid: 0 and max -->
        <line x1={PAD_L} x2={SVG_W - PAD_R} y1={PAD_T} y2={PAD_T} stroke="var(--db-grid-line,#ffffff14)" stroke-width="1" />
        <text x={PAD_L - 3} y={PAD_T + 4} text-anchor="end" font-size="9" fill="var(--db-ink-muted)">{maxCount}</text>

        <!-- Bars -->
        {#each data as w, i}
          {@const isCurrent = w.is_current}
          <rect
            x={barX(i)}
            y={barY(w.count)}
            width={bW()}
            height={barH(w.count)}
            rx="2"
            fill={isCurrent
              ? "var(--project-accent)"
              : "color-mix(in srgb, var(--project-accent) 45%, transparent)"}
          />
          <!-- X labels: every 4 weeks + current -->
          {#if i % 4 === 0 || isCurrent}
            <text
              x={barX(i) + bW() / 2}
              y={SVG_H - PAD_B + 12}
              text-anchor="middle"
              font-size="8"
              font-weight={isCurrent ? "700" : "400"}
              fill={isCurrent ? "var(--project-accent)" : "var(--db-ink-muted)"}
            >{w.week_label}</text>
          {/if}
        {/each}

        <!-- Rolling average line -->
        {#if hasAnyData}
          <polyline
            points={avgPoints()}
            fill="none"
            stroke="var(--db-ink)"
            stroke-width="1.5"
            stroke-linejoin="round"
            opacity="0.5"
          />
        {/if}

        <!-- Y axis -->
        <line x1={PAD_L} x2={PAD_L} y1={PAD_T} y2={SVG_H - PAD_B} stroke="var(--db-border)" stroke-width="1" />
      </svg>
    </div>
    <div class="footer">
      <span class="leg"><span class="avg-line"></span>4-wk avg</span>
      <span class="sub">13-week window</span>
    </div>
  {:else}
    <div class="state-msg">No data</div>
  {/if}
</div>

<style>
  .w14 {
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
  .chart-area svg { display: block; }
  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-shrink: 0;
  }
  .leg {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 10px;
    color: var(--db-ink-muted);
  }
  .avg-line {
    display: inline-block;
    width: 16px;
    height: 1.5px;
    background: var(--db-ink-muted);
    opacity: 0.6;
    border-radius: 1px;
  }
  .sub {
    font-size: 10px;
    color: var(--db-ink-muted);
    opacity: 0.6;
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
