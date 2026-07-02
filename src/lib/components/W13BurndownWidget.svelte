<script lang="ts">
  import { getProjectBurndown } from "$lib/api";
  import type { ProjectBurndown } from "$lib/types";
  import WidgetHeader from "./WidgetHeader.svelte";

  interface Props {
    projectId: string;
    projectColor: string;
  }
  let { projectId, projectColor }: Props = $props();

  let data = $state<ProjectBurndown | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    if (!projectId) return;
    loading = true;
    error = null;
    getProjectBurndown(projectId)
      .then((d) => { data = d; })
      .catch((e) => { error = e instanceof Error ? e.message : String(e); })
      .finally(() => { loading = false; });
  });

  // ── Responsive chart geometry ─────────────────────────────────────────────
  let areaW = $state(420);
  let areaH = $state(200);
  let SVG_W = $derived(Math.max(280, areaW));
  let SVG_H = $derived(Math.max(120, areaH));
  const PAD_L = 34;
  const PAD_R = 10;
  const PAD_T = 10;
  const PAD_B = 20;
  let CHART_W = $derived(SVG_W - PAD_L - PAD_R);
  let CHART_H = $derived(SVG_H - PAD_T - PAD_B);

  let points = $derived(data?.points ?? []);
  let maxHours = $derived(
    Math.max(1, ...points.map((p) => Math.max(p.remaining_hours, p.ideal_hours))),
  );

  function xPos(i: number): number {
    if (points.length <= 1) return PAD_L;
    return PAD_L + (i / (points.length - 1)) * CHART_W;
  }
  function yPos(hours: number): number {
    return PAD_T + CHART_H * (1 - hours / maxHours);
  }

  let actualPath = $derived.by(() => {
    if (points.length === 0) return "";
    return points.map((p, i) => `${i === 0 ? "M" : "L"} ${xPos(i)} ${yPos(p.remaining_hours)}`).join(" ");
  });

  let actualArea = $derived.by(() => {
    if (points.length === 0) return "";
    const base = PAD_T + CHART_H;
    return `${actualPath} L ${xPos(points.length - 1)} ${base} L ${xPos(0)} ${base} Z`;
  });

  let idealPath = $derived.by(() => {
    if (points.length === 0) return "";
    return points.map((p, i) => `${i === 0 ? "M" : "L"} ${xPos(i)} ${yPos(p.ideal_hours)}`).join(" ");
  });

  /** Today marker + ahead/behind assessment at today's (or the latest past) point. */
  let todayInfo = $derived.by(() => {
    if (!data || points.length === 0) return null;
    const today = new Date().toISOString().slice(0, 10);
    let idx = -1;
    for (let i = 0; i < points.length; i++) {
      if (points[i].date <= today) idx = i;
    }
    if (idx === -1) return null;
    const p = points[idx];
    const diff = p.remaining_hours - p.ideal_hours; // + = behind, − = ahead
    return { x: xPos(idx), diff, remaining: p.remaining_hours };
  });

  function fmtH(h: number): string {
    const abs = Math.abs(h);
    return abs >= 10 ? `${abs.toFixed(0)}h` : `${abs.toFixed(1)}h`;
  }

  /** Y-axis gridline hour values: 3 evenly spaced ticks + zero. */
  let yTicks = $derived.by(() => {
    const step = maxHours / 3;
    return [0, step, step * 2, maxHours];
  });

  function fmtAxisDate(iso: string): string {
    const d = new Date(iso + "T00:00:00");
    return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  }
</script>

<div class="w13" style="--project-accent: {projectColor}">
  <WidgetHeader widgetType="p_burndown" />
  {#if loading}
    <div class="state-msg">Loading…</div>
  {:else if error}
    <div class="state-msg state-err">{error}</div>
  {:else if data && points.length > 1}
    <div class="chart-meta">
      {#if todayInfo}
        <span class="pace-badge" class:behind={todayInfo.diff > 0.05} class:ahead={todayInfo.diff < -0.05}>
          {#if todayInfo.diff > 0.05}
            ▲ {fmtH(todayInfo.diff)} behind pace
          {:else if todayInfo.diff < -0.05}
            ▼ {fmtH(todayInfo.diff)} ahead of pace
          {:else}
            ● on pace
          {/if}
        </span>
      {/if}
      <span class="legend-item"><span class="lg-line lg-actual"></span>remaining</span>
      <span class="legend-item"><span class="lg-line lg-ideal"></span>ideal to {data.has_deadline ? "deadline" : "last due"}</span>
    </div>
    <div class="chart-area" bind:clientWidth={areaW} bind:clientHeight={areaH}>
      <svg width={SVG_W} height={SVG_H} viewBox="0 0 {SVG_W} {SVG_H}" role="img"
        aria-label="Burndown: estimated hours remaining over time">
        <defs>
          <linearGradient id="w13-area-{projectId}" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stop-color={projectColor} stop-opacity="0.35" />
            <stop offset="100%" stop-color={projectColor} stop-opacity="0.02" />
          </linearGradient>
        </defs>

        <!-- Y gridlines + hour labels -->
        {#each yTicks as t}
          <line x1={PAD_L} y1={yPos(t)} x2={SVG_W - PAD_R} y2={yPos(t)}
            stroke="var(--db-grid-line, rgba(255,255,255,0.05))" stroke-width="1" />
          <text x={PAD_L - 6} y={yPos(t) + 3} text-anchor="end" class="axis-label">{fmtH(t)}</text>
        {/each}

        <!-- Today marker -->
        {#if todayInfo}
          <line x1={todayInfo.x} y1={PAD_T} x2={todayInfo.x} y2={PAD_T + CHART_H}
            stroke="var(--db-ink-muted, #8b949e)" stroke-opacity="0.45"
            stroke-width="1" stroke-dasharray="2 3" />
          <text x={todayInfo.x} y={PAD_T + 8} text-anchor="middle" class="today-label">TODAY</text>
        {/if}

        <!-- Actual burndown -->
        <path d={actualArea} fill="url(#w13-area-{projectId})" />
        <path d={actualPath} fill="none" stroke={projectColor} stroke-width="2.25"
          stroke-linejoin="round" stroke-linecap="round" />

        <!-- Ideal pace -->
        <path d={idealPath} fill="none" stroke="var(--db-ink, #e6edf3)" stroke-opacity="0.55"
          stroke-width="1.5" stroke-dasharray="5 4" />

        <!-- X-axis start / end labels -->
        <text x={PAD_L} y={SVG_H - 5} class="axis-label">{fmtAxisDate(data.start_date)}</text>
        <text x={SVG_W - PAD_R} y={SVG_H - 5} text-anchor="end" class="axis-label">
          {fmtAxisDate(data.end_date)}{data.has_deadline ? " ⚑" : ""}
        </text>
      </svg>
    </div>
  {:else}
    <div class="state-msg">
      <p class="empty-title">No burndown yet</p>
      <p class="empty-hint">
        Add time estimates to one-off tasks, plus due dates or a project deadline,
        and the remaining-work line appears here.
      </p>
    </div>
  {/if}
</div>

<style>
  .w13 {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .chart-meta {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
    flex-wrap: wrap;
  }

  .pace-badge {
    font-size: 9.5px;
    font-weight: 800;
    letter-spacing: 0.04em;
    padding: 2px 8px;
    border-radius: 999px;
    color: var(--db-ink-muted, #8b949e);
    border: 1px solid var(--db-border, rgba(255, 255, 255, 0.1));
    white-space: nowrap;
  }

  .pace-badge.behind {
    color: #ef4444;
    border-color: rgba(239, 68, 68, 0.4);
    background: rgba(239, 68, 68, 0.08);
  }

  .pace-badge.ahead {
    color: #22c55e;
    border-color: rgba(34, 197, 94, 0.35);
    background: rgba(34, 197, 94, 0.08);
  }

  .legend-item {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 9px;
    color: var(--db-ink-muted, #8b949e);
    white-space: nowrap;
  }

  .lg-line {
    width: 14px;
    height: 0;
    border-top: 2px solid;
  }

  .lg-actual { border-color: var(--project-accent); }
  .lg-ideal {
    border-top-style: dashed;
    border-color: color-mix(in srgb, var(--db-ink, #e6edf3) 55%, transparent);
  }

  .chart-area {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .chart-area svg {
    display: block;
  }

  .axis-label {
    font-size: 9px;
    fill: var(--db-ink-muted, #8b949e);
    font-variant-numeric: tabular-nums;
  }

  .today-label {
    font-size: 7.5px;
    font-weight: 700;
    letter-spacing: 0.1em;
    fill: var(--db-ink-muted, #8b949e);
  }

  .state-msg {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    color: var(--db-ink-muted, #8b949e);
    text-align: center;
    padding: 0 16px;
  }

  .empty-title {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--db-ink, #e6edf3);
  }

  .empty-hint {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.5;
  }

  .state-err { color: #ef4444; }
</style>
