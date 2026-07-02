<script lang="ts">
  import { getProjectCompletionDial } from "$lib/api";
  import type { ProjectCompletionDial } from "$lib/types";
  import WidgetHeader from "./WidgetHeader.svelte";

  interface Props {
    projectId: string;
    projectColor: string;
  }
  let { projectId, projectColor }: Props = $props();

  let data = $state<ProjectCompletionDial | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    if (!projectId) return;
    loading = true;
    error = null;
    getProjectCompletionDial(projectId)
      .then((d) => { data = d; })
      .catch((e) => { error = e instanceof Error ? e.message : String(e); })
      .finally(() => { loading = false; });
  });

  // ── Ring geometry (fixed viewBox; the SVG scales to fill the card) ────────
  const VB = 200;
  const CX = 100;
  const CY = 100;
  const OUTER_R = 84;
  const OUTER_W = 12;
  const INNER_R = 66;
  const INNER_W = 5;

  const outerCirc = 2 * Math.PI * OUTER_R;
  const innerCirc = 2 * Math.PI * INNER_R;

  let countPct = $derived(Math.max(0, Math.min(100, data?.completion_pct ?? 0)));
  let timePct = $derived(Math.max(0, Math.min(100, data?.weighted_pct ?? 0)));
  let isComplete = $derived(countPct >= 100);

  let outerOffset = $derived(outerCirc * (1 - countPct / 100));
  let innerOffset = $derived(innerCirc * (1 - timePct / 100));
</script>

<div class="w4" style="--project-accent: {projectColor}">
  <WidgetHeader widgetType="p_completion_dial" />
  {#if loading}
    <div class="state-msg">Loading…</div>
  {:else if error}
    <div class="state-msg state-err">{error}</div>
  {:else if data && data.total_count > 0}
    <div class="ring-area">
      <svg viewBox="0 0 {VB} {VB}" class="ring-svg" class:complete={isComplete} role="img"
        aria-label="Project progress: {countPct.toFixed(0)} percent of tasks done">
        <defs>
          <linearGradient id="w4-grad-{projectId}" x1="0" y1="1" x2="1" y2="0">
            <stop offset="0%" stop-color={projectColor} stop-opacity="0.55" />
            <stop offset="100%" stop-color={projectColor} stop-opacity="1" />
          </linearGradient>
        </defs>

        <!-- Tracks -->
        <circle cx={CX} cy={CY} r={OUTER_R} fill="none"
          stroke="var(--db-grid-line, rgba(255,255,255,0.06))" stroke-width={OUTER_W} />
        <circle cx={CX} cy={CY} r={INNER_R} fill="none"
          stroke="var(--db-grid-line, rgba(255,255,255,0.06))" stroke-width={INNER_W} />

        <!-- Count progress (outer, bold) -->
        <circle
          cx={CX} cy={CY} r={OUTER_R} fill="none"
          stroke="url(#w4-grad-{projectId})"
          stroke-width={OUTER_W}
          stroke-linecap="round"
          stroke-dasharray={outerCirc}
          stroke-dashoffset={outerOffset}
          transform="rotate(-90 {CX} {CY})"
          class="arc arc-outer"
        />
        <!-- Time-weighted progress (inner, thin) -->
        <circle
          cx={CX} cy={CY} r={INNER_R} fill="none"
          stroke={projectColor}
          stroke-opacity="0.55"
          stroke-width={INNER_W}
          stroke-linecap="round"
          stroke-dasharray={innerCirc}
          stroke-dashoffset={innerOffset}
          transform="rotate(-90 {CX} {CY})"
          class="arc"
        />

        <!-- Center -->
        <text x={CX} y={CY - 4} text-anchor="middle" class="pct-num">
          {countPct.toFixed(0)}<tspan class="pct-sign">%</tspan>
        </text>
        <text x={CX} y={CY + 20} text-anchor="middle" class="pct-sub">
          {data.done_count} of {data.total_count} tasks
        </text>
      </svg>
    </div>
    <div class="legend">
      <span class="legend-item">
        <span class="legend-swatch swatch-count"></span>
        BY COUNT · {countPct.toFixed(0)}%
      </span>
      <span class="legend-item">
        <span class="legend-swatch swatch-time"></span>
        BY TIME · {timePct.toFixed(0)}%
      </span>
    </div>
  {:else}
    <div class="state-msg">No tasks yet</div>
  {/if}
</div>

<style>
  .w4 {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .ring-area {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .ring-svg {
    height: 100%;
    max-width: 100%;
    aspect-ratio: 1;
  }

  .arc {
    transition: stroke-dashoffset 600ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  /* Spec: the ring glows at 100%. */
  .ring-svg.complete .arc-outer {
    filter: drop-shadow(0 0 6px var(--project-accent));
  }

  .pct-num {
    font-size: 40px;
    font-weight: 800;
    fill: var(--db-ink, #e6edf3);
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.02em;
  }

  .pct-sign {
    font-size: 22px;
    font-weight: 700;
    fill: var(--db-ink-muted, #8b949e);
  }

  .pct-sub {
    font-size: 12px;
    fill: var(--db-ink-muted, #8b949e);
  }

  .legend {
    display: flex;
    justify-content: center;
    gap: 14px;
    flex-shrink: 0;
    padding-bottom: 2px;
  }

  .legend-item {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--db-ink-muted, #8b949e);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .legend-swatch {
    width: 14px;
    height: 4px;
    border-radius: 999px;
  }

  .swatch-count {
    background: var(--project-accent);
  }

  .swatch-time {
    background: color-mix(in srgb, var(--project-accent) 55%, transparent);
    height: 3px;
  }

  .state-msg {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    color: var(--db-ink-muted, #8b949e);
  }
  .state-err { color: #ef4444; }
</style>
