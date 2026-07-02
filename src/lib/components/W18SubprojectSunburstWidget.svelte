<script lang="ts">
  import { getProjectSubprojectSunburst } from "$lib/api";
  import type { ProjectSunburstSlice } from "$lib/types";
  import WidgetHeader from "./WidgetHeader.svelte";

  interface Props {
    projectId: string;
    projectColor: string;
    config?: Record<string, unknown>;
    editMode?: boolean;
  }
  let { projectId, projectColor, config: _config = {}, editMode = false }: Props = $props();

  let data = $state<ProjectSunburstSlice[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    if (!projectId) return;
    loading = true;
    error = null;
    getProjectSubprojectSunburst(projectId)
      .then((d) => { data = d; })
      .catch((e) => { error = e instanceof Error ? e.message : String(e); })
      .finally(() => { loading = false; });
  });

  const CX = 70;
  const CY = 70;
  const R_INNER_START = 22;
  const R_INNER_END   = 46;
  const R_OUTER_START = 50;
  const R_OUTER_END   = 66;

  function arcPath(
    outerR: number, innerR: number, startDeg: number, endDeg: number,
  ): string {
    const clamped = Math.min(endDeg, startDeg + 359.9999);
    const toRad = (d: number) => (d * Math.PI) / 180;
    const s = toRad(startDeg - 90);
    const e = toRad(clamped - 90);
    const large = clamped - startDeg > 180 ? 1 : 0;
    const cos = Math.cos; const sin = Math.sin;
    return [
      `M ${CX + outerR * cos(s)} ${CY + outerR * sin(s)}`,
      `A ${outerR} ${outerR} 0 ${large} 1 ${CX + outerR * cos(e)} ${CY + outerR * sin(e)}`,
      `L ${CX + innerR * cos(e)} ${CY + innerR * sin(e)}`,
      `A ${innerR} ${innerR} 0 ${large} 0 ${CX + innerR * cos(s)} ${CY + innerR * sin(s)}`,
      "Z",
    ].join(" ");
  }

  interface ArcEntry {
    path: string;
    color: string;
    name: string;
    depth: number;
    tracked_minutes: number;
  }

  let arcs = $derived((): ArcEntry[] => {
    const depth1 = data.filter((s) => s.depth === 1);
    const depth2 = data.filter((s) => s.depth === 2);
    const total1 = depth1.reduce((s, d) => s + d.tracked_minutes, 0);
    if (total1 === 0) return [];

    const result: ArcEntry[] = [];
    let angle = 0;

    const d1WithAngles = depth1.map((s) => {
      const sweep = (s.tracked_minutes / total1) * 360;
      const entry = { ...s, startAngle: angle, endAngle: angle + sweep };
      angle += sweep;
      return entry;
    });

    // Inner ring
    for (const s of d1WithAngles) {
      result.push({
        path: arcPath(R_INNER_END, R_INNER_START, s.startAngle, s.endAngle),
        color: s.color,
        name: s.name,
        depth: 1,
        tracked_minutes: s.tracked_minutes,
      });
    }

    // Outer ring: depth-2 children within parent's angular span
    for (const parent of d1WithAngles) {
      const children = depth2.filter((c) => c.parent_id === parent.project_id);
      const childTotal = children.reduce((s, c) => s + c.tracked_minutes, 0);
      if (childTotal === 0) continue;
      const parentSpan = parent.endAngle - parent.startAngle;
      let childAngle = parent.startAngle;
      for (const c of children) {
        const sweep = (c.tracked_minutes / childTotal) * parentSpan;
        result.push({
          path: arcPath(R_OUTER_END, R_OUTER_START, childAngle, childAngle + sweep),
          color: c.color,
          name: c.name,
          depth: 2,
          tracked_minutes: c.tracked_minutes,
        });
        childAngle += sweep;
      }
    }

    return result;
  });

  let total1Mins = $derived(
    data.filter((s) => s.depth === 1).reduce((s, d) => s + d.tracked_minutes, 0),
  );

  function fmtMins(m: number): string {
    if (m < 60) return `${m}m`;
    const h = Math.floor(m / 60);
    const rem = m % 60;
    return rem === 0 ? `${h}h` : `${h}h${rem}m`;
  }

  let clientW = $state(0);
</script>

<div class="w18" style="--project-accent: {projectColor}" bind:clientWidth={clientW}>
  <WidgetHeader widgetType="p_subproject_sunburst" />
  {#if loading}
    <div class="state-msg">Loading…</div>
  {:else if error}
    <div class="state-msg state-err">{error}</div>
  {:else if arcs().length > 0}
    <div class="content">
      <svg width="140" height="140" viewBox="0 0 140 140" class:muted={editMode}>
        <!-- Background rings (empty tracks) -->
        <circle cx={CX} cy={CY} r={(R_INNER_START + R_INNER_END) / 2}
          fill="none" stroke="var(--db-border)" stroke-width={R_INNER_END - R_INNER_START} opacity="0.3" />
        <circle cx={CX} cy={CY} r={(R_OUTER_START + R_OUTER_END) / 2}
          fill="none" stroke="var(--db-border)" stroke-width={R_OUTER_END - R_OUTER_START} opacity="0.3" />

        <!-- Slices -->
        {#each arcs() as a}
          <path d={a.path} fill={a.color} opacity={a.depth === 1 ? 0.9 : 0.75} />
        {/each}

        <!-- Center label -->
        {#if total1Mins > 0}
          <text x={CX} y={CY - 5} text-anchor="middle" font-size="12" font-weight="700" fill="var(--db-ink)">{fmtMins(total1Mins)}</text>
          <text x={CX} y={CY + 8} text-anchor="middle" font-size="8" fill="var(--db-ink-muted)">TRACKED</text>
        {/if}
      </svg>

      <!-- Legend: depth-1 only -->
      <div class="legend">
        {#each data.filter((s) => s.depth === 1) as s}
          <div class="leg-row">
            <span class="leg-dot" style="background: {s.color}"></span>
            <span class="leg-name">{s.name}</span>
            <span class="leg-val">{fmtMins(s.tracked_minutes)}</span>
          </div>
        {/each}
      </div>
    </div>
  {:else}
    <div class="state-msg">No tracked time in subprojects</div>
  {/if}
</div>

<style>
  .w18 {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .content {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    overflow: hidden;
  }
  svg { flex-shrink: 0; }
  svg.muted { opacity: 0.7; }
  .legend {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
    overflow: hidden;
  }
  .leg-row {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .leg-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .leg-name {
    flex: 1;
    font-size: 11px;
    color: var(--db-ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .leg-val {
    font-size: 11px;
    font-weight: 600;
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
    text-align: center;
  }
  .state-err { color: #ef4444; }
</style>
