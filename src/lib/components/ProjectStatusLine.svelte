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
  import StatusLineLayoutEditor from "./StatusLineLayoutEditor.svelte";

  interface Props {
    /** The currently-viewed project's id — stats and the resolved layout are always scoped to exactly one project. */
    projectId: string;
    /** Shown next to the health badge in the `"tiles"` style's header row. */
    projectName: string;
  }

  let { projectId, projectName }: Props = $props();

  /**
   * `Settings.status_bar_style` picks which of the 3 visual treatments to
   * render, defaulting to `"tiles"` when settings haven't loaded yet —
   * matching the Rust-side default so the bar never flashes a different
   * style once settings actually arrive.
   */
  let barStyle = $derived(settingsState.current?.status_bar_style ?? "tiles");

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
   * to resolve `stats.effective_layout_id` into its `stat_ids` — there's no
   * per-layout lookup command, and the full list is small (named presets,
   * not a per-project entity), so fetching all of them client-side is
   * simpler than adding a new backend command just for this milestone's
   * read-only need.
   */
  let layouts = $state<StatLayout[]>([]);

  /**
   * Guards against a stale response overwriting fresher state if `projectId`
   * changes again before this fetch resolves — mirrors `TaskEditDialog.svelte`'s
   * `loadSeriesInfo`/`TimeLogSection.svelte`'s identical-shaped guard. The
   * fetched list itself isn't project-scoped, but a layout can be edited/
   * duplicated/deleted out from under an in-flight request (Milestone 4's
   * editor), so an in-flight response for a `projectId` the user has since
   * navigated away from must not clobber whatever loaded after it.
   */
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

  /** The resolved layout's ordered stat ids, or `[]` if the layout hasn't loaded yet or `effective_layout_id` doesn't match any saved layout (a malformed/stale reference degrades to "show nothing" rather than throwing). */
  let statIds = $derived(
    stats ? layouts.find((l) => l.id === stats.effective_layout_id)?.stat_ids ?? [] : [],
  );

  /** `statIds` minus `"status_badge"` (rendered separately in the header) and minus any unrecognized id — the ordered set of stats this bar actually renders as tiles/chips/text. */
  let displayedStatIds = $derived(
    statIds.filter(
      (id): id is Exclude<StatusLineStatId, "status_badge"> => id !== "status_badge" && isKnownStatusLineStatId(id),
    ),
  );
</script>

<div class="status-line" class:tint={barStyle === "tint"} style={barStyle === "tint" && stats ? `--tier-tint: ${tierTintColor(stats.status_tier)}` : undefined}>
  <div class="status-line-header">
    <div class="status-line-identity">
      {#if stats}
        <span class="status-badge" class:severe={stats.status_tier === "severe"} class:critical={stats.status_tier === "critical"} class:needs-attention={stats.status_tier === "needs_attention"} class:on-track={stats.status_tier === "on_track"} class:great={stats.status_tier === "great"}>
          {tierLabel(stats.status_tier)}
        </span>
      {/if}
      <span class="status-line-project-name">{projectName}</span>
    </div>
    <StatusLineLayoutEditor
      {projectId}
      currentLayoutId={stats?.effective_layout_id}
      weekStartsOn={displayState.weekStartsOn}
    />
  </div>

  {#if stats && displayedStatIds.length > 0}
    {#if barStyle === "tiles"}
      <div class="stat-tiles">
        {#each displayedStatIds as statId (statId)}
          <div class="stat-tile">
            <span class="stat-tile-label">{statLabel(statId)}</span>
            <span class="stat-tile-value">{formattedStatValue(statId, stats)}</span>
          </div>
        {/each}
      </div>
    {:else if barStyle === "chips"}
      <div class="stat-chips">
        {#each displayedStatIds as statId, index (statId)}
          {#if index > 0}
            <span class="stat-chip-separator" aria-hidden="true"></span>
          {/if}
          <span class="stat-chip">
            <span class="stat-chip-label">{statLabel(statId)}</span>
            <span class="stat-chip-value">{formattedStatValue(statId, stats)}</span>
          </span>
        {/each}
      </div>
    {:else}
      <div class="stat-tint-row">
        {#each displayedStatIds as statId, index (statId)}
          {#if index > 0}
            <span class="stat-tint-separator" aria-hidden="true">·</span>
          {/if}
          <span class="stat-tint-item">{statLabel(statId)} {formattedStatValue(statId, stats)}</span>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .status-line {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
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

  .status-line-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md);
  }

  .status-line-identity {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    min-width: 0;
  }

  .status-badge {
    flex-shrink: 0;
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

  .status-line-project-name {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-ink-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stat-tiles {
    display: flex;
    flex-wrap: wrap;
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

  .stat-chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-xs);
  }

  .stat-chip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-3xs);
    padding: var(--space-4xs) var(--space-sm);
    border-radius: var(--radius-pill);
    background: var(--color-canvas);
    border: 1px solid var(--color-border);
    font-size: var(--text-xs);
  }

  .stat-chip-label {
    color: var(--color-ink-muted);
  }

  .stat-chip-value {
    font-weight: 600;
  }

  .stat-chip-separator {
    width: 1px;
    height: 0.85rem;
    background: var(--color-border);
  }

  .stat-tint-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2xs);
    font-size: var(--text-sm);
  }

  .stat-tint-item {
    white-space: nowrap;
  }

  .stat-tint-separator {
    color: var(--color-ink-muted);
  }
</style>
