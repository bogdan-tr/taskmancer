<script lang="ts">
  import { getProjectStatusRadial } from "$lib/api";
  import type { ProjectStatusSlice } from "$lib/types";

  interface Props {
    projectId: string;
    projectColor: string;
  }
  let { projectId, projectColor }: Props = $props();

  let data = $state<ProjectStatusSlice[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    if (!projectId) return;
    loading = true;
    error = null;
    getProjectStatusRadial(projectId)
      .then((d) => { data = d; })
      .catch((e) => { error = e instanceof Error ? e.message : String(e); })
      .finally(() => { loading = false; });
  });

  const CX = 60;
  const CY = 60;
  const OUTER_R = 52;
  const INNER_R = 30;

  function arcPath(startDeg: number, endDeg: number): string {
    const clampedEnd = Math.min(endDeg, startDeg + 359.9999);
    const toRad = (d: number) => (d * Math.PI) / 180;
    const s = toRad(startDeg - 90);
    const e = toRad(clampedEnd - 90);
    const large = clampedEnd - startDeg > 180 ? 1 : 0;
    return [
      `M ${CX + OUTER_R * Math.cos(s)} ${CY + OUTER_R * Math.sin(s)}`,
      `A ${OUTER_R} ${OUTER_R} 0 ${large} 1 ${CX + OUTER_R * Math.cos(e)} ${CY + OUTER_R * Math.sin(e)}`,
      `L ${CX + INNER_R * Math.cos(e)} ${CY + INNER_R * Math.sin(e)}`,
      `A ${INNER_R} ${INNER_R} 0 ${large} 0 ${CX + INNER_R * Math.cos(s)} ${CY + INNER_R * Math.sin(s)}`,
      "Z",
    ].join(" ");
  }

  let total = $derived(data.reduce((s, d) => s + d.count, 0));

  interface ArcEntry { path: string; color: string; label: string; count: number }

  let arcs = $derived((): ArcEntry[] => {
    if (total === 0) return [];
    let angle = 0;
    return data.map((slice) => {
      const sweep = (slice.count / total) * 360;
      const entry: ArcEntry = {
        path: arcPath(angle, angle + sweep),
        color: slice.color,
        label: slice.label,
        count: slice.count,
      };
      angle += sweep;
      return entry;
    });
  });
</script>

<div class="w10" style="--project-accent: {projectColor}">
  <span class="widget-label">STATUS BREAKDOWN</span>
  {#if loading}
    <div class="state-msg">Loading…</div>
  {:else if error}
    <div class="state-msg state-err">{error}</div>
  {:else if data.length > 0}
    <div class="content">
      <div class="donut-wrap">
        <svg width="120" height="120" viewBox="0 0 120 120">
          {#each arcs() as a}
            <path d={a.path} fill={a.color} opacity="0.92" />
          {/each}
          <text x={CX} y={CY - 5} text-anchor="middle" font-size="18" font-weight="800" fill="var(--db-ink)">{total}</text>
          <text x={CX} y={CY + 10} text-anchor="middle" font-size="8" fill="var(--db-ink-muted)">TASKS</text>
        </svg>
      </div>
      <div class="legend">
        {#each data as s}
          <div class="row">
            <span class="dot" style="background:{s.color}"></span>
            <span class="lbl">{s.label}</span>
            <span class="cnt">{s.count}</span>
          </div>
        {/each}
      </div>
    </div>
  {:else}
    <div class="state-msg">No tasks</div>
  {/if}
</div>

<style>
  .w10 {
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
    gap: 10px;
    overflow: hidden;
  }
  .donut-wrap { flex-shrink: 0; }
  .legend {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow: hidden;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .lbl {
    flex: 1;
    font-size: 11px;
    color: var(--db-ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .cnt {
    font-size: 11px;
    font-weight: 700;
    color: var(--db-ink-muted);
    font-variant-numeric: tabular-nums;
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
