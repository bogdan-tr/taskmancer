<script lang="ts">
  import { getProjectSubprojectTree } from "$lib/api";
  import { tierColor } from "$lib/tierColors";
  import type { ProjectTreeNode } from "$lib/types";
  import WidgetHeader from "./WidgetHeader.svelte";

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

  // ── Treemap layout ────────────────────────────────────────────────────────
  // Tile area ∝ task count (own tasks + subtree). Hierarchy is shown by
  // nesting: a subproject with children renders as a bordered group with a
  // header strip, its children (plus a "Direct" tile for its own tasks)
  // packed inside. Splits are binary along the longer axis, which keeps
  // tiles close to square.

  interface Item {
    name: string;
    color: string;
    tier: string;
    count: number;
    pct: number;
    /** Child items (empty = leaf). */
    children: Item[];
    isDirect: boolean;
  }

  interface Tile {
    item: Item;
    x: number;
    y: number;
    w: number;
    h: number;
    isGroup: boolean;
  }

  const GAP = 3;
  const HEADER_H = 17;

  function weight(it: Item): number {
    return Math.max(1, it.count) + it.children.reduce((s, c) => s + weight(c), 0);
  }

  function toItem(node: ProjectTreeNode, all: ProjectTreeNode[]): Item {
    const kids = all
      .filter((n) => n.parent_id === node.project_id)
      .map((n) => toItem(n, all));
    return {
      name: node.name,
      color: node.color,
      tier: node.health_tier,
      count: node.task_count,
      pct: node.completion_pct,
      children: kids,
      isDirect: false,
    };
  }

  /** A group's own tasks get their own leaf so every task is visible somewhere. */
  function withDirectLeaf(it: Item): Item[] {
    const kids = it.children;
    if (it.count === 0) return kids;
    return [
      {
        name: "Direct",
        color: it.color,
        tier: it.tier,
        count: it.count,
        pct: it.pct,
        children: [],
        isDirect: true,
      },
      ...kids,
    ];
  }

  /** Binary treemap: split items into two ~equal-weight halves, divide the
   *  rect along its longer axis, recurse. */
  function layout(items: Item[], x: number, y: number, w: number, h: number, out: Tile[]) {
    if (items.length === 0 || w < 8 || h < 8) return;
    if (items.length === 1) {
      place(items[0], x, y, w, h, out);
      return;
    }
    const sorted = [...items].sort((a, b) => weight(b) - weight(a));
    const total = sorted.reduce((s, i) => s + weight(i), 0);
    const a: Item[] = [];
    let aWeight = 0;
    for (const it of sorted) {
      // Greedy: fill group A until it holds ~half the weight.
      if (aWeight < total / 2 || a.length === 0) {
        a.push(it);
        aWeight += weight(it);
      }
    }
    const b = sorted.filter((it) => !a.includes(it));
    if (b.length === 0) {
      // Degenerate (one huge item swallowed everything): peel the last off A.
      b.push(a.pop()!);
      aWeight -= weight(b[0]);
    }
    const frac = aWeight / total;
    if (w >= h) {
      const wA = (w - GAP) * frac;
      layout(a, x, y, wA, h, out);
      layout(b, x + wA + GAP, y, (w - GAP) * (1 - frac), h, out);
    } else {
      const hA = (h - GAP) * frac;
      layout(a, x, y, w, hA, out);
      layout(b, x, y + hA + GAP, w, (h - GAP) * (1 - frac), out);
    }
  }

  function place(it: Item, x: number, y: number, w: number, h: number, out: Tile[]) {
    if (it.children.length === 0) {
      out.push({ item: it, x, y, w, h, isGroup: false });
      return;
    }
    out.push({ item: it, x, y, w, h, isGroup: true });
    layout(
      withDirectLeaf(it),
      x + GAP,
      y + HEADER_H,
      w - GAP * 2,
      h - HEADER_H - GAP,
      out,
    );
  }

  let areaW = $state(360);
  let areaH = $state(260);

  let tiles: Tile[] = $derived.by(() => {
    const root = data.find((n) => n.depth === 0);
    if (!root) return [];
    const rootItem = toItem(root, data);
    if (rootItem.children.length === 0) return [];
    const out: Tile[] = [];
    layout(withDirectLeaf(rootItem), 0, 0, areaW, areaH, out);
    return out;
  });

  function tooltip(it: Item): string {
    return `${it.name} — ${it.count} tasks, ${it.pct.toFixed(0)}% done (${it.tier})`;
  }
</script>

<div class="w16" style="--project-accent: {projectColor}">
  <WidgetHeader widgetType="p_subproject_tree" />
  {#if loading}
    <div class="state-msg">Loading…</div>
  {:else if error}
    <div class="state-msg state-err">{error}</div>
  {:else if tiles.length > 0}
    <div class="map-area" bind:clientWidth={areaW} bind:clientHeight={areaH}>
      {#each tiles as t}
        {#if t.isGroup}
          <div
            class="group"
            style="left:{t.x}px; top:{t.y}px; width:{t.w}px; height:{t.h}px;
                   --tile-color:{t.item.color}; --tier:{tierColor(t.item.tier)}"
            title={tooltip(t.item)}
          >
            <span class="group-name">{t.item.name}</span>
            <span class="group-tier"></span>
          </div>
        {:else}
          <div
            class="tile"
            class:complete={t.item.count > 0 && t.item.pct >= 100}
            class:direct={t.item.isDirect}
            style="left:{t.x}px; top:{t.y}px; width:{t.w}px; height:{t.h}px;
                   --tile-color:{t.item.color}; --tier:{tierColor(t.item.tier)}"
            title={tooltip(t.item)}
          >
            {#if t.w > 52 && t.h > 34}
              <span class="tile-name">{t.item.name}</span>
              <span class="tile-meta">
                {t.item.count > 0 ? `${Math.round((t.item.pct / 100) * t.item.count)}/${t.item.count}` : "—"}
                {#if t.item.count > 0}· {t.item.pct.toFixed(0)}%{/if}
              </span>
            {/if}
            <span class="tile-tier"></span>
            {#if t.item.count > 0 && t.h > 22}
              <span class="tile-progress">
                <span class="tile-progress-fill" style="width:{Math.min(100, t.item.pct)}%"></span>
              </span>
            {/if}
          </div>
        {/if}
      {/each}
    </div>
    <div class="map-legend">
      size = tasks · fill = completion · corner dot = health
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
    gap: 5px;
  }

  .map-area {
    flex: 1;
    min-height: 0;
    position: relative;
    overflow: hidden;
  }

  /* ── Group (subproject with children) ── */
  .group {
    position: absolute;
    border-radius: 7px;
    border: 1px solid color-mix(in srgb, var(--tile-color) 45%, transparent);
    background: color-mix(in srgb, var(--tile-color) 6%, transparent);
    overflow: hidden;
  }

  .group-name {
    position: absolute;
    top: 2px;
    left: 6px;
    right: 14px;
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.04em;
    color: color-mix(in srgb, var(--tile-color) 80%, var(--db-ink, #e6edf3));
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .group-tier {
    position: absolute;
    top: 5px;
    right: 5px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--tier);
  }

  /* ── Leaf tile ── */
  .tile {
    position: absolute;
    border-radius: 6px;
    border: 1px solid color-mix(in srgb, var(--tile-color) 55%, transparent);
    background: color-mix(in srgb, var(--tile-color) 20%, transparent);
    overflow: hidden;
    padding: 4px 6px;
    display: flex;
    flex-direction: column;
    gap: 1px;
    transition: background 150ms;
  }

  .tile:hover {
    background: color-mix(in srgb, var(--tile-color) 32%, transparent);
  }

  .tile.complete {
    opacity: 0.55;
  }

  .tile.direct {
    border-style: dashed;
  }

  .tile-name {
    font-size: 10.5px;
    font-weight: 600;
    color: var(--db-ink, #e6edf3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    padding-right: 10px;
  }

  .tile-meta {
    font-size: 9px;
    color: var(--db-ink-muted, #8b949e);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tile-tier {
    position: absolute;
    top: 5px;
    right: 5px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--tier);
  }

  .tile-progress {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 3px;
    background: rgba(0, 0, 0, 0.25);
  }

  .tile-progress-fill {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    background: var(--tile-color);
    border-radius: 0 2px 2px 0;
    transition: width 400ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  .map-legend {
    flex-shrink: 0;
    font-size: 8.5px;
    letter-spacing: 0.05em;
    color: var(--db-ink-muted, #8b949e);
    text-align: center;
    opacity: 0.75;
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
