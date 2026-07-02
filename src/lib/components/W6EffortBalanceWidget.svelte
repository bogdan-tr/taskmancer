<script lang="ts">
  import { getProjectEffortBalance } from "$lib/api";
  import type { ProjectEffortBalance } from "$lib/types";
  import WidgetHeader from "./WidgetHeader.svelte";

  interface Props {
    projectId: string;
    projectColor: string;
  }
  let { projectId, projectColor }: Props = $props();

  let data = $state<ProjectEffortBalance | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    if (!projectId) return;
    loading = true;
    error = null;
    getProjectEffortBalance(projectId)
      .then((d) => { data = d; })
      .catch((e) => { error = e instanceof Error ? e.message : String(e); })
      .finally(() => { loading = false; });
  });

  function fmtMins(mins: number): string {
    const abs = Math.abs(mins);
    const h = Math.floor(abs / 60);
    const m = abs % 60;
    if (abs === 0) return "0m";
    if (h === 0) return `${m}m`;
    if (m === 0) return `${h}h`;
    return `${h}h ${m}m`;
  }

  let est = $derived(data?.estimated_total_mins ?? 0);
  let tracked = $derived(data?.tracked_total_mins ?? 0);
  /** The scale runs to whichever is larger, so both always fit. */
  let scaleMax = $derived(Math.max(est, tracked, 1));
  let estPct = $derived((est / scaleMax) * 100);
  let trackedWithinPct = $derived((Math.min(tracked, est) / scaleMax) * 100);
  let overflowPct = $derived((Math.max(0, tracked - est) / scaleMax) * 100);
  let isOver = $derived(tracked > est && est > 0);
  let delta = $derived(tracked - est);
</script>

<div class="w6" style="--project-accent: {projectColor}">
  <WidgetHeader widgetType="p_effort_balance" />
  {#if loading}
    <div class="state-msg">Loading…</div>
  {:else if error}
    <div class="state-msg state-err">{error}</div>
  {:else if data && (est > 0 || tracked > 0)}
    <div class="bullet-area">
      <!-- Numbers row -->
      <div class="numbers-row">
        <div class="num-block">
          <span class="num-label">
            <span class="swatch swatch-tracked"></span>TRACKED
          </span>
          <span class="num-value">{fmtMins(tracked)}</span>
        </div>
        <div class="delta-badge" class:over={isOver} class:under={!isOver && est > 0}>
          {#if est === 0}
            no estimate
          {:else if delta > 0}
            +{fmtMins(delta)} over
          {:else if delta < 0}
            {fmtMins(delta)} to go
          {:else}
            spot on
          {/if}
        </div>
        <div class="num-block right">
          <span class="num-label">
            ESTIMATED<span class="swatch swatch-est"></span>
          </span>
          <span class="num-value muted">{fmtMins(est)}</span>
        </div>
      </div>

      <!-- Bullet chart -->
      <div class="bullet" role="img"
        aria-label="Tracked {fmtMins(tracked)} against an estimate of {fmtMins(est)}">
        <!-- Estimate track -->
        <div class="est-track" style="width: {estPct}%"></div>
        <!-- Tracked bar (within estimate) -->
        <div class="tracked-bar" style="width: {trackedWithinPct}%"></div>
        <!-- Overflow past the estimate -->
        {#if overflowPct > 0}
          <div class="over-bar" style="left: {estPct}%; width: {overflowPct}%"></div>
        {/if}
        <!-- Estimate marker line -->
        {#if est > 0}
          <div class="est-marker" style="left: {estPct}%"></div>
        {/if}
      </div>

      <!-- Scale caption -->
      <div class="scale-row">
        <span>0</span>
        <span>{fmtMins(scaleMax)}</span>
      </div>
    </div>
  {:else}
    <div class="state-msg">Nothing estimated or tracked yet</div>
  {/if}
</div>

<style>
  .w6 {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .bullet-area {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 10px;
  }

  .numbers-row {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 8px;
  }

  .num-block {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .num-block.right {
    align-items: flex-end;
    text-align: right;
  }

  .num-label {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.1em;
    color: var(--db-ink-muted, #8b949e);
  }

  .swatch {
    width: 10px;
    height: 4px;
    border-radius: 999px;
  }

  .swatch-tracked {
    background: var(--project-accent);
  }

  .swatch-est {
    background: color-mix(in srgb, var(--db-ink-muted, #8b949e) 45%, transparent);
  }

  .num-value {
    font-size: 20px;
    font-weight: 800;
    color: var(--db-ink, #e6edf3);
    font-variant-numeric: tabular-nums;
    line-height: 1.1;
    white-space: nowrap;
  }

  .num-value.muted {
    color: var(--db-ink-muted, #8b949e);
  }

  .delta-badge {
    align-self: center;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.03em;
    padding: 3px 9px;
    border-radius: 999px;
    border: 1px solid var(--db-border, rgba(255, 255, 255, 0.1));
    color: var(--db-ink-muted, #8b949e);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .delta-badge.over {
    color: #ef4444;
    border-color: rgba(239, 68, 68, 0.4);
    background: rgba(239, 68, 68, 0.1);
  }

  .delta-badge.under {
    color: #22c55e;
    border-color: rgba(34, 197, 94, 0.35);
    background: rgba(34, 197, 94, 0.08);
  }

  /* ── The bullet itself ── */
  .bullet {
    position: relative;
    height: 26px;
    border-radius: 7px;
    background: var(--db-grid-line, rgba(255, 255, 255, 0.04));
    overflow: hidden;
  }

  .est-track {
    position: absolute;
    inset: 0 auto 0 0;
    background: color-mix(in srgb, var(--db-ink-muted, #8b949e) 16%, transparent);
    border-radius: 7px 0 0 7px;
  }

  .tracked-bar {
    position: absolute;
    top: 5px;
    bottom: 5px;
    left: 0;
    background: linear-gradient(90deg,
      color-mix(in srgb, var(--project-accent) 65%, transparent),
      var(--project-accent));
    border-radius: 0 4px 4px 0;
    transition: width 500ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  .over-bar {
    position: absolute;
    top: 5px;
    bottom: 5px;
    background: repeating-linear-gradient(
      -45deg,
      #ef4444 0 5px,
      color-mix(in srgb, #ef4444 55%, transparent) 5px 10px
    );
    border-radius: 0 4px 4px 0;
    transition: width 500ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  .est-marker {
    position: absolute;
    top: -2px;
    bottom: -2px;
    width: 2px;
    background: var(--db-ink, #e6edf3);
    opacity: 0.85;
    border-radius: 1px;
  }

  .scale-row {
    display: flex;
    justify-content: space-between;
    font-size: 9px;
    color: var(--db-ink-muted, #8b949e);
    font-variant-numeric: tabular-nums;
    opacity: 0.7;
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
