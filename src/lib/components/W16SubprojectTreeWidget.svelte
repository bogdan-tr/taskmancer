<script lang="ts">
  import { getProjectSubprojectTree } from "$lib/api";
  import type { ProjectTreeNode } from "$lib/types";

  interface Props {
    projectId: string;
    projectColor: string;
  }
  let { projectId, projectColor }: Props = $props();

  let data = $state<ProjectTreeNode[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    if (!projectId) return;
    loading = true;
    error = null;
    getProjectSubprojectTree(projectId)
      .then((d) => { data = d; })
      .catch((e) => { error = e instanceof Error ? e.message : String(e); })
      .finally(() => { loading = false; });
  });

  function tierColor(tier: string): string {
    switch (tier) {
      case "Severe": return "#ef4444";
      case "Critical": return "#f97316";
      case "Needs Attention": return "#eab308";
      case "On Track": return "#22c55e";
      default: return "#6b7280"; // Great
    }
  }

  function tierSymbol(tier: string): string {
    switch (tier) {
      case "Severe": return "▲";
      case "Critical": return "▲";
      case "Needs Attention": return "●";
      case "On Track": return "✓";
      default: return "✓";
    }
  }
</script>

<div class="w16" style="--project-accent: {projectColor}">
  <span class="widget-label">PROJECT TREE</span>
  {#if loading}
    <div class="state-msg">Loading…</div>
  {:else if error}
    <div class="state-msg state-err">{error}</div>
  {:else if data.length > 0}
    <div class="tree-scroll">
      {#each data as node}
        <div class="node" style="--indent: {node.depth * 14}px; --node-color: {node.color}">
          <div class="node-header">
            <!-- Indent + connector -->
            <span class="spacer" style="width: {node.depth * 14}px"></span>
            {#if node.depth > 0}
              <span class="connector">└</span>
            {/if}
            <!-- Color dot -->
            <span class="color-dot" style="background: {node.color}"></span>
            <!-- Name -->
            <span class="node-name">{node.name}</span>
            <!-- Health indicator -->
            <span class="tier-badge" style="color: {tierColor(node.health_tier)}" title={node.health_tier}>
              {tierSymbol(node.health_tier)}
            </span>
            <!-- Task count -->
            <span class="task-count">{node.task_count}</span>
          </div>

          <!-- Progress bar -->
          {#if node.task_count > 0}
            <div class="progress-row">
              <span class="spacer" style="width: {node.depth * 14 + (node.depth > 0 ? 22 : 8)}px"></span>
              <div class="prog-track">
                <div
                  class="prog-fill"
                  style="transform: scaleX({node.completion_pct / 100}); background: {node.color}"
                ></div>
              </div>
              <span class="pct-label">{node.completion_pct.toFixed(0)}%</span>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {:else}
    <div class="state-msg">No subprojects</div>
  {/if}
</div>

<style>
  .w16 {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .widget-label {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--db-ink-muted);
    flex-shrink: 0;
  }
  .tree-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .tree-scroll::-webkit-scrollbar { width: 3px; }
  .tree-scroll::-webkit-scrollbar-thumb { background: var(--db-border); border-radius: 2px; }

  .node {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .node-header {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }

  .spacer { flex-shrink: 0; }

  .connector {
    font-size: 10px;
    color: var(--db-border);
    flex-shrink: 0;
    margin-right: 2px;
    line-height: 1;
  }

  .color-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .node-name {
    flex: 1;
    font-size: 12px;
    color: var(--db-ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .tier-badge {
    font-size: 9px;
    flex-shrink: 0;
    line-height: 1;
  }

  .task-count {
    font-size: 10px;
    color: var(--db-ink-muted);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
    min-width: 20px;
    text-align: right;
  }

  .progress-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .prog-track {
    flex: 1;
    height: 3px;
    background: var(--db-border);
    border-radius: 2px;
    overflow: hidden;
    min-width: 0;
  }

  .prog-fill {
    width: 100%;
    height: 100%;
    border-radius: 2px;
    opacity: 0.85;
    transform-origin: left center;
    transition: transform 0.3s ease;
  }

  .pct-label {
    font-size: 9px;
    color: var(--db-ink-muted);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
    min-width: 26px;
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
