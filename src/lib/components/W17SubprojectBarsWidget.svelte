<script lang="ts">
  import { tierColor } from "$lib/tierColors";
  import { getProjectSubprojectBars } from "$lib/api";
  import type { ProjectSubprojectBar } from "$lib/types";
  import WidgetHeader from "./WidgetHeader.svelte";

  interface Props {
    projectId: string;
    projectColor: string;
  }
  let { projectId, projectColor }: Props = $props();

  let data = $state<ProjectSubprojectBar[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    if (!projectId) return;
    loading = true;
    error = null;
    getProjectSubprojectBars(projectId)
      .then((d) => { data = d; })
      .catch((e) => { error = e instanceof Error ? e.message : String(e); })
      .finally(() => { loading = false; });
  });


  function tierDot(tier: string): string {
    switch (tier) {
      case "Severe":
      case "Critical":        return "▲";
      case "Needs Attention": return "●";
      default:                return "✓";
    }
  }
</script>

<div class="w17" style="--project-accent: {projectColor}">
  <div class="header-row">
    <WidgetHeader widgetType="p_subproject_bars" />
    <span class="count-badge">{data.length}</span>
  </div>
  {#if loading}
    <div class="state-msg">Loading…</div>
  {:else if error}
    <div class="state-msg state-err">{error}</div>
  {:else if data.length > 0}
    <div class="list">
      {#each data as bar}
        <div
          class="bar-item"
          class:is-grandchild={bar.depth > 1}
          class:is-complete={bar.total > 0 && bar.done === bar.total}
          style="--depth-indent: {(bar.depth - 1) * 10}px"
        >
          <!-- Name row -->
          <div class="name-row">
            <span class="indent-spacer" style="width: {(bar.depth - 1) * 10}px"></span>
            <span class="color-pip" style="background: {bar.color}"></span>
            <span class="bar-name">{bar.name}</span>
            <span class="tier-icon" style="color: {tierColor(bar.health_tier)}" title={bar.health_tier}>{tierDot(bar.health_tier)}</span>
            <span class="pct">{bar.completion_pct.toFixed(0)}%</span>
          </div>
          <!-- Progress track -->
          <div class="track-row">
            <span class="indent-spacer" style="width: {(bar.depth - 1) * 10 + 14}px"></span>
            <div class="track">
              <div
                class="fill"
                style="transform: scaleX({bar.completion_pct / 100}); background: {bar.color}"
              ></div>
            </div>
            <span class="ratio">{bar.done}/{bar.total}</span>
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <div class="state-msg">No subprojects</div>
  {/if}
</div>

<style>
  .w17 {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .header-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
  .header-row :global(.widget-header) {
    flex: 1;
    min-width: 0;
  }
  .count-badge {
    font-size: 10px;
    font-weight: 700;
    color: var(--db-ink-muted);
    background: var(--db-border);
    border-radius: 10px;
    padding: 1px 6px;
    flex-shrink: 0;
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .list::-webkit-scrollbar { width: 3px; }
  .list::-webkit-scrollbar-thumb { background: var(--db-border); border-radius: 2px; }

  .bar-item {
    display: flex;
    flex-direction: column;
    gap: 3px;
    transition: opacity 150ms ease;
  }

  /* Spec: grandchildren render at slightly reduced opacity;
     fully-done subprojects are dimmed. */
  .bar-item.is-grandchild {
    opacity: 0.8;
  }
  .bar-item.is-complete {
    opacity: 0.45;
  }

  .name-row {
    display: flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
  }

  .indent-spacer { flex-shrink: 0; }

  .color-pip {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .bar-name {
    flex: 1;
    font-size: 12px;
    color: var(--db-ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .tier-icon {
    font-size: 9px;
    flex-shrink: 0;
  }

  .pct {
    font-size: 11px;
    font-weight: 700;
    color: var(--db-ink);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
    min-width: 32px;
    text-align: right;
  }

  .track-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .track {
    flex: 1;
    height: 4px;
    background: var(--db-border);
    border-radius: 3px;
    overflow: hidden;
    min-width: 0;
  }

  .fill {
    width: 100%;
    height: 100%;
    border-radius: 3px;
    opacity: 0.9;
    transform-origin: left center;
    transition: transform 0.3s ease;
  }

  .ratio {
    font-size: 10px;
    color: var(--db-ink-muted);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
    min-width: 36px;
    text-align: right;
  }

  .state-msg {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    color: var(--db-ink-muted);
  }
  .state-err { color: #ef4444; }
</style>
