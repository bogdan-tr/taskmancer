<script lang="ts">
  import { dndzone, type DndEvent } from "svelte-dnd-action";
  import { page } from "$app/state";
  import { updateProject } from "$lib/api";
  import { projectsState, refreshProjects } from "$lib/projects.svelte";
  import { childrenOf, computeZoneOrderUpdates } from "$lib/projectTree";
  import { isExpanded, toggleExpanded } from "$lib/projectTree.svelte";
  import { sidebarState } from "$lib/sidebar.svelte";
  import type { Project } from "$lib/types";
  import ProjectTreeNode from "./ProjectTreeNode.svelte";

  /** Matches `KanbanGrid.svelte`'s own drag animation duration. */
  const FLIP_DURATION_MS = 150;

  interface Props {
    project: Project;
    depth: number;
    onCreateSubproject: (parent: Project) => void;
  }

  let { project, depth, onCreateSubproject }: Props = $props();

  let children = $derived(childrenOf(projectsState.items, project.id));
  let hasChildren = $derived(children.length > 0);
  let expanded = $derived(isExpanded(project.id));

  let zoneItems = $state<Project[]>([]);
  $effect(() => {
    zoneItems = children;
  });

  let dropError = $state("");

  function handleConsider(event: CustomEvent<DndEvent<Project>>) {
    zoneItems = event.detail.items;
  }

  async function handleFinalize(event: CustomEvent<DndEvent<Project>>) {
    zoneItems = event.detail.items;
    const { updates, rejected } = computeZoneOrderUpdates(projectsState.items, project.id, zoneItems);
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
      dropError = error instanceof Error ? error.message : "Failed to move project";
      await refreshProjects();
    }
  }
</script>

<li>
  <div class="project-row" style="--depth: {depth}">
    {#if hasChildren}
      <button
        type="button"
        class="expand-toggle"
        class:expanded
        onclick={() => toggleExpanded(project.id)}
        aria-expanded={expanded}
        aria-label={expanded ? `Collapse ${project.name}` : `Expand ${project.name}`}
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <polyline points="9 18 15 12 9 6" />
        </svg>
      </button>
    {:else}
      <span class="expand-spacer" aria-hidden="true"></span>
    {/if}
    <a
      href="/projects/{project.id}"
      class="nav-link"
      class:active={page.url.pathname === `/projects/${project.id}`}
      title={sidebarState.collapsed ? project.name : undefined}
    >
      <span class="color-dot" style="background: {project.color}" aria-hidden="true"></span>
      {#if !sidebarState.collapsed}<span class="project-name">{project.name}</span>{/if}
    </a>
    {#if !sidebarState.collapsed}
      <button
        type="button"
        class="add-subproject-button"
        onclick={() => onCreateSubproject(project)}
        aria-label={`New subproject of ${project.name}`}
        title={`New subproject of ${project.name}`}
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="12"
          height="12"
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
      </button>
    {/if}
  </div>
  {#if hasChildren && expanded}
    <ul
      class="subproject-list"
      use:dndzone={{ items: zoneItems, flipDurationMs: FLIP_DURATION_MS, dropTargetStyle: {} }}
      onconsider={handleConsider}
      onfinalize={handleFinalize}
    >
      {#each zoneItems as child (child.id)}
        <ProjectTreeNode project={child} depth={depth + 1} {onCreateSubproject} />
      {/each}
    </ul>
    {#if dropError}<p class="drop-error" role="alert">{dropError}</p>{/if}
  {/if}
</li>

<style>
  .project-row {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    padding-left: calc(var(--depth) * 0.9rem);
  }

  .project-row:hover .add-subproject-button {
    opacity: 1;
  }

  .expand-toggle,
  .expand-spacer {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1rem;
    height: 1rem;
    flex-shrink: 0;
  }

  .expand-toggle {
    border: none;
    background: transparent;
    color: var(--color-ink-muted);
    cursor: pointer;
    padding: 0;
  }

  .expand-toggle svg {
    transition: transform var(--duration-fast) var(--ease-out-expo);
  }

  .expand-toggle.expanded svg {
    transform: rotate(90deg);
  }

  .nav-link {
    flex: 1;
    min-width: 0;
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

  .add-subproject-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.25rem;
    height: 1.25rem;
    flex-shrink: 0;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-ink-muted);
    cursor: pointer;
    opacity: 0;
    transition: opacity var(--duration-fast) var(--ease-out-expo);
  }

  .add-subproject-button:hover,
  .add-subproject-button:focus-visible {
    opacity: 1;
    color: var(--color-accent);
  }

  .subproject-list {
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
</style>
