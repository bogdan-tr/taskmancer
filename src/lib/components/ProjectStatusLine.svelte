<script lang="ts">
  import { listStatusLayouts } from "$lib/api";
  import { displayState } from "$lib/displaySettings.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import { statusLineState } from "$lib/statusLine.svelte";
  import {
    formattedStatValue,
    isKnownStatusLineStatId,
    statLabel,
    tierLabel,
    tierTintColor,
    type StatusLineStatId,
  } from "$lib/statusLineDisplay";
  import type { StatLayout } from "$lib/types";

  interface Props {
    /** The currently-viewed project's id — stats and the resolved layout are always scoped to exactly one project. */
    projectId: string;
    /**
     * 3-state per-project override from `ProjectBoard.status_bar_enabled_override`.
     * `undefined` inherits the global `Settings.status_bar_enabled`; `true` forces
     * the bar on; `false` forces it off even when globally enabled.
     */
    statusBarEnabledOverride?: boolean;
  }

  let { projectId, statusBarEnabledOverride }: Props = $props();

  /** Resolved enabled state: per-project override wins, falling back to the global setting (default true). */
  let barEnabled = $derived(
    statusBarEnabledOverride !== undefined
      ? statusBarEnabledOverride
      : (settingsState.current?.status_bar_enabled ?? true),
  );

  /**
   * Only meaningful once `statusLineState` actually holds stats for *this*
   * project — `undefined` both before any load and while a previous
   * project's stats are still showing (see `statusLine.svelte`'s
   * `statusLineState` doc comment), so the bar renders nothing below the
   * header rather than a stale or mismatched project's numbers.
   */
  let stats = $derived(statusLineState.projectId === projectId ? statusLineState.stats : undefined);

  /**
   * The full saved layout list, fetched once per `projectId` change purely
   * to resolve `stats.effective_layout_id` into its `stat_ids`.
   */
  let layouts = $state<StatLayout[]>([]);

  async function loadLayouts(forProjectId: string) {
    try {
      const result = await listStatusLayouts();
      if (projectId === forProjectId) {
        layouts = result;
      }
    } catch {
      // Keep the previously loaded layouts; the bar still renders its
      // header/badge area regardless (see `statIds` below).
    }
  }

  $effect(() => {
    void loadLayouts(projectId);
  });

  /** The resolved layout's ordered stat ids, or `[]` if the layout hasn't loaded yet. */
  let statIds = $derived(
    stats ? layouts.find((l) => l.id === stats.effective_layout_id)?.stat_ids ?? [] : [],
  );

  /** Whether the health badge should be shown — governed by `"status_badge"` being in the layout's stat_ids. */
  let showBadge = $derived(statIds.includes("status_badge"));

  /** `statIds` minus `"status_badge"` and minus any unrecognized id. */
  let displayedStatIds = $derived(
    statIds.filter(
      (id): id is Exclude<StatusLineStatId, "status_badge"> =>
        id !== "status_badge" && isKnownStatusLineStatId(id),
    ),
  );

  /** Whether to apply the tint background to the tiles (keyed to the current tier color). */
  let tilesTint = $derived(settingsState.current?.status_bar_tile_tint ?? false);
</script>

{#if barEnabled}
  <div
    class="status-line"
    style={tilesTint && stats ? `--tier-tint: ${tierTintColor(stats.status_tier)}` : undefined}
  >
    {#if stats && (showBadge || displayedStatIds.length > 0)}
      <div class="stat-tiles">
        {#if showBadge}
          <span
            class="status-badge"
            class:severe={stats.status_tier === "severe"}
            class:critical={stats.status_tier === "critical"}
            class:needs-attention={stats.status_tier === "needs_attention"}
            class:on-track={stats.status_tier === "on_track"}
            class:great={stats.status_tier === "great"}
          >
            {tierLabel(stats.status_tier)}
          </span>
        {/if}
        {#each displayedStatIds as statId (statId)}
          <div class="stat-tile" class:tint-tile={tilesTint}>
            <span class="stat-tile-label">{statLabel(statId)}</span>
            <span class="stat-tile-value">{formattedStatValue(statId, stats)}</span>
          </div>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .status-line {
    padding: var(--space-sm) var(--space-lg);
    margin-bottom: var(--space-lg);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
  }

  .status-badge {
    flex-shrink: 0;
    align-self: center;
    padding: var(--space-4xs) var(--space-xs);
    border-radius: var(--radius-pill);
    font-size: var(--text-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: white;
  }

  .status-badge.severe {
    background: oklch(58% 0.19 25);
  }

  .status-badge.critical {
    background: oklch(62% 0.17 45);
  }

  .status-badge.needs-attention {
    background: oklch(72% 0.15 70);
    color: var(--color-ink);
  }

  .status-badge.on-track {
    background: oklch(75% 0.13 145);
    color: var(--color-ink);
  }

  .status-badge.great {
    background: oklch(68% 0.15 145);
  }

  .stat-tiles {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-sm);
  }

  .stat-tile {
    display: flex;
    flex-direction: column;
    gap: var(--space-4xs);
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-sm);
    background: var(--color-canvas);
    border: 1px solid var(--color-border);
    min-width: 6rem;
  }

  .stat-tile.tint-tile {
    background: color-mix(in oklch, var(--tier-tint) 10%, var(--color-canvas));
    border-color: color-mix(in oklch, var(--tier-tint) 25%, var(--color-border));
  }

  .stat-tile-label {
    font-size: var(--text-xs);
    color: var(--color-ink-muted);
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
  }

  .stat-tile-value {
    font-size: var(--text-sm);
    font-weight: 600;
  }
</style>
