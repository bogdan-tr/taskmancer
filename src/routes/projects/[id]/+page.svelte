<script lang="ts">
  import { page } from "$app/state";
  import KanbanBoard from "$lib/components/KanbanBoard.svelte";
  import { projectsState } from "$lib/projects.svelte";

  let project = $derived(projectsState.items.find((p) => p.id === page.params.id));
</script>

{#if project}
  {#key project.id}
    <KanbanBoard title={project.name} accentColor={project.color} projectFilter={project.name} />
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
</style>
