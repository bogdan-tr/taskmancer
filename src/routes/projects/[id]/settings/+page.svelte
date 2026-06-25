<script lang="ts">
  import { page } from "$app/state";
  import DeleteProjectSection from "$lib/components/DeleteProjectSection.svelte";
  import ProjectBoardSettings from "$lib/components/ProjectBoardSettings.svelte";
  import ProjectDefaultsSettings from "$lib/components/ProjectDefaultsSettings.svelte";
  import ProjectDetailsSettings from "$lib/components/ProjectDetailsSettings.svelte";
  import ProjectStatusLineBoardSettings from "$lib/components/ProjectStatusLineBoardSettings.svelte";
  import { projectsState } from "$lib/projects.svelte";

  let project = $derived(projectsState.items.find((p) => p.id === page.params.id));
</script>

{#if project}
  {#key project.id}
    <main class="page">
      <header class="page-header">
        <a class="back-link" href="/projects/{project.id}">← Back to board</a>
        <h1 class="page-title">{project.name} board settings</h1>
      </header>

      <ProjectDetailsSettings {project} />
      <ProjectBoardSettings {project} />
      <ProjectDefaultsSettings {project} />
      <ProjectStatusLineBoardSettings {project} />
      <DeleteProjectSection {project} />
    </main>
  {/key}
{:else}
  <main class="page">
    <p class="placeholder">Project not found.</p>
  </main>
{/if}

<style>
  .page {
    max-width: 720px;
    margin: 0 auto;
    padding: var(--space-xl) var(--space-lg) var(--space-2xl);
  }

  .page-header {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    padding-bottom: var(--space-lg);
    margin-bottom: var(--space-xl);
    border-bottom: 1px solid var(--color-border);
  }

  .back-link {
    color: var(--color-ink-muted);
    text-decoration: none;
    font-size: var(--text-sm);
  }

  .back-link:hover {
    color: var(--color-accent);
  }

  .page-title {
    font-size: var(--text-xl);
    font-weight: 600;
    letter-spacing: var(--tracking-tight);
  }

  .placeholder {
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
  }
</style>
