<script lang="ts">
  import type { DashboardDateRange } from "$lib/api";
  import { getDashboardProductivity } from "$lib/api";
  import type { DashboardProductivityDay, DashboardProductivityProjectEntry } from "$lib/types";

  interface Props {
    dateRange: DashboardDateRange;
  }
  let { dateRange }: Props = $props();

  let data = $state<DashboardProductivityDay[]>([]);
  let loading = $state(true);
  let error = $state(false);

  $effect(() => {
    loading = true;
    error = false;
    getDashboardProductivity(dateRange)
      .then((result) => { data = result; loading = false; })
      .catch(() => { error = true; loading = false; });
  });

  // SVG layout — both width and height adapt to container
  const PAD_L = 36;
  const PAD_R = 8;
  const PAD_T = 10;
  const PAD_B = 28;
  let chartAreaH = $state(200);
  let chartAreaW = $state(500);
  let SVG_W = $derived(Math.max(300, chartAreaW));
  let SVG_H = $derived(Math.max(120, chartAreaH));
  let CHART_W = $derived(SVG_W - PAD_L - PAD_R);
  let CHART_H = $derived(SVG_H - PAD_T - PAD_B);

  // Net (total) hours per day
  let netHours = $derived(data.map((d) => d.tracked_minutes / 60));
  let maxHours = $derived(netHours.length === 0 ? 1 : Math.max(1, ...netHours));
  let avgHours = $derived(
    netHours.length === 0 ? 0 : netHours.reduce((s, h) => s + h, 0) / netHours.length,
  );

  // Unique projects in order of first appearance
  interface ProjectMeta { id: string; name: string; color: string }
  let allProjects: ProjectMeta[] = $derived((() => {
    const map = new Map<string, ProjectMeta>();
    for (const day of data) {
      for (const pe of day.project_entries) {
        if (!map.has(pe.project_id)) {
          map.set(pe.project_id, {
            id: pe.project_id,
            name: pe.project_name,
            color: pe.project_color,
          });
        }
      }
    }
    return [...map.values()];
  })());

  // Stacked cumulative minutes per day per project (in hours for yPos)
  // stacks[dayIndex][projectIndex] = cumulative hours up to and including projectIndex
  let stacks = $derived<number[][]>(
    data.map((day) => {
      let cum = 0;
      return allProjects.map((p) => {
        const entry = day.project_entries.find((e) => e.project_id === p.id);
        cum += (entry?.minutes ?? 0) / 60;
        return cum;
      });
    }),
  );

  function xPos(i: number): number {
    if (data.length <= 1) return PAD_L + CHART_W / 2;
    return PAD_L + (i / (data.length - 1)) * CHART_W;
  }

  function yPos(h: number): number {
    return PAD_T + CHART_H - (h / maxHours) * CHART_H;
  }

  // Build a stacked area path for one project layer
  function stackedAreaPath(projectIndex: number): string {
    if (data.length === 0) return "";
    // Upper edge: left to right
    const pts = data.map((_, i) => `${xPos(i)},${yPos(stacks[i]?.[projectIndex] ?? 0)}`);
    // Lower edge: right to left (previous stack level, or baseline for first)
    const baseY = PAD_T + CHART_H;
    const lowerPts =
      projectIndex > 0
        ? data
            .map((_, i) => data.length - 1 - i)
            .map((i) => `${xPos(i)},${yPos(stacks[i]?.[projectIndex - 1] ?? 0)}`)
        : [`${xPos(data.length - 1)},${baseY}`, `${xPos(0)},${baseY}`];
    return `M ${pts.join(" L ")} L ${lowerPts.join(" L ")} Z`;
  }

  // Net line polyline
  let netPolyline = $derived(
    netHours.map((h, i) => `${xPos(i)},${yPos(h)}`).join(" "),
  );

  let avgY = $derived(yPos(avgHours));

  function fmtDateLabel(dateStr: string): string {
    const d = new Date(dateStr + "T00:00:00");
    return d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  }

  let labelIndices = $derived(() => {
    const n = data.length;
    if (n <= 6) return data.map((_, i) => i);
    const step = Math.ceil(n / 6);
    const idxs: number[] = [];
    for (let i = 0; i < n; i += step) idxs.push(i);
    if (idxs[idxs.length - 1] !== n - 1) idxs.push(n - 1);
    return idxs;
  });

  let yTicks = $derived(() => {
    const ticks: number[] = [];
    const step = maxHours <= 2 ? 0.5 : maxHours <= 8 ? 2 : 4;
    for (let v = 0; v <= maxHours; v += step) ticks.push(v);
    return ticks;
  });

  const NET_COLOR = "#ffffff";
  const AREA_OPACITY = 0.72;
</script>

<div class="widget">
  <span class="widget-label">Productivity</span>

  {#if loading}
    <div class="state-msg">Loading…</div>
  {:else if error}
    <div class="state-msg state-err">Failed to load data</div>
  {:else if data.length === 0}
    <div class="state-msg">No tracked time in this period</div>
  {:else}
    <div class="chart-area" bind:clientHeight={chartAreaH} bind:clientWidth={chartAreaW}>
      <svg
        width={SVG_W}
        height={SVG_H}
        viewBox="0 0 {SVG_W} {SVG_H}"
        aria-label="Daily productivity chart"
        role="img"
      >
        <defs>
          {#each allProjects as p, i (p.id)}
            <linearGradient id="prod-grad-{i}" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stop-color={p.color} stop-opacity="0.9" />
              <stop offset="100%" stop-color={p.color} stop-opacity="0.4" />
            </linearGradient>
          {/each}
          <linearGradient id="prod-net-grad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stop-color={NET_COLOR} stop-opacity="0.15" />
            <stop offset="100%" stop-color={NET_COLOR} stop-opacity="0.02" />
          </linearGradient>
        </defs>

        <!-- Y-axis grid lines -->
        {#each yTicks() as tick (tick)}
          {@const ty = yPos(tick)}
          <line
            x1={PAD_L} y1={ty} x2={SVG_W - PAD_R} y2={ty}
            stroke="var(--db-grid-line, rgba(255,255,255,0.06))"
            stroke-width="1"
          />
          <text x={PAD_L - 4} y={ty + 4} font-size="12" text-anchor="end"
            fill="var(--db-ink-muted, #8b949e)">{tick % 1 === 0 ? `${tick}h` : `${tick}h`}</text>
        {/each}

        <!-- Stacked project areas (bottom to top) -->
        {#each allProjects as p, i (p.id)}
          <path
            d={stackedAreaPath(i)}
            fill="url(#prod-grad-{i})"
            opacity={AREA_OPACITY}
          />
        {/each}

        <!-- Net total area (unattributed time, if any, above stacks) -->

        <!-- Dashed average line -->
        {#if data.length > 1}
          <line
            x1={PAD_L} y1={avgY} x2={SVG_W - PAD_R} y2={avgY}
            stroke={NET_COLOR} stroke-width="1"
            stroke-dasharray="4 3" stroke-opacity="0.4"
          />
          <text x={SVG_W - PAD_R - 2} y={avgY - 5} font-size="12"
            text-anchor="end" fill={NET_COLOR} opacity="0.55">avg</text>
        {/if}

        <!-- Net total line (white, on top) -->
        {#if netHours.length > 1}
          <polyline
            points={netPolyline}
            fill="none"
            stroke={NET_COLOR}
            stroke-width="2"
            stroke-linejoin="round"
            stroke-linecap="round"
            opacity="0.85"
          />
        {:else if netHours.length === 1}
          <circle cx={xPos(0)} cy={yPos(netHours[0])} r="4" fill={NET_COLOR} />
        {/if}

        <!-- Data point dots on net line -->
        {#each netHours as h, i (i)}
          <circle cx={xPos(i)} cy={yPos(h)} r="2.5" fill={NET_COLOR} opacity="0.85" />
        {/each}

        <!-- X-axis labels -->
        {#each labelIndices() as i (i)}
          <text x={xPos(i)} y={SVG_H - 4} font-size="12"
            text-anchor="middle" fill="var(--db-ink-muted, #8b949e)">{fmtDateLabel(data[i].date)}</text>
        {/each}
      </svg>
    </div>

    <!-- Legend: projects + NET -->
    {#if allProjects.length > 0}
      <div class="legend">
        {#each allProjects as p (p.id)}
          <span class="legend-item">
            <span class="legend-dot" style="background:{p.color}"></span>
            {p.name}
          </span>
        {/each}
        <span class="legend-item legend-net">
          <span class="legend-line"></span>
          NET
        </span>
      </div>
    {/if}
  {/if}
</div>

<style>
  .widget {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .widget-label {
    font-size: 13px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--db-ink-muted, #8b949e);
    flex-shrink: 0;
  }

  .chart-area {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .chart-area svg {
    display: block;
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
    justify-content: center;
    flex-shrink: 0;
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--db-ink-muted, #8b949e);
  }

  .legend-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .legend-net {
    color: var(--db-ink, #e6edf3);
    font-weight: 600;
  }

  .legend-line {
    width: 14px;
    height: 2px;
    background: #ffffff;
    border-radius: 1px;
    opacity: 0.85;
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
