<script lang="ts">
  import { page } from "$app/state";
  import NewProjectModal from "./NewProjectModal.svelte";
  import { projectsState } from "$lib/projects.svelte";
  import { sidebarState, toggleSidebar } from "$lib/sidebar.svelte";
  import type { Project } from "$lib/types";

  let newProjectOpen = $state(false);

  function handleProjectCreated(project: Project) {
    projectsState.items = [...projectsState.items, project].sort((a, b) => a.order - b.order);
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

  <a
    href="/"
    class="nav-link"
    class:active={page.url.pathname === "/"}
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

  <div class="nav-section">
    {#if !sidebarState.collapsed}
      <h3 class="section-label">Projects</h3>
    {/if}
    <ul class="project-list">
      {#each projectsState.items as project (project.id)}
        <li>
          <a
            href="/projects/{project.id}"
            class="nav-link"
            class:active={page.url.pathname === `/projects/${project.id}`}
            title={sidebarState.collapsed ? project.name : undefined}
          >
            <span class="color-dot" style="background: {project.color}" aria-hidden="true"></span>
            {#if !sidebarState.collapsed}<span class="project-name">{project.name}</span>{/if}
          </a>
        </li>
      {/each}
    </ul>
    <button
      type="button"
      class="new-project-button"
      onclick={() => (newProjectOpen = true)}
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

  .nav-link {
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

  .sidebar.collapsed .nav-link {
    justify-content: center;
    padding: var(--space-xs);
    border-left: none;
  }

  .nav-link:hover {
    color: var(--color-ink);
    background: var(--color-canvas);
  }

  .nav-link.active {
    color: var(--color-accent);
    background: var(--color-accent-soft);
    border-left-color: var(--color-accent);
  }

  .sidebar.collapsed .nav-link.active {
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

  .color-dot {
    width: 0.625rem;
    height: 0.625rem;
    border-radius: var(--radius-pill);
    flex-shrink: 0;
  }

  .project-name {
    overflow: hidden;
    text-overflow: ellipsis;
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
</style>
