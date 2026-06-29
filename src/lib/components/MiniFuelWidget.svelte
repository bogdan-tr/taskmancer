<script lang="ts">
  import type { ProjectStatusStats } from "$lib/types";

  interface Props {
    stats: ProjectStatusStats;
    projectColor: string;
  }

  let { stats }: Props = $props();

  let total = $derived(stats.total_time_tracked + stats.estimated_time_left);
  let pct = $derived(total > 0 ? stats.total_time_tracked / total : 0);
  let displayPct = $derived(Math.round(pct * 100));

  let fillColor = $derived(
    pct <= 0.5 ? "#22c55e" : pct <= 0.8 ? "#f59e0b" : "#ef4444",
  );
</script>

<div class="fuel-widget">
  {#if total === 0}
    <span class="no-estimate">No estimate</span>
  {:else}
    <div class="fuel-track">
      <div
        class="fuel-fill"
        style="transform: scaleX({Math.min(pct, 1)}); background: {fillColor}"
      ></div>
    </div>
    <span class="fuel-label" style="color: {fillColor}">{displayPct}% used</span>
  {/if}
</div>

<style>
  .fuel-widget {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 3px;
    width: 100%;
    min-width: 4rem;
  }

  .fuel-track {
    width: 100%;
    height: 10px;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 5px;
    overflow: hidden;
  }

  .fuel-fill {
    width: 100%;
    height: 100%;
    border-radius: 5px;
    transform-origin: left center;
    transition: transform 300ms ease;
  }

  .fuel-label {
    font-size: 10px;
    font-weight: 600;
    line-height: 1;
  }

  .no-estimate {
    font-size: 10px;
    color: var(--color-ink-faint, rgba(255, 255, 255, 0.35));
  }
</style>
