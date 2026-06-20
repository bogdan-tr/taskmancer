<script lang="ts">
  import { dndzone, type DndEvent } from "svelte-dnd-action";
  import type { BoardColumn } from "$lib/kanbanGrouping";
  import type { SeriesEditScope } from "$lib/recurrence";
  import type { Task } from "$lib/types";
  import TaskCard from "./TaskCard.svelte";

  const FLIP_DURATION_MS = 150;

  interface Props {
    boardColumns: BoardColumn[];
    groupByPriority: boolean;
    onConsider: (statusId: string | undefined, bucketIndex: number, event: CustomEvent<DndEvent<Task>>) => void;
    onFinalize: (statusId: string | undefined, bucketIndex: number, event: CustomEvent<DndEvent<Task>>) => void;
    onUpdate: (task: Task, scope?: SeriesEditScope) => void;
    onDelete: (id: string, scope?: SeriesEditScope) => void;
    onRemoveRecurrence: (id: string) => void;
  }

  let { boardColumns, groupByPriority, onConsider, onFinalize, onUpdate, onDelete, onRemoveRecurrence }: Props =
    $props();
</script>

<div class="board">
  {#each boardColumns as column (column.id)}
    <section class="column" style="--status-accent: {column.color}">
      <h2>{column.label}</h2>
      {#each column.buckets as bucket, bucketIndex (bucket.priorityId ?? "other")}
        <div class="priority-group">
          {#if groupByPriority}
            <p class="priority-group-label" style="--priority-color: {bucket.color}">
              {bucket.label}
            </p>
          {/if}
          <ul
            use:dndzone={{
              items: bucket.tasks,
              flipDurationMs: FLIP_DURATION_MS,
              zoneItemTabIndex: -1,
              // Empty object overrides (not merges with) the library default
              // outline: 'rgba(255, 255, 102, 0.7) solid 2px' — disables it.
              dropTargetStyle: {},
            }}
            onconsider={(event) => onConsider(column.id, bucketIndex, event)}
            onfinalize={(event) => onFinalize(column.id, bucketIndex, event)}
          >
            {#each bucket.tasks as task (task.id)}
              <TaskCard {task} {onUpdate} {onDelete} {onRemoveRecurrence} />
            {/each}
          </ul>
        </div>
      {/each}
    </section>
  {/each}
</div>

<style>
  .board {
    display: flex;
    align-items: flex-start;
    gap: var(--space-lg);
    overflow-x: auto;
    padding-bottom: var(--space-sm);
  }

  .column {
    position: relative;
    display: flex;
    flex-direction: column;
    flex: 0 0 var(--column-width, 240px);
    width: var(--column-width, 240px);
    gap: var(--space-sm);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-md);
    min-height: 200px;
  }

  .column::before {
    content: "";
    position: absolute;
    top: 0;
    left: var(--space-md);
    right: var(--space-md);
    height: 3px;
    border-radius: 0 0 var(--radius-pill) var(--radius-pill);
    background: var(--status-accent, var(--color-border-strong));
  }

  .column h2 {
    margin: 0;
    padding-top: var(--space-2xs);
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
  }

  .priority-group {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
  }

  .priority-group-label {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    margin: 0;
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-faint);
  }

  .priority-group-label::before {
    content: "";
    width: 0.5rem;
    height: 0.5rem;
    border-radius: var(--radius-pill);
    background: var(--priority-color, var(--color-border-strong));
    flex-shrink: 0;
  }

  .column ul {
    list-style: none;
    margin: 0;
    padding: 0;
    /* Tall enough that a dragged card's center can land inside this rect
       even when the bucket is empty (svelte-dnd-action computes drop
       targets via element-center-inside-collection-rect). */
    min-height: 2.5rem;
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .column ul:empty {
    border: 1px dashed var(--color-border-strong);
    border-radius: var(--radius-md);
  }
</style>
