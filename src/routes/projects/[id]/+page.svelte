<script lang="ts">
  import { page } from "$app/state";
  import KanbanBoard from "$lib/components/KanbanBoard.svelte";
  import { projectsState } from "$lib/projects.svelte";
  import { ancestorsOf } from "$lib/projectTree";

  let project = $derived(projectsState.items.find((p) => p.id === page.params.id));
  /** Root-first ancestor trail, for the breadcrumb above the board. Empty for a top-level project. */
  let breadcrumb = $derived(project ? [...ancestorsOf(projectsState.items, project.id)].reverse() : []);
</script>

{#if project}
  {#key project.id}
    {#if breadcrumb.length > 0}
      <nav aria-label="Project breadcrumb" class="breadcrumb">
        {#each breadcrumb as ancestor, index (ancestor.id)}
          <a href="/projects/{ancestor.id}">{ancestor.name}</a>
          <span aria-hidden="true">/</span>
        {/each}
        <span class="current">{project.name}</span>
      </nav>
    {/if}
    <KanbanBoard title={project.name} accentColor={project.color} projectFilter={project.id} />
  {/key}
{:else}
  <main class="page">
    <p class="placeholder">Project not found.</p>
  </main>
{/if}

<style>
  .page {
    max-width: 1200px;
    margin: 0 auto;
    padding: var(--space-xl) var(--space-lg) var(--space-2xl);
  }

  .placeholder {
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
  }

  .breadcrumb {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    max-width: 1200px;
    margin: 0 auto;
    padding: var(--space-md) var(--space-lg) 0;
    font-size: var(--text-sm);
    color: var(--color-ink-muted);
  }

  .breadcrumb a {
    color: var(--color-ink-muted);
    text-decoration: none;
  }

  .breadcrumb a:hover {
    color: var(--color-accent);
    text-decoration: underline;
  }

  .breadcrumb .current {
    color: var(--color-ink);
    font-weight: 600;
  }
</style>
