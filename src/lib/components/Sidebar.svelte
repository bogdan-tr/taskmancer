<script lang="ts">
  import { dndzone, type DndEvent } from "svelte-dnd-action";
  import { page } from "$app/state";
  import { updateProject } from "$lib/api";
  import { vimState } from "$lib/vim.svelte";
  import NewProjectModal from "./NewProjectModal.svelte";
  import ProjectTreeNode from "./ProjectTreeNode.svelte";
  import { getErrorMessage } from "$lib/errors";
  import { projectsState, refreshProjects } from "$lib/projects.svelte";
  import { childrenOf, computeZoneOrderUpdates } from "$lib/projectTree";
  import { expandIfUnset } from "$lib/projectTree.svelte";
  import { sidebarState, toggleSidebar } from "$lib/sidebar.svelte";
  import { containerOwner } from "$lib/subtasks";
  import { tasksState } from "$lib/tasks.svelte";
  import type { Project, SavedView } from "$lib/types";
  import TrackingTray from "./TrackingTray.svelte";
  import { filterViewState } from "$lib/filterViewState.svelte";
  import { savedViewsState, refreshSavedViews } from "$lib/savedViewsState.svelte";
  import { deleteSavedView, reorderSavedViews, updateSavedView } from "$lib/api";

  const FLIP_DURATION_MS = 150;

  let newProjectOpen = $state(false);
  let subprojectParent: Project | undefined = $state(undefined);
  let dropError = $state("");

  /** Top-level projects, minus any auto-generated subtask container — those are reachable only via their owning task's own card/views, never the sidebar. */
  let topLevelProjects = $derived(
    childrenOf(projectsState.items, undefined).filter(
      (project) => containerOwner(project.id, tasksState.items) === undefined,
    ),
  );
  let zoneItems = $state<Project[]>([]);
  $effect(() => {
    zoneItems = topLevelProjects;
  });

  function handleConsider(event: CustomEvent<DndEvent<Project>>) {
    zoneItems = event.detail.items;
  }

  async function handleFinalize(event: CustomEvent<DndEvent<Project>>) {
    zoneItems = event.detail.items;
    const { updates, rejected } = computeZoneOrderUpdates(projectsState.items, undefined, zoneItems);
    if (rejected) {
      dropError = "Can't move a project into one of its own subprojects.";
      await refreshProjects();
      return;
    }

    dropError = "";
    try {
      for (const update of updates) {
        const target = projectsState.items.find((p) => p.id === update.id);
        if (!target) continue;
        await updateProject({ ...target, parent_id: update.parent_id, order: update.order });
      }
      await refreshProjects();
    } catch (error) {
      dropError = getErrorMessage(error, "Failed to move project");
      await refreshProjects();
    }
  }

  function openNewProjectModal() {
    subprojectParent = undefined;
    newProjectOpen = true;
  }

  function openNewSubprojectModal(parent: Project) {
    subprojectParent = parent;
    newProjectOpen = true;
  }

  function handleProjectCreated(project: Project) {
    projectsState.items = [...projectsState.items, project].sort((a, b) => a.order - b.order);
    if (project.parent_id) expandIfUnset(project.parent_id);
  }

  // ── Saved Views ──────────────────────────────────────────────────────────────

  const ICON_GLYPHS: Record<string, string> = {
    star: "★", bolt: "⚡", heart: "♥", flag: "⚑",
    target: "◎", filter: "⊟", tag: "⌖", clock: "◷",
  };

  let savedViewsZone = $state<SavedView[]>([]);
  $effect(() => {
    savedViewsZone = savedViewsState.items;
  });

  let renamingId: string | null = $state(null);
  let renameValue = $state("");

  $effect(() => {
    void refreshSavedViews();
  });

  function handleViewsConsider(event: CustomEvent<DndEvent<SavedView>>) {
    savedViewsZone = event.detail.items;
  }

  async function handleViewsFinalize(event: CustomEvent<DndEvent<SavedView>>) {
    savedViewsZone = event.detail.items;
    savedViewsState.items = savedViewsZone;
    await reorderSavedViews(savedViewsZone.map((v) => v.id));
  }

  function loadSavedView(view: SavedView) {
    try {
      filterViewState.config = JSON.parse(view.filter_config);
      filterViewState.sort = JSON.parse(view.sort_config);
    } catch {
      filterViewState.reset();
    }
    filterViewState.activeViewId = view.id;
    filterViewState.drawerOpen = false;
    filterViewState.requestedView = "filter";
  }

  async function deleteSavedViewItem(id: string) {
    await deleteSavedView(id);
    if (filterViewState.activeViewId === id) filterViewState.activeViewId = null;
    await refreshSavedViews();
  }

  function startRename(view: SavedView) {
    renamingId = view.id;
    renameValue = view.name;
  }

  async function commitRename(view: SavedView) {
    const name = renameValue.trim();
    if (!name || name === view.name) {
      renamingId = null;
      return;
    }
    await updateSavedView(view.id, name, view.color, view.icon, view.filter_config, view.sort_config);
    await refreshSavedViews();
    renamingId = null;
  }

  function openNewFilterView() {
    filterViewState.reset();
    filterViewState.drawerOpen = true;
    filterViewState.requestedView = "filter";
  }
</script>

<nav aria-label="Primary" class="sidebar" class:collapsed={sidebarState.collapsed}>
  <div class="sidebar-header">
    <span class="brand-badge" aria-hidden="true">TM</span>
    {#if !sidebarState.collapsed}
      <span class="brand-mark">taskmancer</span>
    {/if}
    <button
      type="button"
      class="collapse-toggle"
      class:collapsed={sidebarState.collapsed}
      onclick={toggleSidebar}
      aria-expanded={!sidebarState.collapsed}
      aria-label={sidebarState.collapsed ? "Expand sidebar" : "Collapse sidebar"}
      title={sidebarState.collapsed ? "Expand sidebar" : "Collapse sidebar"}
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <polyline points="15 18 9 12 15 6" />
      </svg>
    </button>
  </div>

  <TrackingTray />

  <div class="nav-section views-section">
    {#if !sidebarState.collapsed}
      <h3 class="section-label">Views</h3>
    {/if}
    <a
      href="/"
      class="nav-link"
      class:active={page.url.pathname === "/"}
      class:vim-sidebar-cursor={vimState.sidebarCursorRoute === "/"}
      title={sidebarState.collapsed ? "All Tasks" : undefined}
    >
      <svg
        class="nav-icon"
        xmlns="http://www.w3.org/2000/svg"
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <rect x="3" y="3" width="18" height="18" rx="2" />
        <path d="M8 8h8M8 12h8M8 16h5" />
      </svg>
      {#if !sidebarState.collapsed}<span>All Tasks</span>{/if}
    </a>
    <a
      href="/dashboard"
      class="nav-link"
      class:active={page.url.pathname === "/dashboard"}
      class:vim-sidebar-cursor={vimState.sidebarCursorRoute === "/dashboard"}
      title={sidebarState.collapsed ? "Dashboard" : undefined}
    >
      <svg
        class="nav-icon"
        xmlns="http://www.w3.org/2000/svg"
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <rect x="3" y="3" width="7" height="7" rx="1" />
        <rect x="14" y="3" width="7" height="7" rx="1" />
        <rect x="3" y="14" width="7" height="7" rx="1" />
        <rect x="14" y="14" width="7" height="7" rx="1" />
      </svg>
      {#if !sidebarState.collapsed}<span>Dashboard</span>{/if}
    </a>
  </div>

  <!-- Saved Views -->
  <div class="nav-section saved-views-section">
    {#if !sidebarState.collapsed}
      <div class="section-header">
        <h3 class="section-label">Saved Views</h3>
        <button
          type="button"
          class="section-add-btn"
          onclick={openNewFilterView}
          title="New filter view"
          aria-label="New filter view"
        >+</button>
      </div>
    {:else}
      <button
        type="button"
        class="new-view-collapsed"
        onclick={openNewFilterView}
        title="New filter view"
        aria-label="New filter view"
      >⊟</button>
    {/if}
    {#if savedViewsZone.length > 0}
      <ul
        class="saved-views-list"
        use:dndzone={{ items: savedViewsZone, flipDurationMs: FLIP_DURATION_MS, dropTargetStyle: {} }}
        onconsider={handleViewsConsider}
        onfinalize={handleViewsFinalize}
      >
        {#each savedViewsZone as view (view.id)}
          <li class="saved-view-item" class:view-active={filterViewState.activeViewId === view.id}>
            {#if renamingId === view.id}
              <input
                class="rename-input"
                bind:value={renameValue}
                onblur={() => void commitRename(view)}
                onkeydown={(e) => {
                  if (e.key === "Enter") void commitRename(view);
                  if (e.key === "Escape") renamingId = null;
                }}
              />
            {:else}
              <button
                type="button"
                class="saved-view-btn"
                class:view-active={filterViewState.activeViewId === view.id}
                onclick={() => loadSavedView(view)}
                ondblclick={() => startRename(view)}
                title={sidebarState.collapsed ? view.name : undefined}
              >
                <span class="view-icon" style="color: {view.color}" aria-hidden="true">
                  {ICON_GLYPHS[view.icon] ?? "★"}
                </span>
                {#if !sidebarState.collapsed}
                  <span class="saved-view-name">{view.name}</span>
                {/if}
              </button>
              {#if !sidebarState.collapsed}
                <button
                  type="button"
                  class="view-delete-btn"
                  onclick={() => void deleteSavedViewItem(view.id)}
                  aria-label="Delete {view.name}"
                >×</button>
              {/if}
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <div class="nav-section">
    {#if !sidebarState.collapsed}
      <h3 class="section-label">Projects</h3>
    {/if}
    <ul
      class="project-list"
      use:dndzone={{ items: zoneItems, flipDurationMs: FLIP_DURATION_MS, dropTargetStyle: {} }}
      onconsider={handleConsider}
      onfinalize={handleFinalize}
    >
      {#each zoneItems as project (project.id)}
        <ProjectTreeNode {project} depth={0} onCreateSubproject={openNewSubprojectModal} />
      {/each}
    </ul>
    {#if dropError}<p class="drop-error" role="alert">{dropError}</p>{/if}
    <button
      type="button"
      class="new-project-button"
      onclick={openNewProjectModal}
      title={sidebarState.collapsed ? "New project" : undefined}
    >
      <svg
        class="nav-icon"
        xmlns="http://www.w3.org/2000/svg"
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <line x1="12" y1="5" x2="12" y2="19" />
        <line x1="5" y1="12" x2="19" y2="12" />
      </svg>
      {#if !sidebarState.collapsed}<span>New project</span>{/if}
    </button>
  </div>

  <a
    href="/settings"
    class="nav-link settings-link"
    class:active={page.url.pathname === "/settings"}
    title={sidebarState.collapsed ? "Settings" : undefined}
  >
    <svg
      class="nav-icon"
      xmlns="http://www.w3.org/2000/svg"
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      <circle cx="12" cy="12" r="3" />
      <path
        d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
      />
    </svg>
    {#if !sidebarState.collapsed}<span>Settings</span>{/if}
  </a>
</nav>

<NewProjectModal
  open={newProjectOpen}
  onClose={() => (newProjectOpen = false)}
  onCreated={handleProjectCreated}
  parentProject={subprojectParent}
/>

<style>
  .sidebar {
    --sidebar-width: 15rem;
    --sidebar-width-collapsed: 4.25rem;

    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
    width: var(--sidebar-width);
    flex-shrink: 0;
    padding: var(--space-md);
    background: var(--color-surface);
    border-right: 1px solid var(--color-border);
    transition: width var(--duration-normal) var(--ease-out-expo);
    overflow-y: auto;
    overflow-x: hidden;
    position: relative;
    z-index: 50;
  }

  .sidebar.collapsed {
    width: var(--sidebar-width-collapsed);
    align-items: center;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-2xs) var(--space-2xs) var(--space-md);
    margin-bottom: var(--space-2xs);
    border-bottom: 1px solid var(--color-border);
  }

  .sidebar.collapsed .sidebar-header {
    flex-direction: column;
    gap: var(--space-sm);
  }

  .brand-badge {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    flex-shrink: 0;
    border-radius: var(--radius-md);
    background: var(--color-accent);
    color: var(--color-accent-ink);
    font-size: var(--text-xs);
    font-weight: 700;
    letter-spacing: var(--tracking-wide);
  }

  .brand-mark {
    flex: 1;
    font-size: var(--text-base);
    font-weight: 600;
    letter-spacing: var(--tracking-tight);
    white-space: nowrap;
  }

  .collapse-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.75rem;
    height: 1.75rem;
    flex-shrink: 0;
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface-raised);
    color: var(--color-ink-muted);
    cursor: pointer;
    transition:
      color var(--duration-fast) var(--ease-out-expo),
      border-color var(--duration-fast) var(--ease-out-expo),
      transform var(--duration-fast) var(--ease-out-expo);
  }

  .collapse-toggle:hover {
    color: var(--color-accent);
    border-color: var(--color-accent);
  }

  .collapse-toggle svg {
    transition: transform var(--duration-normal) var(--ease-out-expo);
  }

  .collapse-toggle.collapsed svg {
    transform: rotate(180deg);
  }

  /*
   * :global() here because ProjectTreeNode.svelte (a child component) also
   * renders `.nav-link` anchors for project rows — Svelte's CSS scoping
   * only covers elements a component renders directly, not a child
   * component's own output, even though they end up as real DOM
   * descendants of `.sidebar` below.
   */
  :global(.nav-link) {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-xs) var(--space-sm);
    border-radius: var(--radius-md);
    border-left: 3px solid transparent;
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
    font-weight: 600;
    text-decoration: none;
    white-space: nowrap;
    transition:
      color var(--duration-fast) var(--ease-out-expo),
      background var(--duration-fast) var(--ease-out-expo),
      border-color var(--duration-fast) var(--ease-out-expo);
  }

  .sidebar.collapsed :global(.nav-link) {
    justify-content: center;
    padding: var(--space-xs);
    border-left: none;
  }

  :global(.nav-link:hover) {
    color: var(--color-ink);
    background: var(--color-canvas);
  }

  :global(.nav-link.active) {
    color: var(--color-accent);
    background: var(--color-accent-soft);
    border-left-color: var(--color-accent);
  }

  .sidebar.collapsed :global(.nav-link.active) {
    background: var(--color-accent-soft);
  }

  .nav-icon {
    flex-shrink: 0;
  }

  .nav-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
    margin-top: var(--space-md);
  }

  .views-section {
    margin-top: var(--space-sm);
  }

  .section-label {
    margin: 0;
    padding: 0 var(--space-sm);
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-faint);
  }

  .project-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .drop-error {
    margin: 0;
    padding: var(--space-2xs) var(--space-sm);
    font-size: var(--text-xs);
    color: var(--color-danger);
  }

  .new-project-button {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-xs) var(--space-sm);
    border-radius: var(--radius-md);
    border: 1px dashed var(--color-border-strong);
    background: transparent;
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    transition:
      color var(--duration-fast) var(--ease-out-expo),
      border-color var(--duration-fast) var(--ease-out-expo),
      background var(--duration-fast) var(--ease-out-expo);
  }

  .sidebar.collapsed .new-project-button {
    justify-content: center;
    padding: var(--space-xs);
  }

  .new-project-button:hover {
    color: var(--color-accent);
    border-color: var(--color-accent);
    background: var(--color-accent-soft);
  }

  .settings-link {
    margin-top: auto;
  }

  :global(.vim-sidebar-cursor) {
    outline: 2px solid var(--color-accent);
    outline-offset: -2px;
    background: color-mix(in oklch, var(--color-accent) 10%, transparent) !important;
  }

  /* ── Saved Views section ──────────────────────────────────────────────────── */

  .saved-views-section {
    margin-top: var(--space-sm);
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-right: var(--space-xs);
  }

  .section-add-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--color-ink-faint);
    font-size: 1.125rem;
    line-height: 1;
    cursor: pointer;
    transition: color var(--duration-fast) var(--ease-out-expo);
    padding: 0;
  }

  .section-add-btn:hover {
    color: var(--color-accent);
  }

  .new-view-collapsed {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    background: none;
    border: 1px dashed var(--color-border-strong);
    border-radius: var(--radius-md);
    color: var(--color-ink-muted);
    font-size: 0.875rem;
    cursor: pointer;
    transition: color var(--duration-fast) var(--ease-out-expo),
      border-color var(--duration-fast) var(--ease-out-expo);
  }

  .new-view-collapsed:hover {
    color: var(--color-accent);
    border-color: var(--color-accent);
  }

  .saved-views-list {
    display: flex;
    flex-direction: column;
    gap: 1px;
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .saved-view-item {
    position: relative;
    display: flex;
    align-items: center;
    border-radius: var(--radius-md);
  }

  .saved-view-item:hover .view-delete-btn {
    opacity: 1;
  }

  .saved-view-btn {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-xs) var(--space-sm);
    border-radius: var(--radius-md);
    border: none;
    border-left: 3px solid transparent;
    background: none;
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
    text-align: left;
    transition: color var(--duration-fast) var(--ease-out-expo),
      background var(--duration-fast) var(--ease-out-expo);
  }

  .saved-view-btn:hover {
    color: var(--color-ink);
    background: var(--color-canvas);
  }

  .saved-view-btn.view-active {
    color: var(--color-accent);
    background: var(--color-accent-soft);
    border-left-color: var(--color-accent);
  }

  .sidebar.collapsed .saved-view-btn {
    justify-content: center;
    padding: var(--space-xs);
    border-left: none;
  }

  .view-icon {
    font-size: 1rem;
    flex-shrink: 0;
    line-height: 1;
  }

  .saved-view-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .view-delete-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    flex-shrink: 0;
    margin-right: var(--space-xs);
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--color-ink-faint);
    font-size: 1rem;
    line-height: 1;
    cursor: pointer;
    opacity: 0;
    transition: color var(--duration-fast) var(--ease-out-expo),
      opacity var(--duration-fast) var(--ease-out-expo);
    padding: 0;
  }

  .view-delete-btn:hover {
    color: var(--color-danger);
  }

  .rename-input {
    flex: 1;
    min-width: 0;
    padding: 2px var(--space-xs);
    border: 1px solid var(--color-accent);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-ink);
    font-size: var(--text-sm);
    font-weight: 600;
    margin: 2px var(--space-xs);
  }

  .rename-input:focus {
    outline: none;
  }
</style>
