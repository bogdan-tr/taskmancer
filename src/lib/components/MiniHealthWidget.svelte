<script lang="ts">
  import { tierLabel } from "$lib/statusLineDisplay";
  import type { ProjectStatusStats, StatusTier } from "$lib/types";

  interface Props {
    stats: ProjectStatusStats;
    projectColor: string;
  }

  let { stats }: Props = $props();

  const TIER_COLORS: Record<StatusTier, string> = {
    great: "#22c55e",
    on_track: "#3b82f6",
    needs_attention: "#f59e0b",
    critical: "#ef4444",
    severe: "#9333ea",
  };

  let tierColor = $derived(TIER_COLORS[stats.status_tier]);
  let label = $derived(tierLabel(stats.status_tier).toUpperCase());
</script>

<div class="mini-health" style="--tier-color: {tierColor}">
  <svg width="20" height="18" viewBox="0 0 100 90" aria-hidden="true">
    <path
      d="M50 80 C50 80 5 50 5 25 A25 25 0 0 1 50 20 A25 25 0 0 1 95 25 C95 50 50 80 50 80Z"
      fill={tierColor}
    />
  </svg>
  <span class="tier-label">{label}</span>
</div>

<style>
  .mini-health {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }

  .tier-label {
    font-size: 11px;
    font-weight: 700;
    color: var(--tier-color);
    letter-spacing: 0.05em;
    line-height: 1;
  }
</style>
