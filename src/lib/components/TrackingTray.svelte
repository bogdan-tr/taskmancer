<script lang="ts">
  import { getErrorMessage } from "$lib/errors";
  import { formatHms } from "$lib/liveTimer";
  import { sidebarState } from "$lib/sidebar.svelte";
  import { refreshTasks, tasksState } from "$lib/tasks.svelte";
  import { liveTrackedSecondsFor, stopTaskTracking, trackingState } from "$lib/tracking.svelte";

  let expanded = $state(false);
  /** The `task_id` of whichever active session currently has a stop call in flight, so its own stop button disables without blocking the others. */
  let pendingTaskId: string | undefined = $state(undefined);
  let errorMessage = $state("");

  function taskTitleFor(taskId: string): string {
    return tasksState.items.find((t) => t.id === taskId)?.title ?? "Unknown task";
  }

  /** Live "total tracked so far" seconds for `taskId`, via `liveTrackedSecondsFor` — needs the task's own `tracked_minutes`, not just its id, so this looks it up the same way `taskTitleFor` does. `0` if the task can't be found (shouldn't happen for a session this tray is actively showing). */
  function liveSecondsFor(taskId: string): number {
    const task = tasksState.items.find((t) => t.id === taskId);
    return (task ? liveTrackedSecondsFor(task) : undefined) ?? 0;
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
            <li class="tray-row">
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
