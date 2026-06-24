<script lang="ts">
  import { legibleInkColor, neonCardColor, NEON_CARD_CHROMA_BOOST, NEON_CARD_LIGHTNESS } from "$lib/colorPresets";
  import { displayState } from "$lib/displaySettings.svelte";
  import { getErrorMessage } from "$lib/errors";
  import { formatHms } from "$lib/liveTimer";
  import { resolveCardLightness, resolveInkMode, resolveProjectColor } from "$lib/projectColor";
  import { projectsState } from "$lib/projects.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import { sidebarState } from "$lib/sidebar.svelte";
  import { refreshTasks, tasksState } from "$lib/tasks.svelte";
  import { liveTrackedSecondsFor, projectTrackedByTask, stopTaskTracking, trackingState } from "$lib/tracking.svelte";
  import type { Task } from "$lib/types";

  let expanded = $state(false);
  /** The `task_id` of whichever active session currently has a stop call in flight, so its own stop button disables without blocking the others. */
  let pendingTaskId: string | undefined = $state(undefined);
  let errorMessage = $state("");

  function taskFor(taskId: string): Task | undefined {
    return tasksState.items.find((t) => t.id === taskId);
  }

  /**
   * `"{Project name} project"` for a project's hidden tracking anchor task
   * (its own stored title, `"{Project name} — General time"`, is an
   * internal implementation detail of how the anchor is represented, not
   * something to surface verbatim — see `Task.hidden`'s own doc comment),
   * or the task's real title for an ordinary task.
   */
  function taskTitleFor(taskId: string): string {
    const project = projectTrackedByTask(taskId, projectsState.items);
    if (project) return `${project.name} project`;
    return taskFor(taskId)?.title ?? "Unknown task";
  }

  /** Live "total tracked so far" seconds for `taskId`, via `liveTrackedSecondsFor` — needs the task's own `tracked_minutes`, not just its id, so this looks it up the same way `taskTitleFor` does. `0` if the task can't be found (shouldn't happen for a session this tray is actively showing). */
  function liveSecondsFor(taskId: string): number {
    const task = taskFor(taskId);
    return (task ? liveTrackedSecondsFor(task) : undefined) ?? 0;
  }

  /** `true` when the row for `taskId` should pick up its task's project color, mirroring `TaskCard.svelte`'s own `isColorCoded` gate — when the global "color code" display setting is off, cards render with no project tint, and this tray must match that exactly rather than forcing color the cards themselves don't have. */
  function isRowColorCoded(taskId: string): boolean {
    return displayState.cardColorMode === "color_code" && taskFor(taskId) !== undefined;
  }

  /**
   * `--tray-row-bg`/`--tray-row-text` custom-property values for `taskId`'s
   * row, via the exact same `resolveProjectColor`/`resolveCardLightness`/
   * `neonCardColor`/`resolveInkMode`/`legibleInkColor` derivation chain
   * `TaskCard.svelte` itself uses for `--task-color-code-bg`/`-text` — so a
   * task being tracked here renders with the identical background/text
   * color its own kanban card would. Only meaningful when `isRowColorCoded`
   * is `true`; the CSS rule consuming these variables is scoped to that
   * same condition via the `.tray-row-colored` class.
   */
  function rowColorStyle(taskId: string): string {
    const task = taskFor(taskId);
    if (!task) return "";

    const projectColor = resolveProjectColor(task.project_id, projectsState.items);
    const cardLightness = resolveCardLightness(
      task.project_id,
      projectsState.items,
      settingsState.current?.card_lightness ?? NEON_CARD_LIGHTNESS,
    );
    const background = neonCardColor(projectColor, cardLightness, NEON_CARD_CHROMA_BOOST);
    const inkMode = resolveInkMode(task.project_id, projectsState.items, settingsState.current?.ink_mode ?? "auto");
    const textColor = legibleInkColor(background, inkMode);

    return `--tray-row-bg: ${background}; --tray-row-text: ${textColor};`;
  }

  /**
   * Stops `taskId`'s session from the tray. Always calls `refreshTasks`
   * afterward (not just `upsertCachedTask`) — the tray has no convenient way
   * to know whether `taskId` is a project's hidden tracker task without an
   * extra lookup, and a full task-list refresh is cheap and low-frequency
   * enough here that correctness wins over the minor extra fetch (mirrors
   * `stopProjectTracking`'s own already-existing `refreshTasks` call).
   */
  async function handleStop(taskId: string) {
    pendingTaskId = taskId;
    errorMessage = "";
    try {
      await stopTaskTracking(taskId);
      await refreshTasks();
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to stop tracking");
    } finally {
      pendingTaskId = undefined;
    }
  }
</script>

{#if trackingState.activeSessions.length > 0}
  {#if sidebarState.collapsed}
    <div class="tray-badge" title="{trackingState.activeSessions.length} timers running">
      <svg viewBox="0 0 16 16" width="14" height="14" fill="currentColor" aria-hidden="true">
        <path
          d="M8 1a7 7 0 1 0 7 7 7 7 0 0 0-7-7zm0 12.5A5.5 5.5 0 1 1 13.5 8 5.5 5.5 0 0 1 8 13.5zM8.75 4.5h-1.5v4l3.25 1.95.75-1.23-2.5-1.5z"
        />
      </svg>
      <span class="tray-badge-count">{trackingState.activeSessions.length}</span>
    </div>
  {:else}
    <div class="tray">
      <button
        type="button"
        class="tray-summary"
        onclick={() => (expanded = !expanded)}
        aria-expanded={expanded}
        aria-controls="tracking-tray-list"
      >
        <svg viewBox="0 0 16 16" width="14" height="14" fill="currentColor" aria-hidden="true">
          <path
            d="M8 1a7 7 0 1 0 7 7 7 7 0 0 0-7-7zm0 12.5A5.5 5.5 0 1 1 13.5 8 5.5 5.5 0 0 1 8 13.5zM8.75 4.5h-1.5v4l3.25 1.95.75-1.23-2.5-1.5z"
          />
        </svg>
        <span class="tray-summary-text">
          {trackingState.activeSessions.length}
          {trackingState.activeSessions.length === 1 ? "timer" : "timers"} running
        </span>
        <svg
          class="tray-chevron"
          class:expanded
          viewBox="0 0 16 16"
          width="12"
          height="12"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <polyline points="4 6 8 10 12 6" />
        </svg>
      </button>

      {#if expanded}
        <ul class="tray-list" id="tracking-tray-list">
          {#each trackingState.activeSessions as entry (entry.id)}
            <li
              class="tray-row"
              class:tray-row-colored={isRowColorCoded(entry.task_id)}
              style={isRowColorCoded(entry.task_id) ? rowColorStyle(entry.task_id) : undefined}
            >
              <span class="tray-row-title">{taskTitleFor(entry.task_id)}</span>
              <span class="tray-row-ticker">{formatHms(liveSecondsFor(entry.task_id))}</span>
              <button
                type="button"
                class="tray-stop-button"
                disabled={pendingTaskId === entry.task_id}
                onclick={() => handleStop(entry.task_id)}
                aria-label="Stop tracking {taskTitleFor(entry.task_id)}"
                title="Stop tracking"
              >
                <svg viewBox="0 0 16 16" width="11" height="11" fill="currentColor" aria-hidden="true">
                  <rect x="3" y="2" width="3.5" height="12" rx="0.75" />
                  <rect x="9.5" y="2" width="3.5" height="12" rx="0.75" />
                </svg>
              </button>
            </li>
          {/each}
        </ul>
      {/if}

      {#if errorMessage}
        <p class="tray-error" role="alert">{errorMessage}</p>
      {/if}
    </div>
  {/if}
{/if}

<style>
  .tray {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
    margin-bottom: var(--space-md);
    padding: var(--space-2xs);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-accent-soft);
  }

  .tray-summary {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding: var(--space-xs) var(--space-sm);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-accent);
    font-size: var(--text-sm);
    font-weight: 600;
    cursor: pointer;
    width: 100%;
    text-align: left;
  }

  .tray-summary:hover {
    background: color-mix(in oklch, var(--color-accent-soft) 60%, transparent);
  }

  .tray-summary-text {
    flex: 1;
  }

  .tray-chevron {
    flex-shrink: 0;
    transition: transform var(--duration-fast) var(--ease-out-expo);
  }

  .tray-chevron.expanded {
    transform: rotate(180deg);
  }

  .tray-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-3xs);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .tray-row {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
  }

  /* Matches the tracked task's own kanban card background exactly, via the
     `--tray-row-bg`/`--tray-row-text` custom properties `rowColorStyle`
     computes with the identical derivation `TaskCard.svelte` uses for
     `--task-color-code-bg`/`-text`. Only the title text adapts for
     legibility — the ticker keeps its own accent color regardless,
     mirroring how `.chip.tracked-live` is likewise untouched by
     `.task.color-coded` on the card itself. */
  .tray-row-colored {
    background: var(--tray-row-bg);
  }

  .tray-row-colored .tray-row-title {
    color: var(--tray-row-text);
  }

  .tray-row-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--color-ink);
  }

  .tray-row-ticker {
    flex-shrink: 0;
    font-size: var(--text-xs);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    color: var(--color-accent);
  }

  .tray-stop-button {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 1.5rem;
    height: 1.5rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface-raised);
    color: var(--color-ink-muted);
    cursor: pointer;
    transition:
      background var(--duration-fast) var(--ease-out-expo),
      color var(--duration-fast) var(--ease-out-expo);
  }

  .tray-stop-button:hover:not(:disabled) {
    background: var(--color-canvas);
    color: var(--color-danger);
  }

  .tray-stop-button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .tray-error {
    margin: 0;
    padding: var(--space-2xs) var(--space-sm);
    font-size: var(--text-xs);
    color: var(--color-danger);
  }

  .tray-badge {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3xs);
    margin-bottom: var(--space-md);
    padding: var(--space-xs);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-accent-soft);
    color: var(--color-accent);
  }

  .tray-badge-count {
    font-size: var(--text-xs);
    font-weight: 700;
  }
</style>
