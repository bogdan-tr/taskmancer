<script lang="ts">
  import { getProjectDueTimeline } from "$lib/api";
  import type { ProjectDueDateTimeline } from "$lib/types";
  import WidgetHeader from "./WidgetHeader.svelte";

  interface Props {
    projectId: string;
    projectColor: string;
  }
  let { projectId, projectColor }: Props = $props();

  let data = $state<ProjectDueDateTimeline | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    if (!projectId) return;
    loading = true;
    error = null;
    // Current-state widget: the horizon is always "from today", so it
    // deliberately ignores the dashboard picker (range = all_time).
    getProjectDueTimeline(projectId, "all_time")
      .then((d) => { data = d; })
      .catch((e) => { error = e instanceof Error ? e.message : String(e); })
      .finally(() => { loading = false; });
  });

  interface Bucket {
    key: string;
    label: string;
    count: number;
    kind: "overdue" | "today" | "soon" | "later";
  }

  /** Sums open tasks into: Overdue / Today / Tomorrow / Next 7 days / Later. */
  let buckets: Bucket[] = $derived.by(() => {
    if (!data) return [];
    const today = data.today;
    const d = new Date(today + "T00:00:00");
    const iso = (offset: number) => {
      const x = new Date(d);
      x.setDate(x.getDate() + offset);
      return x.toISOString().slice(0, 10);
    };
    const tomorrow = iso(1);
    const week = iso(7);

    let overdue = 0, todayN = 0, tomorrowN = 0, soon = 0, later = 0;
    for (const p of data.points) {
      if (p.date < today) overdue += p.overdue_count;
      else if (p.date === today) todayN += p.open_count;
      else if (p.date === tomorrow) tomorrowN += p.open_count;
      else if (p.date <= week) soon += p.open_count;
      else later += p.open_count;
    }
    return [
      { key: "overdue", label: "Overdue", count: overdue, kind: "overdue" as const },
      { key: "today", label: "Today", count: todayN, kind: "today" as const },
      { key: "tomorrow", label: "Tomorrow", count: tomorrowN, kind: "soon" as const },
      { key: "week", label: "Next 7 days", count: soon, kind: "soon" as const },
      { key: "later", label: "Later", count: later, kind: "later" as const },
    ];
  });

  let totalOpen = $derived(buckets.reduce((s, b) => s + b.count, 0));
  let maxCount = $derived(Math.max(1, ...buckets.map((b) => b.count)));

  /** "in 12d" / "today" / "overdue" chip for the project deadline. */
  let deadlineChip = $derived.by(() => {
    if (!data?.deadline) return null;
    const days = Math.round(
      (new Date(data.deadline + "T00:00:00").getTime() -
        new Date(data.today + "T00:00:00").getTime()) /
        86400000,
    );
    if (days < 0) return { text: `deadline ${-days}d ago`, urgent: true };
    if (days === 0) return { text: "deadline today", urgent: true };
    return { text: `deadline in ${days}d`, urgent: days <= 7 };
  });
</script>

<div class="w12" style="--project-accent: {projectColor}">
  <WidgetHeader widgetType="p_due_timeline" />
  {#if loading}
    <div class="state-msg">Loading…</div>
  {:else if error}
    <div class="state-msg state-err">{error}</div>
  {:else if data && totalOpen > 0}
    <div class="strip">
      {#each buckets as b (b.key)}
        <div class="bucket bucket-{b.kind}" class:empty={b.count === 0}>
          <span class="bucket-count">{b.count}</span>
          <div class="bucket-meter">
            <div class="meter-fill" style="transform: scaleY({b.count / maxCount})"></div>
          </div>
          <span class="bucket-label">{b.label}</span>
        </div>
      {/each}
    </div>
    {#if deadlineChip}
      <div class="deadline-row">
        <span class="deadline-chip" class:urgent={deadlineChip.urgent}>
          ⚑ {deadlineChip.text}
        </span>
      </div>
    {/if}
  {:else}
    <div class="state-msg">Nothing due — no open tasks with due dates</div>
  {/if}
</div>

<style>
  .w12 {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .strip {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 6px;
  }

  .bucket {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-end;
    gap: 4px;
    padding: 8px 4px 7px;
    border-radius: 9px;
    border: 1px solid var(--db-border, rgba(255, 255, 255, 0.07));
    background: rgba(255, 255, 255, 0.02);
    min-width: 0;
    --bucket-color: var(--db-ink-muted, #8b949e);
    transition: border-color 150ms, background 150ms;
  }

  .bucket-overdue { --bucket-color: #ef4444; }
  .bucket-today { --bucket-color: var(--project-accent); }
  .bucket-soon { --bucket-color: color-mix(in srgb, var(--project-accent) 65%, var(--db-ink-muted, #8b949e)); }
  .bucket-later { --bucket-color: var(--db-ink-muted, #8b949e); }

  .bucket-overdue:not(.empty) {
    border-color: rgba(239, 68, 68, 0.4);
    background: rgba(239, 68, 68, 0.07);
  }

  .bucket-today:not(.empty) {
    border-color: color-mix(in srgb, var(--project-accent) 45%, transparent);
    background: color-mix(in srgb, var(--project-accent) 8%, transparent);
  }

  .bucket.empty {
    opacity: 0.45;
  }

  .bucket-count {
    font-size: clamp(16px, 4cqi, 26px);
    font-weight: 800;
    color: var(--bucket-color);
    font-variant-numeric: tabular-nums;
    line-height: 1;
  }

  .bucket-meter {
    width: 60%;
    max-width: 44px;
    height: 22px;
    flex-shrink: 1;
    min-height: 8px;
    border-radius: 3px;
    background: var(--db-grid-line, rgba(255, 255, 255, 0.05));
    overflow: hidden;
    display: flex;
    align-items: flex-end;
  }

  .meter-fill {
    width: 100%;
    height: 100%;
    background: color-mix(in srgb, var(--bucket-color) 75%, transparent);
    transform-origin: bottom;
    transition: transform 500ms cubic-bezier(0.16, 1, 0.3, 1);
    border-radius: 3px 3px 0 0;
  }

  .bucket-label {
    font-size: 8.5px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--db-ink-muted, #8b949e);
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .deadline-row {
    display: flex;
    justify-content: center;
    flex-shrink: 0;
  }

  .deadline-chip {
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.05em;
    padding: 2px 9px;
    border-radius: 999px;
    color: var(--db-ink-muted, #8b949e);
    border: 1px solid var(--db-border, rgba(255, 255, 255, 0.1));
  }

  .deadline-chip.urgent {
    color: #ef4444;
    border-color: rgba(239, 68, 68, 0.4);
    background: rgba(239, 68, 68, 0.08);
  }

  .state-msg {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12.5px;
    color: var(--db-ink-muted, #8b949e);
    text-align: center;
    padding: 0 8px;
  }
  .state-err { color: #ef4444; }
</style>
