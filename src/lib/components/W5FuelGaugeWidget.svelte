<script lang="ts">
  import { getProjectFuelGauge } from "$lib/api";
  import type { ProjectFuelGauge } from "$lib/types";
  import WidgetHeader from "./WidgetHeader.svelte";

  interface Props {
    projectId: string;
    projectColor: string;
  }
  let { projectId, projectColor }: Props = $props();

  let data = $state<ProjectFuelGauge | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    if (!projectId) return;
    loading = true;
    error = null;
    getProjectFuelGauge(projectId)
      .then((d) => { data = d; })
      .catch((e) => { error = e instanceof Error ? e.message : String(e); })
      .finally(() => { loading = false; });
  });

  function fmtMins(mins: number): string {
    if (mins <= 0) return "0m";
    const h = Math.floor(mins / 60);
    const m = mins % 60;
    if (h === 0) return `${m}m`;
    if (m === 0) return `${h}h`;
    return `${h}h ${m}m`;
  }

  /** Fraction of estimated work still remaining: 1 = full tank, 0 = empty. */
  let frac = $derived.by(() => {
    if (!data || data.estimated_total_mins <= 0) return 0;
    return Math.max(0, Math.min(1, data.estimated_remaining_mins / data.estimated_total_mins));
  });
  let isLow = $derived(frac <= 0.1);

  /** More remaining work = hotter: a full tank of TO-DO is red, and the
   * gauge cools to green as the work burns down (finishing is good). */
  let fuelColor = $derived.by(() => {
    if (frac > 0.66) return "#ef4444";
    if (frac > 0.33) return "#f59e0b";
    return "#22c55e";
  });

  // ── Semicircular gauge geometry ───────────────────────────────────────────
  // Arc spans 180° from (20,105) to (180,105) over the top, radius 80.
  const ARC_R = 80;
  const ARC_CX = 100;
  const ARC_CY = 105;
  const ARC_LEN = Math.PI * ARC_R;

  /** Needle angle: 180° (left, EMPTY) … 0° (right, FULL). */
  let needleAngle = $derived(180 - frac * 180);
  let needleRad = $derived((needleAngle * Math.PI) / 180);
  let needleX = $derived(ARC_CX + Math.cos(needleRad) * (ARC_R - 14));
  let needleY = $derived(ARC_CY - Math.sin(needleRad) * (ARC_R - 14));

  /** Tick marks every 25%. */
  const ticks = [0, 0.25, 0.5, 0.75, 1].map((f) => {
    const a = ((180 - f * 180) * Math.PI) / 180;
    return {
      x1: ARC_CX + Math.cos(a) * (ARC_R + 6),
      y1: ARC_CY - Math.sin(a) * (ARC_R + 6),
      x2: ARC_CX + Math.cos(a) * (ARC_R + 12),
      y2: ARC_CY - Math.sin(a) * (ARC_R + 12),
    };
  });
</script>

<div class="w5" style="--project-accent: {projectColor}; --fuel-color: {fuelColor}">
  <WidgetHeader widgetType="p_fuel_gauge" />
  {#if loading}
    <div class="state-msg">Loading…</div>
  {:else if error}
    <div class="state-msg state-err">{error}</div>
  {:else if data && data.estimated_total_mins > 0}
    <div class="gauge-area">
      <svg viewBox="0 0 200 128" class="gauge-svg" role="img"
        aria-label="Work remaining: {fmtMins(data.estimated_remaining_mins)} of {fmtMins(data.estimated_total_mins)} estimated">
        <!-- Track -->
        <path
          d="M 20 105 A {ARC_R} {ARC_R} 0 0 1 180 105"
          fill="none"
          stroke="var(--db-grid-line, rgba(255,255,255,0.07))"
          stroke-width="13"
          stroke-linecap="round"
        />
        <!-- Fuel arc: fills from the left (empty side) up to the needle -->
        <path
          d="M 20 105 A {ARC_R} {ARC_R} 0 0 1 180 105"
          fill="none"
          stroke="var(--fuel-color)"
          stroke-width="13"
          stroke-linecap="round"
          stroke-dasharray={ARC_LEN}
          stroke-dashoffset={ARC_LEN * (1 - frac)}
          class="fuel-arc"
          class:low={isLow}
        />
        <!-- Ticks -->
        {#each ticks as t}
          <line x1={t.x1} y1={t.y1} x2={t.x2} y2={t.y2}
            stroke="var(--db-ink-muted, #8b949e)" stroke-opacity="0.45" stroke-width="1.5" />
        {/each}
        <!-- End labels: left = done side, right = full workload -->
        <text x="14" y="122" class="ef-label">✓</text>
        <text x="181" y="122" text-anchor="middle" class="ef-label">MAX</text>

        <!-- Needle -->
        <line x1={ARC_CX} y1={ARC_CY} x2={needleX} y2={needleY}
          stroke="var(--db-ink, #e6edf3)" stroke-width="2.5" stroke-linecap="round" class="needle" />
        <circle cx={ARC_CX} cy={ARC_CY} r="5" fill="var(--db-ink, #e6edf3)" />
        <circle cx={ARC_CX} cy={ARC_CY} r="2.2" fill="var(--db-card, #161b22)" />
      </svg>
    </div>
    <div class="readout">
      <span class="big-num" class:low={isLow}>{fmtMins(data.estimated_remaining_mins)}</span>
      <span class="sub">left of {fmtMins(data.estimated_total_mins)} estimated</span>
      {#if isLow}
        <span class="empty-flag">{data.estimated_remaining_mins <= 0 ? "ALL DONE" : "ALMOST THERE"}</span>
      {/if}
    </div>
  {:else}
    <div class="state-msg">No estimated tasks yet — add time estimates to fuel the gauge</div>
  {/if}
</div>

<style>
  .w5 {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .gauge-area {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: flex-end;
    justify-content: center;
  }

  .gauge-svg {
    width: 100%;
    height: 100%;
    max-height: 150px;
  }

  .fuel-arc {
    transition: stroke-dashoffset 600ms cubic-bezier(0.16, 1, 0.3, 1), stroke 300ms;
  }

  .fuel-arc.low {
    filter: drop-shadow(0 0 5px var(--fuel-color));
  }

  .needle {
    transition: x2 600ms, y2 600ms;
  }

  .ef-label {
    font-size: 10px;
    font-weight: 700;
    fill: var(--db-ink-muted, #8b949e);
  }

  .readout {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
    flex-shrink: 0;
    padding-bottom: 2px;
  }

  .big-num {
    font-size: 22px;
    font-weight: 800;
    color: var(--db-ink, #e6edf3);
    font-variant-numeric: tabular-nums;
    line-height: 1.1;
  }

  .big-num.low {
    color: var(--fuel-color);
  }

  .sub {
    font-size: 10.5px;
    color: var(--db-ink-muted, #8b949e);
  }

  .empty-flag {
    margin-top: 3px;
    font-size: 8.5px;
    font-weight: 800;
    letter-spacing: 0.14em;
    color: var(--fuel-color);
    border: 1px solid color-mix(in srgb, var(--fuel-color) 45%, transparent);
    background: color-mix(in srgb, var(--fuel-color) 12%, transparent);
    padding: 2px 7px;
    border-radius: 999px;
  }

  .state-msg {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
    font-size: 12px;
    padding: 0 10px;
    color: var(--db-ink-muted, #8b949e);
  }
  .state-err { color: #ef4444; }
</style>
