<script lang="ts">
  import { getProjectDueTimeline } from "$lib/api";
  import type { ProjectDueDateTimeline, ProjectDueDatePoint } from "$lib/types";

  interface Props {
    projectId: string;
    projectColor: string;
  }
  let { projectId, projectColor }: Props = $props();

  let data = $state<ProjectDueDateTimeline | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    if (!projectId) return;
    loading = true;
    error = null;
    getProjectDueTimeline(projectId)
      .then((d) => { data = d; })
      .catch((e) => { error = e instanceof Error ? e.message : String(e); })
      .finally(() => { loading = false; });
  });

  const PAD_L = 4;
  const PAD_R = 4;
  const PAD_T = 6;
  const PAD_B = 32;

  let chartW = $state(0);
  let chartH = $state(120);
  let SVG_W = $derived(Math.max(200, chartW));
  let SVG_H = $derived(Math.max(100, chartH));
  let CHART_W = $derived(SVG_W - PAD_L - PAD_R);
  let CHART_H = $derived(SVG_H - PAD_T - PAD_B);

  let visiblePoints = $derived((): ProjectDueDatePoint[] => {
    if (!data || data.points.length === 0) return [];
    // Show up to 30 points centred around today
    const today = data.today;
    const pts = data.points;
    if (pts.length <= 30) return pts;
    const todayIdx = pts.findIndex((p) => p.date >= today);
    const anchor = todayIdx < 0 ? pts.length - 1 : todayIdx;
    const start = Math.max(0, anchor - 14);
    return pts.slice(start, start + 30);
  });

  let maxCount = $derived(
    visiblePoints().reduce((m, p) => Math.max(m, p.count), 1),
  );

  function xPos(i: number, total: number): number {
    if (total <= 1) return PAD_L + CHART_W / 2;
    return PAD_L + (i / (total - 1)) * CHART_W;
  }

  function barW(total: number): number {
    if (total <= 1) return Math.min(CHART_W, 20);
    return Math.max(4, (CHART_W / total) * 0.7);
  }

  function yPos(count: number): number {
    return PAD_T + CHART_H - (count / maxCount) * CHART_H;
  }

  function todayX(): number {
    const pts = visiblePoints();
    if (!data || pts.length === 0) return -1;
    const idx = pts.findIndex((p) => p.date === data!.today);
    if (idx < 0) return -1;
    return xPos(idx, pts.length);
  }

  function fmtDate(d: string): string {
    const [, m, day] = d.split("-");
    const months = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];
    return `${months[parseInt(m) - 1]} ${parseInt(day)}`;
  }
</script>

<div class="w12" style="--project-accent: {projectColor}">
  <span class="widget-label">DUE-DATE TIMELINE</span>
  {#if loading}
    <div class="state-msg">Loading…</div>
  {:else if error}
    <div class="state-msg state-err">{error}</div>
  {:else if data && data.points.length > 0}
    {@const pts = visiblePoints()}
    {@const n = pts.length}
    <div class="chart-area" bind:clientWidth={chartW} bind:clientHeight={chartH}>
      <svg width={SVG_W} height={SVG_H} viewBox="0 0 {SVG_W} {SVG_H}">
        <!-- today line -->
        {#if todayX() >= 0}
          <line
            x1={todayX()} x2={todayX()}
            y1={PAD_T} y2={SVG_H - PAD_B}
            stroke="var(--project-accent)"
            stroke-width="1.5"
            stroke-dasharray="3 3"
            opacity="0.7"
          />
        {/if}

        <!-- Bars: done (green), overdue (red), pending (accent) -->
        {#each pts as pt, i}
          {@const x = xPos(i, n)}
          {@const bw = barW(n)}
          {@const totalH = (pt.count / maxCount) * CHART_H}
          {@const doneH = (pt.done_count / maxCount) * CHART_H}
          {@const overdueH = (pt.overdue_count / maxCount) * CHART_H}
          {@const pendingH = totalH - doneH - overdueH}

          <!-- pending portion -->
          {#if pendingH > 0}
            <rect
              x={x - bw / 2} y={yPos(pt.count)}
              width={bw} height={pendingH}
              rx="2"
              fill="color-mix(in srgb, var(--project-accent) 60%, transparent)"
            />
          {/if}
          <!-- overdue portion -->
          {#if overdueH > 0}
            <rect
              x={x - bw / 2} y={yPos(pt.count) + pendingH}
              width={bw} height={overdueH}
              rx="2"
              fill="#ef444488"
            />
          {/if}
          <!-- done portion -->
          {#if doneH > 0}
            <rect
              x={x - bw / 2} y={SVG_H - PAD_B - doneH}
              width={bw} height={doneH}
              rx="2"
              fill="#22c55e88"
            />
          {/if}
        {/each}

        <!-- X-axis labels: first, today, last -->
        {#if n > 0}
          <text x={xPos(0, n)} y={SVG_H - PAD_B + 12} text-anchor="middle" font-size="9" fill="var(--db-ink-muted)">{fmtDate(pts[0].date)}</text>
          <text x={xPos(n - 1, n)} y={SVG_H - PAD_B + 12} text-anchor="middle" font-size="9" fill="var(--db-ink-muted)">{fmtDate(pts[n - 1].date)}</text>
        {/if}
        {#if todayX() >= 0}
          <text x={todayX()} y={SVG_H - PAD_B + 22} text-anchor="middle" font-size="9" font-weight="600" fill="var(--project-accent)">today</text>
        {/if}

        <!-- Deadline marker -->
        {#if data.deadline}
          {@const dlIdx = pts.findIndex((p) => p.date === data!.deadline)}
          {#if dlIdx >= 0}
            {@const dlX = xPos(dlIdx, n)}
            <line x1={dlX} x2={dlX} y1={PAD_T} y2={SVG_H - PAD_B} stroke="#f59e0b" stroke-width="2" stroke-dasharray="4 2" />
            <text x={dlX} y={PAD_T - 1} text-anchor="middle" font-size="8" fill="#f59e0b">deadline</text>
          {/if}
        {/if}
      </svg>
    </div>

    <!-- Legend -->
    <div class="legend">
      <span class="leg-item"><span class="leg-dot" style="background:color-mix(in srgb, var(--project-accent) 60%, transparent)"></span>pending</span>
      <span class="leg-item"><span class="leg-dot" style="background:#ef4444aa"></span>overdue</span>
      <span class="leg-item"><span class="leg-dot" style="background:#22c55eaa"></span>done</span>
    </div>
  {:else if data}
    <div class="state-msg">No due dates set</div>
  {:else}
    <div class="state-msg">No data</div>
  {/if}
</div>

<style>
  .w12 {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .widget-label {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--db-ink-muted);
    flex-shrink: 0;
  }
  .chart-area {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .chart-area svg { display: block; }
  .legend {
    display: flex;
    gap: 12px;
    flex-shrink: 0;
    justify-content: center;
  }
  .leg-item {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    color: var(--db-ink-muted);
  }
  .leg-dot {
    width: 8px;
    height: 8px;
    border-radius: 2px;
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
