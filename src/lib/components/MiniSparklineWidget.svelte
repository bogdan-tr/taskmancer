<script lang="ts">
  import { getProjectWeeklyRhythm } from "$lib/api";
  import type { ProjectWeeklyRhythm } from "$lib/types";

  interface Props {
    projectId: string;
    projectColor: string;
  }

  let { projectId, projectColor }: Props = $props();

  let data = $state<ProjectWeeklyRhythm | null>(null);

  $effect(() => {
    getProjectWeeklyRhythm(projectId, "all_time")
      .then((r) => { data = r; })
      .catch(() => {}); // silently — no error state in a mini widget
  });
</script>

{#if data}
  {@const maxH = Math.max(...data.weekday_hours, 0.01)}
  <svg width="56" height="20" viewBox="0 0 56 20" aria-label="Weekly hours sparkline">
    {#each data.weekday_hours as h, i}
      {@const barH = Math.max(2, Math.round((h / maxH) * 18))}
      <rect
        x={i * 8}
        y={20 - barH}
        width="6"
        height={barH}
        fill={projectColor}
        opacity={i === data.today_weekday ? 1 : 0.45}
        rx="1"
      />
    {/each}
  </svg>
{:else}
  <!-- Placeholder: 7 flat bars while loading -->
  <svg width="56" height="20" viewBox="0 0 56 20" aria-hidden="true">
    {#each Array(7) as _, i}
      <rect x={i * 8} y="18" width="6" height="2" fill={projectColor} opacity="0.3" rx="1" />
    {/each}
  </svg>
{/if}
