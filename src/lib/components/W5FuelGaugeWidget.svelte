<script lang="ts">
  import { getProjectFuelGauge } from "$lib/api";
  import type { ProjectFuelGauge } from "$lib/types";

  interface Props {
    projectId: string;
    projectColor: string;
  }
  let { projectId, projectColor }: Props = $props();

  let data = $state<ProjectFuelGauge | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let clientW = $state(0);

  $effect(() => {
    if (!projectId) return;
    loading = true;
    error = null;
    getProjectFuelGauge(projectId)
      .then((d) => { data = d; })
      .catch((e) => { error = e instanceof Error ? e.message : String(e); })
      .finally(() => { loading = false; });
  });

  // SVG donut ring constants.
  const VB = 200;
  const CX = 100;
  const CY = 100;
  const R_OUTER = 80;
  const R_INNER = 56; // ring thickness = 24px
  const STROKE_W = R_OUTER - R_INNER;
  const R_MID = (R_OUTER + R_INNER) / 2; // 68 — stroke centre

  // fill_pct = budget consumed = (total - remaining) / max(total, 1)
  // Values >1.0 mean over-budget — we let it exceed 1 for the "OVER" badge.
  let rawFillPct = $derived(
    data
      ? (data.estimated_total_mins - data.estimated_remaining_mins) /
          Math.max(data.estimated_total_mins, 1)
      : 0,
  );

  let hasEstimate = $derived(data ? data.estimated_total_mins > 0 : false);
  let isOver = $derived(rawFillPct > 1.0);
  let fillPct = $derived(Math.min(rawFillPct, 1.0)); // clamp to ring max

  // Color tiers: ≤50% → project accent, 50-80% → amber, >80% → red.
  let ringColor = $derived(
    fillPct <= 0.5 ? projectColor : fillPct <= 0.8 ? "#f59e0b" : "#ef4444",
  );

  // Full circle via SVG stroke-dasharray / stroke-dashoffset on a circle element.
  // circumference = 2π*R_MID; dashoffset shifts the filled portion.
  const CIRC = 2 * Math.PI * R_MID;
  let dashFilled = $derived(fillPct * CIRC);
  let dashEmpty = $derived(CIRC - dashFilled);

  // Responsive: square SVG scales to container width, max 200px.
  let SVG_SIZE = $derived(Math.min(Math.max(100, clientW), VB));
</script>

<div
  class="ring-wrap"
  bind:clientWidth={clientW}
  style="--project-accent: {projectColor}"
>
  {#if loading}
    <div class="skeleton">
      <div class="sk-ring"></div>
    </div>
  {:else if error}
    <p class="err">{error}</p>
  {:else if data && !hasEstimate}
    <div class="no-estimate">
      <span class="no-est-icon">—</span>
      <p class="no-est-label">No estimate</p>
    </div>
  {:else if data}
    <svg
      width={SVG_SIZE}
      height={SVG_SIZE}
      viewBox="0 0 {VB} {VB}"
      xmlns="http://www.w3.org/2000/svg"
      aria-label="Effort consumed: {Math.round(rawFillPct * 100)}%"
    >
      <!-- Background track ring -->
      <circle
        cx={CX}
        cy={CY}
        r={R_MID}
        fill="none"
        stroke="var(--db-border)"
        stroke-width={STROKE_W}
      />

      <!-- Consumed arc — starts at the top (−90° offset via transform) -->
      {#if fillPct > 0}
        <circle
          cx={CX}
          cy={CY}
          r={R_MID}
          fill="none"
          stroke={ringColor}
          stroke-width={STROKE_W}
          stroke-linecap="butt"
          stroke-dasharray="{dashFilled} {dashEmpty}"
          transform="rotate(-90 {CX} {CY})"
          style="transition: stroke-dasharray 0.4s ease, stroke 0.3s ease;"
        />
      {/if}

      <!-- Center: percentage -->
      <text
        x={CX}
        y={CY - 10}
        text-anchor="middle"
        dominant-baseline="auto"
        font-size="30"
        font-weight="800"
        fill={isOver ? "#ef4444" : "var(--db-ink)"}
        font-family="inherit"
        letter-spacing="-1"
      >
        {Math.round(rawFillPct * 100)}%
      </text>

      <!-- Center: label -->
      <text
        x={CX}
        y={CY + 10}
        text-anchor="middle"
        dominant-baseline="auto"
        font-size="10"
        font-weight="600"
        fill="var(--db-ink-muted)"
        font-family="inherit"
        letter-spacing="1"
      >
        CONSUMED
      </text>

      <!-- OVER badge shown when budget is exceeded -->
      {#if isOver}
        <rect x="70" y="125" width="60" height="20" rx="10" fill="#ef4444" />
        <text
          x={CX}
          y="138"
          text-anchor="middle"
          dominant-baseline="middle"
          font-size="10"
          font-weight="700"
          fill="white"
          font-family="inherit"
          letter-spacing="1"
        >
          OVER
        </text>
      {/if}
    </svg>
  {:else}
    <p class="empty">No data.</p>
  {/if}
</div>

<style>
  .ring-wrap {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  svg {
    display: block;
    flex-shrink: 0;
    overflow: visible;
  }

  .skeleton {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: 16px;
  }

  .sk-ring {
    width: 100px;
    height: 100px;
    border-radius: 50%;
    border: 20px solid var(--db-border);
    animation: pulse 1.4s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 0.8; }
  }

  .no-estimate {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 16px;
  }

  .no-est-icon {
    font-size: 28px;
    font-weight: 800;
    color: var(--db-ink-muted);
    line-height: 1;
  }

  .no-est-label {
    margin: 0;
    font-size: 11px;
    font-weight: 600;
    color: var(--db-ink-muted);
    letter-spacing: 0.5px;
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
