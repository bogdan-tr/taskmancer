<script lang="ts">
  import { goto } from "$app/navigation";
  import { deleteProject, listTasks } from "$lib/api";
  import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
  import DeleteProjectDialog from "$lib/components/DeleteProjectDialog.svelte";
  import {
    buildTaskStrategy,
    isDefaultProject,
    reassignTargets,
    tasksForProject,
    type DeleteStrategyKind,
  } from "$lib/deleteProject";
  import { projectsState, refreshProjects } from "$lib/projects.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import type { Project } from "$lib/types";

  interface Props {
    project: Project;
  }

  let { project }: Props = $props();

  let isDefault = $derived(
    isDefaultProject(project.name, settingsState.current?.default_project ?? ""),
  );
  let otherProjects = $derived(reassignTargets(projectsState.items, project.id));

  let errorMessage = $state("");
  let isDeleting = $state(false);
  let simpleConfirmOpen = $state(false);
  let strategyDialogOpen = $state(false);
  let pendingTaskCount = $state(0);

  async function startDelete() {
    errorMessage = "";
    try {
      const tasks = await listTasks();
      pendingTaskCount = tasksForProject(tasks, project.name).length;
      if (pendingTaskCount === 0) {
        simpleConfirmOpen = true;
      } else {
        strategyDialogOpen = true;
      }
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : "Failed to check project tasks";
    }
  }

  async function confirmSimpleDelete() {
    simpleConfirmOpen = false;
    await performDelete();
  }

  async function confirmStrategyDelete(kind: DeleteStrategyKind, targetProjectId: string) {
    strategyDialogOpen = false;
    await performDelete(buildTaskStrategy(kind, targetProjectId));
  }

  async function performDelete(strategy?: ReturnType<typeof buildTaskStrategy>) {
    isDeleting = true;
    try {
      await deleteProject(project.id, strategy);
      await refreshProjects();
      await goto("/");
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : "Failed to delete project";
    } finally {
      isDeleting = false;
    }
  }
</script>

<section aria-labelledby="delete-project-heading">
  <h2 id="delete-project-heading">Delete project</h2>

  {#if isDefault}
    <p class="hint">
      "{project.name}" is the default project and can't be deleted. Choose a different default
      project in Settings first if you want to remove it.
    </p>
  {:else}
    <p class="hint">
      Permanently remove "{project.name}" from your project list. This can't be undone.
    </p>

    {#if errorMessage}
      <p class="error" role="alert">{errorMessage}</p>
    {/if}

    <div class="actions">
      <button type="button" class="danger" disabled={isDeleting} onclick={startDelete}>
        {isDeleting ? "Deleting…" : "Delete project"}
      </button>
    </div>
  {/if}
</section>

<ConfirmDialog
  open={simpleConfirmOpen}
  title="Delete project?"
  message={`Are you sure you want to delete "${project.name}"? This can't be undone.`}
  confirmLabel="Delete"
  onConfirm={confirmSimpleDelete}
  onCancel={() => (simpleConfirmOpen = false)}
/>

<DeleteProjectDialog
  open={strategyDialogOpen}
  projectName={project.name}
  taskCount={pendingTaskCount}
  {otherProjects}
  onConfirm={confirmStrategyDelete}
  onCancel={() => (strategyDialogOpen = false)}
/>

<style>
  section {
    margin-top: var(--space-2xl);
  }

  h2 {
    margin: 0 0 var(--space-md);
    font-size: var(--text-sm);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
  }

  .hint {
    margin: 0 0 var(--space-md);
    max-width: 32rem;
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
  }

  .error {
    margin: 0 0 var(--space-md);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    background: var(--color-danger-soft);
    color: var(--color-danger);
    font-weight: 600;
    font-size: var(--text-sm);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
  }

  .actions button.danger {
    padding: var(--space-sm) var(--space-lg);
    border-radius: var(--radius-md);
    border: none;
    background: var(--color-danger);
    color: var(--color-accent-ink);
    font-weight: 600;
    font-size: var(--text-base);
    cursor: pointer;
    box-shadow: var(--shadow-sm);
    transition:
      background var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo),
      transform var(--duration-fast) var(--ease-out-expo);
  }

  .actions button.danger:hover:not(:disabled) {
    background: var(--color-danger-hover);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .actions button.danger:disabled {
    background: var(--color-border);
    color: var(--color-ink-muted);
    cursor: not-allowed;
    box-shadow: none;
    transform: none;
  }
</style>
