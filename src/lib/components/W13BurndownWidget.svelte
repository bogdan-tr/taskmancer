<script lang="ts">
  import { getProjectBurndown } from "$lib/api";
  import type { ProjectBurndown, ProjectBurndownPoint } from "$lib/types";

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

  const PAD_L = 38;
  const PAD_R = 8;
  const PAD_T = 8;
  const PAD_B = 28;

  let chartW = $state(0);
  let chartH = $state(140);
  let SVG_W = $derived(Math.max(240, chartW));
  let SVG_H = $derived(Math.max(100, chartH));
  let CW = $derived(SVG_W - PAD_L - PAD_R);
  let CH = $derived(SVG_H - PAD_T - PAD_B);

  let maxHours = $derived((): number => {
    if (!data || data.points.length === 0) return 1;
    return Math.max(...data.points.map((p) => Math.max(p.remaining_hours, p.ideal_hours)), 1);
  });

  function xPos(i: number, total: number): number {
    if (total <= 1) return PAD_L;
    return PAD_L + (i / (total - 1)) * CW;
  }

  function yPos(h: number): number {
    return PAD_T + CH - (h / maxHours()) * CH;
  }

  function polyline(getter: (p: ProjectBurndownPoint) => number): string {
    if (!data || data.points.length === 0) return "";
    return data.points
      .map((p, i) => `${xPos(i, data!.points.length)},${yPos(getter(p))}`)
      .join(" ");
  }

  function gridStepHours(): number {
    const m = maxHours();
    if (m <= 4) return 1;
    if (m <= 10) return 2;
    if (m <= 20) return 5;
    return Math.ceil(m / 4 / 5) * 5;
  }

  let gridLines = $derived((): number[] => {
    const step = gridStepHours();
    const lines: number[] = [];
    let v = step;
    while (v <= maxHours()) {
      lines.push(v);
      v += step;
    }
    return lines;
  });

  function fmtDate(d: string): string {
    const [, m, day] = d.split("-");
    const months = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];
    return `${months[parseInt(m) - 1]} ${parseInt(day)}`;
  }

  let todayX = $derived((): number => {
    if (!data || data.points.length === 0) return -1;
    const today = new Date().toISOString().slice(0, 10);
    const idx = data.points.findIndex((p) => p.date >= today);
    if (idx < 0) return -1;
    return xPos(idx, data.points.length);
  });
</script>

<div class="w13" style="--project-accent: {projectColor}">
  <span class="widget-label">BURNDOWN</span>
  {#if loading}
    <div class="state-msg">Loading…</div>
  {:else if error}
    <div class="state-msg state-err">{error}</div>
  {:else if data && data.points.length > 0}
    {@const pts = data.points}
    {@const n = pts.length}
    <div class="chart-area" bind:clientWidth={chartW} bind:clientHeight={chartH}>
      <svg width={SVG_W} height={SVG_H} viewBox="0 0 {SVG_W} {SVG_H}">
        <!-- grid -->
        {#each gridLines() as gv}
          <line
            x1={PAD_L} x2={SVG_W - PAD_R}
            y1={yPos(gv)} y2={yPos(gv)}
            stroke="var(--db-grid-line, #ffffff14)"
            stroke-width="1"
          />
          <text x={PAD_L - 4} y={yPos(gv) + 4} text-anchor="end" font-size="9" fill="var(--db-ink-muted)">{gv}h</text>
        {/each}

        <!-- today marker -->
        {#if todayX() >= 0}
          <line
            x1={todayX()} x2={todayX()}
            y1={PAD_T} y2={SVG_H - PAD_B}
            stroke="var(--db-ink-muted)"
            stroke-width="1"
            stroke-dasharray="3 3"
            opacity="0.5"
          />
        {/if}

        <!-- ideal line (dashed) -->
        <polyline
          points={polyline((p) => p.ideal_hours)}
          fill="none"
          stroke="var(--db-ink-muted)"
          stroke-width="1.5"
          stroke-dasharray="5 3"
          opacity="0.5"
        />

        <!-- remaining area fill -->
        <polygon
          points="{polyline((p) => p.remaining_hours)} {xPos(n - 1, n)},{SVG_H - PAD_B} {PAD_L},{SVG_H - PAD_B}"
          fill="color-mix(in srgb, var(--project-accent) 18%, transparent)"
        />

        <!-- remaining line -->
        <polyline
          points={polyline((p) => p.remaining_hours)}
          fill="none"
          stroke="var(--project-accent)"
          stroke-width="2"
          stroke-linejoin="round"
        />

        <!-- Y axis -->
        <line x1={PAD_L} x2={PAD_L} y1={PAD_T} y2={SVG_H - PAD_B} stroke="var(--db-border)" stroke-width="1" />

        <!-- X labels: start + end -->
        <text x={PAD_L} y={SVG_H - PAD_B + 12} text-anchor="middle" font-size="9" fill="var(--db-ink-muted)">{fmtDate(data.start_date)}</text>
        <text x={SVG_W - PAD_R} y={SVG_H - PAD_B + 12} text-anchor="middle" font-size="9" fill={data.has_deadline ? "#f59e0b" : "var(--db-ink-muted)"}>{fmtDate(data.end_date)}</text>
      </svg>
    </div>
    <div class="legend">
      <span class="leg"><span class="leg-line accent"></span>remaining</span>
      <span class="leg"><span class="leg-line dashed"></span>ideal</span>
      {#if data.has_deadline}<span class="leg deadline-tag">deadline</span>{/if}
    </div>
  {:else if data}
    <div class="state-msg">No estimate data</div>
  {:else}
    <div class="state-msg">No data</div>
  {/if}
</div>

<style>
  .w13 {
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
    gap: 14px;
    justify-content: center;
    flex-shrink: 0;
  }
  .leg {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 10px;
    color: var(--db-ink-muted);
  }
  .leg-line {
    display: inline-block;
    width: 18px;
    height: 2px;
    border-radius: 1px;
  }
  .leg-line.accent { background: var(--project-accent); }
  .leg-line.dashed {
    background: repeating-linear-gradient(
      90deg,
      var(--db-ink-muted) 0 5px,
      transparent 5px 8px
    );
    opacity: 0.6;
  }
  .deadline-tag {
    font-size: 10px;
    color: #f59e0b;
    font-weight: 600;
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
