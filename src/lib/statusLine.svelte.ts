import { getProjectStatusStats } from "./api";
import type { WeekStartsOn } from "./displaySettings.svelte";
import type { ProjectStatusStats } from "./types";

/**
 * Boxed in an object because Svelte 5 forbids exporting a reassigned `$state`
 * binding directly from a module — only its properties may be mutated.
 *
 * Unlike `tasksState`/`projectsState` (a global cache of every item), this
 * holds stats for at most *one* project at a time — whichever project's
 * board is currently being viewed. `projectId` records which project
 * `stats` belongs to, so a component can tell "the previous project's stats
 * are still showing while the new project's load" (`projectId` differs from
 * the project actually being viewed) apart from "nothing has loaded yet at
 * all" (`stats` is `undefined`). `stats` is left in place on a failed
 * refresh — see `refreshProjectStatusStats` — which means it can also be
 * stale data *for the same* `projectId`, not just a different one; callers
 * that need to distinguish those two staleness cases should track their own
 * loading flag rather than infer it from this store alone.
 *
 * `projectId` and `stats` are only ever written together, both *after* a
 * fetch resolves — never eagerly set to the new id before the request
 * completes. So while a fetch for a newly-viewed project is in flight, this
 * pair still reads as whichever project's stats loaded last, not the new
 * (not-yet-arrived) one — exactly the "stale value belongs to the old
 * project" reading `hasFreshStatsFor`/`ProjectStatusLine.svelte` rely on.
 */
export const statusLineState = $state<{ projectId: string | undefined; stats: ProjectStatusStats | undefined }>({
  projectId: undefined,
  stats: undefined,
});

/**
 * Loads `projectId`'s status-line stats from the backend. On failure the
 * previously loaded stats are left in place (even if they belonged to a
 * different project) so the bar keeps showing *something* rather than
 * blanking out on a transient error — mirroring `refreshTasks`/
 * `refreshActiveSessions`'s swallow-and-keep-stale convention. `projectId`
 * and `stats` are only updated together once the call resolves (see
 * `statusLineState`'s own doc comment) — a component checking
 * `statusLineState.projectId === viewedProjectId` mid-flight still reads the
 * old, not-yet-overwritten value, correctly attributing it to whichever
 * project loaded last rather than the new one this call is fetching.
 *
 * Deliberately not reactive: callers decide exactly when to re-fetch (see
 * `ProjectStatusLine.svelte` and `KanbanBoard.svelte`'s call sites) rather
 * than this store recomputing on every unrelated state change — each call
 * does real file I/O and a SQLite query on the backend, so it must stay an
 * explicit, deliberate action.
 */
export async function refreshProjectStatusStats(
  projectId: string,
  weekStartsOn: WeekStartsOn,
): Promise<void> {
  try {
    const stats = await getProjectStatusStats(projectId, weekStartsOn);
    statusLineState.projectId = projectId;
    statusLineState.stats = stats;
  } catch {
    // Keep the previously loaded stats (possibly for a different project).
  }
}

/** `true` when `statusLineState.stats` is loaded and belongs to `projectId` specifically — `false` both before any load and while stats for a *different* project are still showing. */
export function hasFreshStatsFor(projectId: string): boolean {
  return statusLineState.projectId === projectId && statusLineState.stats !== undefined;
}
