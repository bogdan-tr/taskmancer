<script lang="ts">
  import { listStatusLayouts } from "$lib/api";
  import { displayState } from "$lib/displaySettings.svelte";
  import { projectsState } from "$lib/projects.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import { statusLineState } from "$lib/statusLine.svelte";
  import {
    formattedStatValue,
    isKnownStatusLineStatId,
    MINI_WIDGET_LABELS,
    MINI_WIDGET_STAT_IDS,
    statLabel,
    tierLabel,
    tierTintColor,
    type StatusLineStatId,
    type TextStatId,
  } from "$lib/statusLineDisplay";
  import type { StatLayout } from "$lib/types";
  import MiniCompletionWidget from "./MiniCompletionWidget.svelte";
  import MiniHealthWidget from "./MiniHealthWidget.svelte";
  import MiniFuelWidget from "./MiniFuelWidget.svelte";
  import MiniSparklineWidget from "./MiniSparklineWidget.svelte";

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

  /** The project record for this status line — used to resolve `project.color` for mini widget fills. */
  let project = $derived(projectsState.items.find((p) => p.id === projectId));

  /** Fallback project color when the project record hasn't loaded yet. */
  const FALLBACK_COLOR = "#6366f1";
</script>

{#if barEnabled}
  <div
    class="status-line"
    class:tint={tilesTint && !!stats}
    style={tilesTint && stats ? `--tier-tint: ${tierTintColor(stats.status_tier)}` : undefined}
  >
    {#if stats && (showBadge || displayedStatIds.length > 0)}
      <div class="stat-tiles">
        {#if showBadge}
          <div
            class="stat-tile badge-tile"
            class:severe={stats.status_tier === "severe"}
            class:critical={stats.status_tier === "critical"}
            class:needs-attention={stats.status_tier === "needs_attention"}
            class:on-track={stats.status_tier === "on_track"}
            class:great={stats.status_tier === "great"}
          >
            <span class="stat-tile-label">Status</span>
            <span class="stat-tile-value badge-value">{tierLabel(stats.status_tier)}</span>
          </div>
        {/if}
        {#each displayedStatIds as statId (statId)}
          {#if MINI_WIDGET_STAT_IDS.has(statId)}
            <div class="stat-tile mini-tile" class:tint-tile={tilesTint}>
              <span class="stat-tile-label">{MINI_WIDGET_LABELS[statId]}</span>
              <div class="mini-visual">
                {#if statId === "mini_health" && stats}
                  <MiniHealthWidget {stats} projectColor={project?.color ?? FALLBACK_COLOR} />
                {:else if statId === "mini_completion" && stats}
                  <MiniCompletionWidget {stats} projectColor={project?.color ?? FALLBACK_COLOR} />
                {:else if statId === "mini_fuel" && stats}
                  <MiniFuelWidget {stats} projectColor={project?.color ?? FALLBACK_COLOR} />
                {:else if statId === "mini_sparkline"}
                  <MiniSparklineWidget {projectId} projectColor={project?.color ?? FALLBACK_COLOR} />
                {/if}
              </div>
            </div>
          {:else}
            <div class="stat-tile" class:tint-tile={tilesTint}>
              <span class="stat-tile-label">{statLabel(statId as TextStatId)}</span>
              <span class="stat-tile-value">{formattedStatValue(statId, stats)}</span>
            </div>
          {/if}
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

  .status-line.tint {
    background: color-mix(in oklch, var(--tier-tint) 16%, var(--color-surface));
    border-color: color-mix(in oklch, var(--tier-tint) 35%, var(--color-border));
  }

  .stat-tiles {
    display: flex;
    flex-wrap: wrap;
    align-items: stretch;
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

  .mini-tile {
    min-width: 5rem;
    align-items: center;
  }

  .mini-visual {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 24px;
    width: 100%;
  }

  /* Badge tile: same box shape as other tiles but colored by tier */
  .badge-tile {
    border-width: 0;
  }

  /* Canonical tier palette (see src/lib/tierColors.ts), darkened ~20% so the
     white tile text keeps contrast on the filled background. */
  .badge-tile.severe {
    background: color-mix(in srgb, #ef4444 82%, #000);
  }

  .badge-tile.critical {
    background: color-mix(in srgb, #f97316 78%, #000);
  }

  .badge-tile.needs-attention {
    background: color-mix(in srgb, #f59e0b 72%, #000);
  }

  .badge-tile.on-track {
    background: color-mix(in srgb, #3b82f6 82%, #000);
  }

  .badge-tile.great {
    background: color-mix(in srgb, #22c55e 74%, #000);
  }

  .badge-tile .stat-tile-label,
  .badge-tile .badge-value {
    color: white;
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
    color: var(--color-ink);
  }

  .badge-value {
    font-size: var(--text-sm);
    font-weight: 700;
  }
</style>
