<script lang="ts">
  import type { ProjectStatusStats } from "$lib/types";

  interface Props {
    stats: ProjectStatusStats;
    projectColor: string;
  }

  let { stats, projectColor }: Props = $props();

  const R = 14;
  const CIRC = 2 * Math.PI * R;

  let pct = $derived(stats.completion_pct ?? 0);
  let dashArray = $derived(`${pct * CIRC} ${CIRC}`);
  let dashOffset = $derived(CIRC * 0.25);
  let displayPct = $derived(Math.round(pct * 100));
</script>

<svg width="36" height="36" viewBox="0 0 36 36" aria-label="{displayPct}% complete">
  <!-- Track -->
  <circle
    cx="18"
    cy="18"
    r={R}
    fill="none"
    stroke="rgba(255,255,255,0.1)"
    stroke-width="6"
  />
  <!-- Fill arc -->
  <circle
    cx="18"
    cy="18"
    r={R}
    fill="none"
    stroke={projectColor}
    stroke-width="6"
    stroke-dasharray={dashArray}
    stroke-dashoffset={dashOffset}
    stroke-linecap="round"
  />
  <!-- Center percentage -->
  <text
    x="18"
    y="21"
    text-anchor="middle"
    font-size="9"
    fill="var(--color-ink, white)"
    font-weight="600"
    font-family="inherit"
  >{displayPct}%</text>
</svg>
