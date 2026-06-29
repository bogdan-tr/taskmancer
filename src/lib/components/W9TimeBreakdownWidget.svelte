<script lang="ts">
  import { getProjectTimeBreakdown } from "$lib/api";
  import type { ProjectTimeBreakdown, ProjectTimeBreakdownSlice } from "$lib/types";

  interface Props {
    projectId: string;
    projectColor: string;
  }
  let { projectId, projectColor }: Props = $props();

  let data = $state<ProjectTimeBreakdown | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    if (!projectId) return;
    loading = true;
    error = null;
    getProjectTimeBreakdown(projectId)
      .then((d) => { data = d; })
      .catch((e) => { error = e instanceof Error ? e.message : String(e); })
      .finally(() => { loading = false; });
  });

  const CX = 60;
  const CY = 60;
  const OUTER_R = 52;
  const INNER_R = 32;

  interface ArcSlice {
    path: string;
    color: string;
    name: string;
    pct: number;
  }

  function arcPath(cx: number, cy: number, outerR: number, innerR: number, startDeg: number, endDeg: number): string {
    const clampedEnd = Math.min(endDeg, startDeg + 359.9999);
    const toRad = (d: number) => (d * Math.PI) / 180;
    const cos = Math.cos;
    const sin = Math.sin;
    const s = toRad(startDeg - 90);
    const e = toRad(clampedEnd - 90);
    const large = clampedEnd - startDeg > 180 ? 1 : 0;
    return [
      `M ${cx + outerR * cos(s)} ${cy + outerR * sin(s)}`,
      `A ${outerR} ${outerR} 0 ${large} 1 ${cx + outerR * cos(e)} ${cy + outerR * sin(e)}`,
      `L ${cx + innerR * cos(e)} ${cy + innerR * sin(e)}`,
      `A ${innerR} ${innerR} 0 ${large} 0 ${cx + innerR * cos(s)} ${cy + innerR * sin(s)}`,
      "Z",
    ].join(" ");
  }

  let slices = $derived((): ArcSlice[] => {
    if (!data || data.total_tracked_minutes === 0) return [];
    const d = data;
    let angle = 0;
    return d.slices.map((s: ProjectTimeBreakdownSlice) => {
      const pct = s.tracked_minutes / d.total_tracked_minutes;
      const sweep = pct * 360;
      const slice: ArcSlice = {
        path: arcPath(CX, CY, OUTER_R, INNER_R, angle, angle + sweep),
        color: s.color,
        name: s.name,
        pct,
      };
      angle += sweep;
      return slice;
    });
  });

  function fmtMins(mins: number): string {
    if (mins < 60) return `${mins}m`;
    const h = Math.floor(mins / 60);
    const m = mins % 60;
    return m === 0 ? `${h}h` : `${h}h ${m}m`;
  }

  let clientW = $state(0);
</script>

<div class="w9" style="--project-accent: {projectColor}" bind:clientWidth={clientW}>
  <span class="widget-label">TIME BREAKDOWN</span>
  {#if loading}
    <div class="state-msg">Loading…</div>
  {:else if error}
    <div class="state-msg state-err">{error}</div>
  {:else if data && data.total_tracked_minutes > 0}
    <div class="content">
      <!-- Donut -->
      <div class="donut-wrap">
        <svg width="120" height="120" viewBox="0 0 120 120">
          {#each slices() as s}
            <path d={s.path} fill={s.color} opacity="0.9" />
          {/each}
          <!-- Center total -->
          <text x={CX} y={CY - 6} text-anchor="middle" font-size="11" font-weight="700" fill="var(--db-ink)">{fmtMins(data.total_tracked_minutes)}</text>
          <text x={CX} y={CY + 8} text-anchor="middle" font-size="8" fill="var(--db-ink-muted)">TOTAL</text>
        </svg>
      </div>
      <!-- Legend -->
      <div class="legend">
        {#each data.slices.slice(0, 6) as s}
          <div class="legend-row">
            <span class="legend-dot" style="background:{s.color}"></span>
            <span class="legend-name">{s.name}</span>
            <span class="legend-val">{(s.tracked_minutes / data.total_tracked_minutes * 100).toFixed(0)}%</span>
          </div>
        {/each}
        {#if !data.by_subproject}
          <div class="legend-mode">by tag</div>
        {/if}
      </div>
    </div>
  {:else if data}
    <div class="state-msg">No tracked time</div>
  {:else}
    <div class="state-msg">No data</div>
  {/if}
</div>

<style>
  .w9 {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .widget-label {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--db-ink-muted);
    flex-shrink: 0;
  }
  .content {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    overflow: hidden;
  }
  .donut-wrap {
    flex-shrink: 0;
  }
  .legend {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
    overflow: hidden;
  }
  .legend-row {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .legend-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .legend-name {
    flex: 1;
    font-size: 11px;
    color: var(--db-ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .legend-val {
    font-size: 11px;
    font-weight: 600;
    color: var(--db-ink-muted);
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
  }
  .legend-mode {
    font-size: 9px;
    letter-spacing: 0.06em;
    color: var(--db-ink-muted);
    text-transform: uppercase;
    margin-top: 2px;
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
