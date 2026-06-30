<script lang="ts">
  import { filterViewState, DEFAULT_FILTER_CONFIG } from "$lib/filterViewState.svelte";
  import { projectsState } from "$lib/projects.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import { FALLBACK_PRIORITIES, sortedPriorities } from "$lib/priorities.svelte";
  import { FALLBACK_STATUSES, sortedStatuses, statusColor } from "$lib/statuses.svelte";
  import { childrenOf } from "$lib/projectTree";
  import type { FilterConfig, DueDateFilter } from "$lib/types";

  const priorities = $derived(
    sortedPriorities(settingsState.current?.priorities ?? FALLBACK_PRIORITIES),
  );
  const statuses = $derived(
    sortedStatuses(settingsState.current?.statuses ?? FALLBACK_STATUSES),
  );
  const topLevelProjects = $derived(childrenOf(projectsState.items, undefined));

  function update(patch: Partial<FilterConfig>) {
    filterViewState.config = { ...filterViewState.config, ...patch };
    filterViewState.activeViewId = null;
  }

  function updateDue(patch: Partial<DueDateFilter>) {
    update({ dueFilter: { ...filterViewState.config.dueFilter, ...patch } });
  }

  function updateScheduled(patch: Partial<DueDateFilter>) {
    update({ scheduledFilter: { ...filterViewState.config.scheduledFilter, ...patch } });
  }

  function toggleStatus(id: string) {
    const s = filterViewState.config.statuses;
    update({ statuses: s.includes(id) ? s.filter((x) => x !== id) : [...s, id] });
  }

  function toggleProject(id: string) {
    const p = filterViewState.config.projectIds;
    update({ projectIds: p.includes(id) ? p.filter((x) => x !== id) : [...p, id] });
  }

  function togglePriority(id: string) {
    const p = filterViewState.config.priorities;
    update({ priorities: p.includes(id) ? p.filter((x) => x !== id) : [...p, id] });
  }

  function addTag(e: KeyboardEvent) {
    if (e.key !== "Enter" && e.key !== ",") return;
    e.preventDefault();
    const input = e.currentTarget as HTMLInputElement;
    const val = input.value.trim().replace(/^#/, "");
    if (!val) return;
    if (!filterViewState.config.tags.includes(val)) {
      update({ tags: [...filterViewState.config.tags, val] });
    }
    input.value = "";
  }

  function removeTag(tag: string) {
    update({ tags: filterViewState.config.tags.filter((t) => t !== tag) });
  }

  function resetAll() {
    filterViewState.config = { ...DEFAULT_FILTER_CONFIG };
    filterViewState.activeViewId = null;
  }

  // Recursive project tree renderer helper
  function projectChildren(parentId: string | undefined) {
    return childrenOf(projectsState.items, parentId);
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<aside class="filter-drawer" aria-label="Filter options">
  <div class="drawer-header">
    <span class="drawer-title">Filters</span>
    <div class="drawer-header-actions">
      <button type="button" class="reset-btn" onclick={resetAll}>Reset all</button>
      <button
        type="button"
        class="close-btn"
        onclick={() => (filterViewState.drawerOpen = false)}
        aria-label="Close filter drawer"
      >
        ×
      </button>
    </div>
  </div>

  <div class="drawer-body">
    <!-- Group mode -->
    <section class="filter-section">
      <h4 class="section-label">Match</h4>
      <div class="radio-row">
        <label>
          <input
            type="radio"
            name="groupMode"
            value="all"
            checked={filterViewState.config.groupMode === "all"}
            onchange={() => update({ groupMode: "all" })}
          />
          All conditions (AND)
        </label>
        <label>
          <input
            type="radio"
            name="groupMode"
            value="any"
            checked={filterViewState.config.groupMode === "any"}
            onchange={() => update({ groupMode: "any" })}
          />
          Any condition (OR)
        </label>
      </div>
    </section>

    <!-- Text search -->
    <section class="filter-section">
      <h4 class="section-label">Text search</h4>
      <input
        type="text"
        class="text-input"
        placeholder="Search text…"
        value={filterViewState.config.text}
        oninput={(e) => update({ text: (e.currentTarget as HTMLInputElement).value })}
      />
      <label class="checkbox-label">
        <input
          type="checkbox"
          checked={filterViewState.config.titleOnly}
          onchange={(e) => update({ titleOnly: (e.currentTarget as HTMLInputElement).checked })}
        />
        Title only
      </label>
    </section>

    <!-- Status -->
    <section class="filter-section">
      <h4 class="section-label">Status</h4>
      <div class="chip-group">
        {#each statuses as status (status.id)}
          {@const active = filterViewState.config.statuses.includes(status.id)}
          <button
            type="button"
            class="chip"
            class:chip-active={active}
            style="--chip-color: {statusColor(statuses, status.id)}"
            onclick={() => toggleStatus(status.id)}
          >
            {status.label}
          </button>
        {/each}
      </div>
      {#if filterViewState.config.statuses.length > 0}
        <button type="button" class="clear-link" onclick={() => update({ statuses: [] })}>
          Clear
        </button>
      {/if}
    </section>

    <!-- Project -->
    <section class="filter-section">
      <h4 class="section-label">Project</h4>
      <div class="project-tree">
        {#each topLevelProjects as project (project.id)}
          {@const active = filterViewState.config.projectIds.includes(project.id)}
          <label class="project-row">
            <input
              type="checkbox"
              checked={active}
              onchange={() => toggleProject(project.id)}
            />
            <span class="project-dot" style="background:{project.color}"></span>
            <span class="project-name">{project.name}</span>
          </label>
          {#each projectChildren(project.id) as child (child.id)}
            {@const childActive = filterViewState.config.projectIds.includes(child.id)}
            <label class="project-row project-row-child">
              <input
                type="checkbox"
                checked={childActive}
                onchange={() => toggleProject(child.id)}
              />
              <span class="project-dot" style="background:{child.color}"></span>
              <span class="project-name">{child.name}</span>
            </label>
          {/each}
        {/each}
      </div>
      {#if filterViewState.config.projectIds.length > 0}
        <button type="button" class="clear-link" onclick={() => update({ projectIds: [] })}>
          Clear
        </button>
      {/if}
    </section>

    <!-- Tags -->
    <section class="filter-section">
      <h4 class="section-label">Tags</h4>
      <div class="tag-chips">
        {#each filterViewState.config.tags as tag (tag)}
          <span class="tag-chip">
            #{tag}
            <button type="button" class="tag-remove" onclick={() => removeTag(tag)} aria-label="Remove #{tag}">×</button>
          </span>
        {/each}
        <input
          type="text"
          class="tag-input"
          placeholder="Add tag…"
          onkeydown={addTag}
        />
      </div>
      {#if filterViewState.config.tags.length > 1}
        <div class="radio-row">
          <label>
            <input
              type="radio"
              name="tagMode"
              value="any"
              checked={filterViewState.config.tagMode === "any"}
              onchange={() => update({ tagMode: "any" })}
            />
            Match any tag
          </label>
          <label>
            <input
              type="radio"
              name="tagMode"
              value="all"
              checked={filterViewState.config.tagMode === "all"}
              onchange={() => update({ tagMode: "all" })}
            />
            Match all tags
          </label>
        </div>
      {/if}
    </section>

    <!-- Due date -->
    <section class="filter-section">
      <h4 class="section-label">Due date</h4>
      {#each (["any", "overdue", "today", "this_week", "this_month", "custom"] as const) as opt}
        <label class="radio-label">
          <input
            type="radio"
            name="dueFilter"
            value={opt}
            checked={filterViewState.config.dueFilter.type === opt}
            onchange={() => updateDue({ type: opt })}
          />
          {opt === "any" ? "Any time" : opt === "overdue" ? "Overdue" : opt === "today" ? "Today" : opt === "this_week" ? "This week" : opt === "this_month" ? "This month" : "Custom range"}
        </label>
      {/each}
      {#if filterViewState.config.dueFilter.type === "custom"}
        <div class="date-range">
          <input
            type="date"
            class="date-input"
            value={filterViewState.config.dueFilter.from ?? ""}
            onchange={(e) => updateDue({ from: (e.currentTarget as HTMLInputElement).value || undefined })}
          />
          <span>→</span>
          <input
            type="date"
            class="date-input"
            value={filterViewState.config.dueFilter.to ?? ""}
            onchange={(e) => updateDue({ to: (e.currentTarget as HTMLInputElement).value || undefined })}
          />
        </div>
      {/if}
    </section>

    <!-- Scheduled date -->
    <section class="filter-section">
      <h4 class="section-label">Scheduled date</h4>
      {#each (["any", "overdue", "today", "this_week", "this_month", "custom"] as const) as opt}
        <label class="radio-label">
          <input
            type="radio"
            name="scheduledFilter"
            value={opt}
            checked={filterViewState.config.scheduledFilter.type === opt}
            onchange={() => updateScheduled({ type: opt })}
          />
          {opt === "any" ? "Any time" : opt === "overdue" ? "Overdue" : opt === "today" ? "Today" : opt === "this_week" ? "This week" : opt === "this_month" ? "This month" : "Custom range"}
        </label>
      {/each}
      {#if filterViewState.config.scheduledFilter.type === "custom"}
        <div class="date-range">
          <input
            type="date"
            class="date-input"
            value={filterViewState.config.scheduledFilter.from ?? ""}
            onchange={(e) => updateScheduled({ from: (e.currentTarget as HTMLInputElement).value || undefined })}
          />
          <span>→</span>
          <input
            type="date"
            class="date-input"
            value={filterViewState.config.scheduledFilter.to ?? ""}
            onchange={(e) => updateScheduled({ to: (e.currentTarget as HTMLInputElement).value || undefined })}
          />
        </div>
      {/if}
    </section>

    <!-- Priority -->
    <section class="filter-section">
      <h4 class="section-label">Priority</h4>
      <div class="chip-group">
        {#each priorities as priority (priority.id)}
          {@const active = filterViewState.config.priorities.includes(priority.id)}
          <button
            type="button"
            class="chip"
            class:chip-active={active}
            style="--chip-color: {priority.color}"
            onclick={() => togglePriority(priority.id)}
          >
            {priority.label}
          </button>
        {/each}
      </div>
      {#if filterViewState.config.priorities.length > 0}
        <button type="button" class="clear-link" onclick={() => update({ priorities: [] })}>
          Clear
        </button>
      {/if}
    </section>

    <!-- Estimate -->
    <section class="filter-section">
      <h4 class="section-label">Estimate (hours)</h4>
      <div class="range-row">
        <label>
          Min
          <input
            type="number"
            class="number-input"
            min="0"
            step="0.5"
            placeholder="—"
            value={filterViewState.config.estimateMinHours ?? ""}
            oninput={(e) => {
              const v = parseFloat((e.currentTarget as HTMLInputElement).value);
              update({ estimateMinHours: isNaN(v) ? null : v });
            }}
          />
        </label>
        <label>
          Max
          <input
            type="number"
            class="number-input"
            min="0"
            step="0.5"
            placeholder="—"
            value={filterViewState.config.estimateMaxHours ?? ""}
            oninput={(e) => {
              const v = parseFloat((e.currentTarget as HTMLInputElement).value);
              update({ estimateMaxHours: isNaN(v) ? null : v });
            }}
          />
        </label>
      </div>
    </section>

    <!-- Tracked time -->
    <section class="filter-section">
      <h4 class="section-label">Tracked time (hours)</h4>
      <div class="range-row">
        <label>
          Min
          <input
            type="number"
            class="number-input"
            min="0"
            step="0.5"
            placeholder="—"
            value={filterViewState.config.trackedMinHours ?? ""}
            oninput={(e) => {
              const v = parseFloat((e.currentTarget as HTMLInputElement).value);
              update({ trackedMinHours: isNaN(v) ? null : v });
            }}
          />
        </label>
        <label>
          Max
          <input
            type="number"
            class="number-input"
            min="0"
            step="0.5"
            placeholder="—"
            value={filterViewState.config.trackedMaxHours ?? ""}
            oninput={(e) => {
              const v = parseFloat((e.currentTarget as HTMLInputElement).value);
              update({ trackedMaxHours: isNaN(v) ? null : v });
            }}
          />
        </label>
      </div>
    </section>

    <!-- Has subtasks -->
    <section class="filter-section">
      <h4 class="section-label">Has subtasks</h4>
      <div class="radio-row">
        {#each (["any", "yes", "no"] as const) as opt}
          <label>
            <input
              type="radio"
              name="hasSubtasks"
              value={opt}
              checked={filterViewState.config.hasSubtasks === opt}
              onchange={() => update({ hasSubtasks: opt })}
            />
            {opt === "any" ? "Any" : opt === "yes" ? "Yes" : "No"}
          </label>
        {/each}
      </div>
    </section>

    <!-- Is recurring -->
    <section class="filter-section">
      <h4 class="section-label">Recurring</h4>
      <div class="radio-row">
        {#each (["any", "yes", "no"] as const) as opt}
          <label>
            <input
              type="radio"
              name="isRecurring"
              value={opt}
              checked={filterViewState.config.isRecurring === opt}
              onchange={() => update({ isRecurring: opt })}
            />
            {opt === "any" ? "Any" : opt === "yes" ? "Recurring only" : "Non-recurring only"}
          </label>
        {/each}
      </div>
    </section>

    <!-- Is archived -->
    <section class="filter-section">
      <h4 class="section-label">Archive</h4>
      <div class="radio-row">
        {#each (["active_only", "archived_only", "both"] as const) as opt}
          <label>
            <input
              type="radio"
              name="isArchived"
              value={opt}
              checked={filterViewState.config.isArchived === opt}
              onchange={() => update({ isArchived: opt })}
            />
            {opt === "active_only" ? "Active only" : opt === "archived_only" ? "Archived only" : "Both"}
          </label>
        {/each}
      </div>
    </section>
  </div>
</aside>

<style>
  .filter-drawer {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    width: 360px;
    background: var(--color-surface);
    border-left: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
    z-index: 200;
    box-shadow: -4px 0 24px color-mix(in srgb, var(--color-text) 8%, transparent);
    overflow: hidden;
  }

  .drawer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-md) var(--space-lg);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .drawer-title {
    font-size: 0.9375rem;
    font-weight: 600;
    color: var(--color-text);
  }

  .drawer-header-actions {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .reset-btn {
    font-size: 0.8125rem;
    color: var(--color-text-muted);
    background: none;
    border: none;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    transition: color 150ms;
  }

  .reset-btn:hover {
    color: var(--color-text);
    background: var(--color-surface-raised);
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: none;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    color: var(--color-text-muted);
    font-size: 1.25rem;
    line-height: 1;
    cursor: pointer;
    transition: color 150ms, border-color 150ms;
  }

  .close-btn:hover {
    color: var(--color-text);
    border-color: var(--color-text-muted);
  }

  .drawer-body {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-sm) 0;
  }

  .filter-section {
    padding: var(--space-sm) var(--space-lg);
    border-bottom: 1px solid var(--color-border);
  }

  .filter-section:last-child {
    border-bottom: none;
  }

  .section-label {
    font-size: 0.6875rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--color-text-muted);
    margin: 0 0 var(--space-xs) 0;
  }

  .text-input {
    width: 100%;
    padding: var(--space-xs) var(--space-sm);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    font-size: 0.875rem;
    box-sizing: border-box;
  }

  .text-input:focus {
    outline: none;
    border-color: var(--color-accent);
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: 0.8125rem;
    color: var(--color-text-muted);
    margin-top: var(--space-xs);
    cursor: pointer;
    user-select: none;
  }

  .radio-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-sm);
  }

  .radio-row label,
  .radio-label {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 0.8125rem;
    color: var(--color-text-muted);
    cursor: pointer;
    user-select: none;
  }

  .radio-label {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 4px;
  }

  .chip-group {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-xs);
  }

  .chip {
    padding: 2px 10px;
    border-radius: 999px;
    font-size: 0.75rem;
    font-weight: 600;
    cursor: pointer;
    border: 1.5px solid color-mix(in srgb, var(--chip-color) 40%, transparent);
    background: color-mix(in srgb, var(--chip-color) 10%, transparent);
    color: var(--chip-color);
    transition: background 150ms, border-color 150ms;
    user-select: none;
  }

  .chip-active {
    background: color-mix(in srgb, var(--chip-color) 25%, transparent);
    border-color: var(--chip-color);
  }

  .chip:hover {
    background: color-mix(in srgb, var(--chip-color) 20%, transparent);
  }

  .clear-link {
    font-size: 0.75rem;
    color: var(--color-text-muted);
    background: none;
    border: none;
    cursor: pointer;
    padding: 2px 0;
    margin-top: var(--space-xs);
    text-decoration: underline;
    display: block;
  }

  .clear-link:hover {
    color: var(--color-text);
  }

  /* Project tree */
  .project-tree {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .project-row {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: 0.8125rem;
    color: var(--color-text-muted);
    cursor: pointer;
    user-select: none;
    padding: 2px 0;
  }

  .project-row-child {
    padding-left: var(--space-lg);
  }

  .project-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  /* Tags */
  .tag-chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-xs);
    align-items: center;
  }

  .tag-chip {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 2px 8px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--color-accent) 15%, transparent);
    color: var(--color-accent);
    font-size: 0.75rem;
    font-weight: 500;
  }

  .tag-remove {
    background: none;
    border: none;
    cursor: pointer;
    color: inherit;
    opacity: 0.7;
    font-size: 1rem;
    line-height: 1;
    padding: 0 1px;
  }

  .tag-remove:hover {
    opacity: 1;
  }

  .tag-input {
    border: none;
    outline: none;
    background: none;
    font-size: 0.8125rem;
    color: var(--color-text);
    min-width: 80px;
    flex: 1;
  }

  /* Date range */
  .date-range {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    margin-top: var(--space-xs);
  }

  .date-input {
    flex: 1;
    padding: var(--space-xs);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    font-size: 0.8125rem;
  }

  /* Estimate / tracked range */
  .range-row {
    display: flex;
    gap: var(--space-md);
  }

  .range-row label {
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 0.75rem;
    color: var(--color-text-muted);
    flex: 1;
  }

  .number-input {
    padding: var(--space-xs);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    font-size: 0.875rem;
    width: 100%;
  }
</style>
