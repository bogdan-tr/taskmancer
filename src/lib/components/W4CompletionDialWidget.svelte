<script lang="ts">
  import { getProjectCompletionDial } from "$lib/api";
  import type { ProjectCompletionDial } from "$lib/types";

  interface Props {
    projectId: string;
    projectColor: string;
  }
  let { projectId, projectColor }: Props = $props();

  let data = $state<ProjectCompletionDial | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let clientW = $state(0);

  $effect(() => {
    if (!projectId) return;
    loading = true;
    error = null;
    getProjectCompletionDial(projectId)
      .then((d) => { data = d; })
      .catch((e) => { error = e instanceof Error ? e.message : String(e); })
      .finally(() => { loading = false; });
  });

  // SVG layout constants.
  // viewBox starts at y=10 so we clip the bottom half of the full circle and
  // show only the top semicircle. CY=100 means the arc apex is at y=20 and
  // the diameter endpoints are at y=100 — all within the 110px visible height.
  const VB_W = 200;
  const VB_H = 110;
  const VB_OFFSET_Y = 10;
  const CX = 100;
  const CY = 100;
  const R_OUTER = 80;
  const R_INNER = 60;
  const SW_OUTER = 14;
  const SW_INNER = 10;

  // Converts polar coords to cartesian SVG coords.
  function polar(cx: number, cy: number, r: number, deg: number): [number, number] {
    const rad = (deg * Math.PI) / 180;
    return [cx + r * Math.cos(rad), cy + r * Math.sin(rad)];
  }

  // Builds an SVG arc path from startDeg to endDeg going CLOCKWISE (sweep-flag=1),
  // which in SVG y-down coordinates means the arc curves through the TOP of the
  // circle (270° = y=CY-R) rather than the bottom.
  //
  // The 179.999° clamp prevents the degenerate case where start==end when the
  // arc spans exactly 180°, which would make SVG collapse it to a line.
  function arcPath(r: number, startDeg: number, endDeg: number): string {
    const span = Math.min(endDeg - startDeg, 179.999);
    const actualEnd = startDeg + span;
    const [x1, y1] = polar(CX, CY, r, startDeg);
    const [x2, y2] = polar(CX, CY, r, actualEnd);
    const large = span > 180 ? 1 : 0;
    // sweep-flag=1: clockwise in SVG (visually goes OVER the top of the circle).
    return `M ${x1.toFixed(3)} ${y1.toFixed(3)} A ${r} ${r} 0 ${large} 1 ${x2.toFixed(3)} ${y2.toFixed(3)}`;
  }

  // Background track: full semicircle from 180° (left) to 360° (right).
  const TRACK_OUTER = arcPath(R_OUTER, 180, 360);
  const TRACK_INNER = arcPath(R_INNER, 180, 360);

  let outerArcPath = $derived(
    data && data.completion_pct > 0
      ? arcPath(R_OUTER, 180, 180 + (Math.min(data.completion_pct, 100) / 100) * 180)
      : null,
  );

  let innerArcPath = $derived(
    data && data.weighted_pct > 0
      ? arcPath(R_INNER, 180, 180 + (Math.min(data.weighted_pct, 100) / 100) * 180)
      : null,
  );

  let isComplete = $derived(data ? data.completion_pct >= 99.9 : false);

  let SVG_W = $derived(Math.max(120, clientW));
  let SVG_H = $derived((SVG_W / VB_W) * VB_H);
</script>

<div
  class="dial-wrap"
  bind:clientWidth={clientW}
  style="--project-accent: {projectColor}"
>
  {#if loading}
    <div class="skeleton">
      <div class="sk-arc"></div>
      <div class="sk-pct"></div>
    </div>
  {:else if error}
    <p class="err">{error}</p>
  {:else if data}
    <svg
      width={SVG_W}
      height={SVG_H}
      viewBox="0 {VB_OFFSET_Y} {VB_W} {VB_H}"
      xmlns="http://www.w3.org/2000/svg"
      aria-label="Completion dial: {data.completion_pct.toFixed(0)}%"
    >
      <!-- Background track arcs -->
      <path
        d={TRACK_OUTER}
        fill="none"
        stroke="var(--db-border)"
        stroke-width={SW_OUTER}
        stroke-linecap="round"
      />
      <path
        d={TRACK_INNER}
        fill="none"
        stroke="var(--db-border)"
        stroke-width={SW_INNER}
        stroke-linecap="round"
        opacity="0.5"
      />

      <!-- Weighted-by-time arc (inner ring, 45% opacity) -->
      {#if innerArcPath}
        <path
          d={innerArcPath}
          fill="none"
          stroke={projectColor}
          stroke-width={SW_INNER}
          stroke-linecap="round"
          opacity="0.45"
        />
      {/if}

      <!-- Task-count completion arc (outer ring) -->
      {#if outerArcPath}
        <path
          d={outerArcPath}
          fill="none"
          stroke={projectColor}
          stroke-width={SW_OUTER}
          stroke-linecap="round"
          filter={isComplete ? `drop-shadow(0 0 6px ${projectColor})` : "none"}
        />
      {/if}

      <!-- Center text — positioned at the base of the semicircle -->
      <text
        x={CX}
        y={CY - 5}
        text-anchor="middle"
        dominant-baseline="auto"
        font-size="28"
        font-weight="800"
        fill="var(--db-ink)"
        font-family="inherit"
        letter-spacing="-1"
      >
        {data.completion_pct.toFixed(0)}%
      </text>
      <text
        x={CX}
        y={CY + 14}
        text-anchor="middle"
        dominant-baseline="auto"
        font-size="9"
        font-weight="600"
        fill="var(--db-ink-muted)"
        font-family="inherit"
        letter-spacing="0.5"
      >
        WTD: {data.weighted_pct.toFixed(0)}%
      </text>
    </svg>
  {:else}
    <p class="empty">No data.</p>
  {/if}
</div>

<style>
  .dial-wrap {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  svg {
    overflow: visible;
    display: block;
    flex-shrink: 0;
  }

  .skeleton {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 16px;
  }

  .sk-arc {
    width: 100px;
    height: 52px;
    border-radius: 100px 100px 0 0;
    border: 12px solid var(--db-border);
    border-bottom: none;
    animation: pulse 1.4s ease-in-out infinite;
  }

  .sk-pct {
    width: 52px;
    height: 20px;
    border-radius: 4px;
    background: var(--db-border);
    animation: pulse 1.4s ease-in-out infinite 0.2s;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 0.8; }
  }

  .err,
  .empty {
    margin: 0;
    font-size: 12px;
    color: var(--db-ink-muted);
    text-align: center;
    padding: 16px;
  }

  .err {
    color: #ef4444;
  }
</style>
