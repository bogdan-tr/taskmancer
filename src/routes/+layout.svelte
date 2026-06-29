<script lang="ts">
  import { onMount } from "svelte";
  import "../styles/global.css";
  import OrphanedSessionsDialog from "$lib/components/OrphanedSessionsDialog.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import VimModeIndicator from "$lib/components/VimModeIndicator.svelte";
  import { initDisplay } from "$lib/displaySettings.svelte";
  import { initGeneral } from "$lib/generalSettings.svelte";
  import { initProjectTree } from "$lib/projectTree.svelte";
  import { refreshProjects } from "$lib/projects.svelte";
  import { refreshSettings } from "$lib/settings.svelte";
  import { initSidebar } from "$lib/sidebar.svelte";
  import { refreshTags } from "$lib/tags.svelte";
  import { initTheme } from "$lib/theme.svelte";
  import {
    ORPHAN_GRACE_MS,
    initTracking,
    isOrphaned,
    refreshActiveSessions,
    trackingState,
  } from "$lib/tracking.svelte";
  import type { TimeEntry } from "$lib/types";

  let { children } = $props();

  initTheme();
  initSidebar();
  initDisplay();
  initGeneral();
  initProjectTree();
  initTracking();

  /** The current launch's orphaned-session batch, surfaced via `OrphanedSessionsDialog` — see that component's own doc comment for why it keeps its own shrinking working copy rather than reading this directly as it resolves entries. */
  let orphanedEntries: TimeEntry[] = $state([]);
  let orphanedDialogOpen = $derived(orphanedEntries.length > 0);

  onMount(() => {
    void refreshProjects();
    void refreshTags();
    void refreshSettings();
    void (async () => {
      await refreshActiveSessions();
      // Captured once here, not read from `trackingState.nowMs` — that
      // ticker only advances while `initTracking`'s interval has already
      // ticked at least once, which could itself be stale/unset immediately
      // after launch. See `isOrphaned`'s own doc comment.
      const now = Date.now();
      orphanedEntries = trackingState.activeSessions.filter((entry) => isOrphaned(entry, now, ORPHAN_GRACE_MS));
    })();
  });

  function handleOrphanedResolved() {
    orphanedEntries = [];
  }
</script>

<div class="app-shell">
  <Sidebar />
  <div class="app-main">
    {@render children()}
  </div>
</div>

<VimModeIndicator />

<OrphanedSessionsDialog
  open={orphanedDialogOpen}
  {orphanedEntries}
  onResolved={handleOrphanedResolved}
/>

<style>
  .app-shell {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }

  .app-main {
    flex: 1;
    min-width: 0;
    overflow: auto;
  }
</style>
