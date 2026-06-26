<script lang="ts">
  import { getDashboardStatusDistributionByProject } from "$lib/api";
  import type { DashboardProjectStatusDist } from "$lib/types";

  let data = $state<DashboardProjectStatusDist[]>([]);
  let loading = $state(true);
  let error = $state(false);

  $effect(() => {
    loading = true;
    error = false;
    getDashboardStatusDistributionByProject()
      .then((result) => {
        // Sort by total task count, most first
        const sorted = [...result].sort((a, b) => {
          const totalA = a.statuses.reduce((s, st) => s + st.count, 0);
          const totalB = b.statuses.reduce((s, st) => s + st.count, 0);
          return totalB - totalA;
        });
        data = sorted;
        loading = false;
      })
      .catch(() => {
        error = true;
        loading = false;
      });
  });

  let hoveredKey = $state<string | null>(null);

  function totalCount(row: DashboardProjectStatusDist): number {
    return row.statuses.reduce((s, st) => s + st.count, 0);
  }

  function segmentKey(projectId: string, statusId: string): string {
    return `${projectId}:${statusId}`;
  }
</script>

<div class="widget">
  <span class="widget-label">Status by Project</span>

  {#if loading}
    <div class="state-msg">Loading…</div>
  {:else if error}
    <div class="state-msg state-err">Failed to load data</div>
  {:else if data.length === 0}
    <div class="state-msg">No tasks found</div>
  {:else}
    <div class="bars-container">
      {#each data as row (row.project_id)}
        {@const total = totalCount(row)}
        <div class="bar-row">
          <span class="project-name" title={row.project_name}>
            <span class="project-dot" style="background:{row.project_color}"></span>
            {row.project_name}
          </span>
          <div class="bar-track" role="group" aria-label="Status distribution for {row.project_name}">
            {#each row.statuses as slot (slot.status_id)}
              {@const pct = total > 0 ? (slot.count / total) * 100 : 0}
              {@const key = segmentKey(row.project_id, slot.status_id)}
              <div
                class="bar-segment"
                class:hovered={hoveredKey === key}
                style="width:{pct}%; background:{slot.status_color}"
                role="presentation"
                title="{slot.status_name}: {slot.count}"
                onmouseenter={() => { hoveredKey = key; }}
                onmouseleave={() => { hoveredKey = null; }}
              ></div>
            {/each}
          </div>
          <span class="total-count">{total}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .widget {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .widget-label {
    font-size: 13px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--db-ink-muted, #8b949e);
  }

  .bars-container {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  .bar-row {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 28px;
  }

  .project-name {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 14px;
    color: var(--db-ink, #e6edf3);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .project-dot {
    display: inline-block;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .project-name {
    width: 130px;
    flex-shrink: 0;
  }

  .total-count {
    width: 36px;
    flex-shrink: 0;
  }

  .bar-track {
    flex: 1;
    display: flex;
    height: clamp(18px, 38%, 44px);
    border-radius: 4px;
    overflow: hidden;
    background: var(--db-grid-line, rgba(255,255,255,0.05));
  }

  .bar-segment {
    height: 100%;
    transition: filter 150ms ease, opacity 150ms ease;
    min-width: 1px;
  }

  .bar-segment.hovered {
    filter: brightness(1.25);
    opacity: 0.9;
  }

  .total-count {
    font-size: 13px;
    color: var(--db-ink-muted, #8b949e);
    text-align: right;
  }

  .state-msg {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    color: var(--db-ink-muted, #8b949e);
  }

  .state-err {
    color: #ef4444;
  }
</style>
