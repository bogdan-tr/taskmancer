<script lang="ts">
  import { getProjectVelocity } from "$lib/api";
  import type { ProjectVelocity } from "$lib/types";

  interface Props {
    projectId: string;
    projectColor: string;
  }
  let { projectId, projectColor }: Props = $props();

  let data = $state<ProjectVelocity | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    if (!projectId) return;
    loading = true;
    error = null;
    getProjectVelocity(projectId)
      .then((d) => { data = d; })
      .catch((e) => { error = e instanceof Error ? e.message : String(e); })
      .finally(() => { loading = false; });
  });

  let trendLabel = $derived((): string => {
    if (!data) return "";
    const prev = data.done_per_week_prev_avg;
    const curr = data.done_per_week_avg;
    if (prev <= 0) return curr > 0 ? "NEW" : "";
    const pct = ((curr - prev) / prev) * 100;
    if (pct >= 0) return `+${pct.toFixed(0)}%`;
    return `${pct.toFixed(0)}%`;
  });

  let trendColor = $derived((): string => {
    if (!data) return "var(--db-ink-muted)";
    const prev = data.done_per_week_prev_avg;
    const curr = data.done_per_week_avg;
    if (prev <= 0) return curr > 0 ? "#22c55e" : "var(--db-ink-muted)";
    return curr >= prev ? "#22c55e" : "#ef4444";
  });

  let trendArrow = $derived((): string => {
    if (!data) return "";
    const prev = data.done_per_week_prev_avg;
    const curr = data.done_per_week_avg;
    if (prev <= 0) return "";
    return curr >= prev ? "↑" : "↓";
  });
</script>

<div class="velocity" style="--project-accent: {projectColor}">
  {#if loading}
    <div class="skeleton">
      <div class="sk-big"></div>
      <div class="sk-sm"></div>
      <div class="divider"></div>
      <div class="sk-big" style="width:50%"></div>
      <div class="sk-sm" style="width:60%"></div>
    </div>
  {:else if error}
    <p class="err">{error}</p>
  {:else if data}
    <div class="sections">
      <!-- Top: weekly avg -->
      <div class="section">
        <span class="big-num">{data.done_per_week_avg.toFixed(1)}</span>
        <span class="sub-label">done/wk (4-wk avg)</span>
        {#if trendLabel()}
          <span class="trend" style="color: {trendColor()}">
            {trendArrow()}{trendLabel()}
          </span>
        {/if}
      </div>

      <div class="rule"></div>

      <!-- Bottom: due next 7 days -->
      <div class="section">
        <span class="big-num sm">{data.due_next_7_days}</span>
        <span class="sub-label">DUE NEXT 7 DAYS</span>
      </div>
    </div>
  {:else}
    <p class="empty">No velocity data.</p>
  {/if}
</div>

<style>
  .velocity {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .sections {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    padding: 8px;
  }

  .section {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    width: 100%;
  }

  .big-num {
    font-size: clamp(1.8rem, 4.5vw, 3rem);
    font-weight: 800;
    color: var(--project-accent);
    line-height: 1;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.03em;
  }

  .big-num.sm {
    font-size: clamp(1.4rem, 3.5vw, 2.4rem);
  }

  .sub-label {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--db-ink-muted);
  }

  .trend {
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.04em;
  }

  .rule {
    width: 60%;
    height: 1px;
    background: var(--db-border);
    margin: 4px 0;
  }

  /* Skeleton */
  .skeleton {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 16px;
    width: 100%;
  }

  .sk-big {
    width: 40%;
    height: 36px;
    border-radius: 6px;
    background: var(--db-border);
    animation: pulse 1.4s ease-in-out infinite;
  }

  .sk-sm {
    width: 55%;
    height: 10px;
    border-radius: 4px;
    background: var(--db-border);
    animation: pulse 1.4s ease-in-out infinite 0.2s;
  }

  .divider {
    width: 50%;
    height: 1px;
    background: var(--db-border);
    margin: 4px 0;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 0.8; }
  }

  .err, .empty {
    margin: 0;
    font-size: 12px;
    color: var(--db-ink-muted);
    text-align: center;
    padding: 16px;
  }
  .err { color: #ef4444; }
</style>
