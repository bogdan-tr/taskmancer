<script lang="ts">
  import type { DashboardDateRange } from "$lib/api";
  import { listStatusLayouts, updateStatusLayout } from "$lib/api";
  import { projectsState } from "$lib/projects.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import type { DashboardWidget, StatLayout } from "$lib/types";
  import { DATE_RANGE_LABELS } from "$lib/widgetCatalog";
  import { GridStack, type GridStackNode } from "gridstack";
  import "gridstack/dist/gridstack.min.css";
  import CompletionOverviewWidget from "./CompletionOverviewWidget.svelte";
  import ProductivityWidget from "./ProductivityWidget.svelte";
  import ProjectHealthWidget from "./ProjectHealthWidget.svelte";
  import ProjectScaleWidget from "./ProjectScaleWidget.svelte";
  import StatusByProjectWidget from "./StatusByProjectWidget.svelte";

  interface Props {
    projectId: string | null;
  }
  let { projectId }: Props = $props();

  // ── Date range ───────────────────────────────────────────────────────────
  let dateRange = $state<DashboardDateRange>("last_30_days");

  // ── Layout resolution ────────────────────────────────────────────────────
  let project = $derived(
    projectId ? projectsState.items.find((p) => p.id === projectId) : undefined,
  );

  let resolvedLayoutId = $derived(
    (project?.board.dashboard_layout_id ??
      settingsState.current?.default_dashboard_layout_id) ?? "",
  );

  let layouts = $state<StatLayout[]>([]);

  $effect(() => {
    listStatusLayouts()
      .then((all) => { layouts = all.filter((l) => l.kind === "dashboard"); })
      .catch(() => {});
  });

  let resolvedLayout = $derived(
    layouts.find((l) => l.id === resolvedLayoutId) ?? layouts[0] ?? null,
  );

  // ── Defaults ─────────────────────────────────────────────────────────────
  const DEFAULT_WIDGETS: DashboardWidget[] = [
    { widget_type: "completion_overview", x: 0, y: 0, w: 6, h: 4 },
    { widget_type: "project_scale",       x: 6, y: 0, w: 6, h: 4 },
    { widget_type: "status_by_project",   x: 0, y: 4, w: 4, h: 4 },
    { widget_type: "project_health",      x: 4, y: 4, w: 3, h: 4, include_subprojects: false },
    { widget_type: "productivity",        x: 7, y: 4, w: 5, h: 4 },
  ];

  const WIDGET_LABELS: Partial<Record<DashboardWidget["widget_type"], string>> = {
    completion_overview: "Completion Overview",
    project_scale:       "Project Time & Scale",
    status_by_project:   "Status by Project",
    project_health:      "Project Health",
    productivity:        "Productivity",
  };

  const WIDGET_DEFAULTS: Partial<Record<
    DashboardWidget["widget_type"],
    Omit<DashboardWidget, "widget_type">
  >> = {
    completion_overview: { x: 0, y: 100, w: 6, h: 4 },
    project_scale:       { x: 6, y: 100, w: 6, h: 4 },
    status_by_project:   { x: 0, y: 100, w: 4, h: 4 },
    project_health:      { x: 4, y: 100, w: 3, h: 4, include_subprojects: false },
    productivity:        { x: 7, y: 100, w: 5, h: 4 },
  };

  const ALL_WIDGET_TYPES: DashboardWidget["widget_type"][] = [
    "completion_overview",
    "project_scale",
    "status_by_project",
    "project_health",
    "productivity",
  ];

  // ── View-mode derived values ─────────────────────────────────────────────
  let widgets = $derived(resolvedLayout?.dashboard_widgets ?? DEFAULT_WIDGETS);

  function gridStyle(w: DashboardWidget): string {
    return [
      `grid-column: ${w.x + 1} / span ${w.w}`,
      `grid-row: ${w.y + 1} / span ${w.h}`,
    ].join("; ");
  }

  // ── Edit mode state ──────────────────────────────────────────────────────
  let editMode = $state(false);
  let localWidgetTypes = $state<DashboardWidget["widget_type"][]>([]);
  let localTheme = $state<"dark" | "app" | "glass" | "aurora">("dark");
  let healthSubprojectsDraft = $state(false);
  let isSaving = $state(false);
  let saveError = $state<string | null>(null);

  // Non-reactive position store — updated by gridstack events, no Svelte re-renders
  const positions = new Map<
    DashboardWidget["widget_type"],
    { x: number; y: number; w: number; h: number; include_subprojects?: boolean }
  >();

  function enterEditMode() {
    const src = resolvedLayout?.dashboard_widgets ?? DEFAULT_WIDGETS;
    localWidgetTypes = src.map((w) => w.widget_type);
    localTheme = resolvedLayout?.dashboard_theme ?? "dark";
    positions.clear();
    for (const w of src) {
      positions.set(w.widget_type, {
        x: w.x, y: w.y, w: w.w, h: w.h,
        include_subprojects: w.include_subprojects,
      });
    }
    healthSubprojectsDraft =
      src.find((w) => w.widget_type === "project_health")?.include_subprojects ?? false;
    editMode = true;
  }

  function cancelEdit() {
    if (grid) { grid.destroy(false); grid = null; }
    editMode = false;
  }

  async function saveLayout() {
    if (isSaving) return;
    isSaving = true;
    saveError = null;

    // Read final positions from gridstack DOM attributes — reliable after any drag/resize
    if (grid) {
      for (const el of grid.getGridItems()) {
        const type = el.dataset.widgetType as DashboardWidget["widget_type"] | undefined;
        if (!type) continue;
        const prev = positions.get(type) ?? { x: 0, y: 0, w: 4, h: 4 };
        const x = parseInt(el.getAttribute("gs-x") ?? "");
        const y = parseInt(el.getAttribute("gs-y") ?? "");
        const w = parseInt(el.getAttribute("gs-w") ?? "");
        const h = parseInt(el.getAttribute("gs-h") ?? "");
        positions.set(type, {
          ...prev,
          x: isNaN(x) ? prev.x : x,
          y: isNaN(y) ? prev.y : y,
          w: isNaN(w) ? prev.w : w,
          h: isNaN(h) ? prev.h : h,
        });
      }
    }

    // Sync health subprojects toggle into positions
    const hPos = positions.get("project_health");
    if (hPos) positions.set("project_health", { ...hPos, include_subprojects: healthSubprojectsDraft });

    const newWidgets: DashboardWidget[] = localWidgetTypes.map((type) => {
      const pos = positions.get(type) ?? WIDGET_DEFAULTS[type] ?? { x: 0, y: 100, w: 4, h: 4 };
      return { widget_type: type, ...pos };
    });

    const baseLayout = resolvedLayout ?? layouts[0] ?? null;
    if (!baseLayout) {
      saveError = "No dashboard layout found.";
      isSaving = false;
      return;
    }

    const updated: StatLayout = {
      ...baseLayout,
      dashboard_widgets: newWidgets,
      dashboard_theme: localTheme,
    };
    try {
      await updateStatusLayout(updated);
      const all = await listStatusLayouts();
      layouts = all.filter((l) => l.kind === "dashboard");
      if (grid) { grid.destroy(false); grid = null; }
      editMode = false;
    } catch (e) {
      saveError = e instanceof Error ? e.message : String(e) || "Save failed — please try again.";
    } finally {
      isSaving = false;
    }
  }

  function addWidget(wt: DashboardWidget["widget_type"]) {
    if (localWidgetTypes.includes(wt)) return;
    const def = WIDGET_DEFAULTS[wt] ?? { x: 0, y: 100, w: 4, h: 4 };
    positions.set(wt, { x: def.x, y: def.y, w: def.w, h: def.h, include_subprojects: (def as { include_subprojects?: boolean }).include_subprojects });
    if (wt === "project_health") healthSubprojectsDraft = (def as { include_subprojects?: boolean }).include_subprojects ?? false;
    localWidgetTypes = [...localWidgetTypes, wt]; // triggers re-init via editKey
  }

  function removeWidget(wt: DashboardWidget["widget_type"]) {
    positions.delete(wt);
    localWidgetTypes = localWidgetTypes.filter((t) => t !== wt); // triggers re-init
  }

  function toggleHealthSub() {
    healthSubprojectsDraft = !healthSubprojectsDraft;
    const pos = positions.get("project_health");
    if (pos) positions.set("project_health", { ...pos, include_subprojects: healthSubprojectsDraft });
  }

  // ── Theme: optimistic in edit mode ───────────────────────────────────────
  let theme = $derived(editMode ? localTheme : (resolvedLayout?.dashboard_theme ?? "dark"));

  // ── GridStack ────────────────────────────────────────────────────────────
  let grid: GridStack | null = null;
  let gridEl = $state<HTMLElement | null>(null);

  // Re-init gridstack only when widget types change (add/remove), NOT on position changes
  let editKey = $derived(editMode ? localWidgetTypes.join(",") : "");

  $effect(() => {
    const _key = editKey; // track dependency
    if (!editMode || !gridEl || !_key) return;

    const g = GridStack.init(
      { column: 12, cellHeight: 80, margin: 8, float: false, animate: false },
      gridEl,
    );
    grid = g;

    g.on("change", (_event: Event, items: GridStackNode | GridStackNode[] | boolean) => {
      if (!Array.isArray(items)) return;
      for (const item of items) {
        const el = item.el as HTMLElement | undefined;
        const type = el?.dataset?.widgetType as DashboardWidget["widget_type"] | undefined;
        if (!type) continue;
        const prev = positions.get(type) ?? { x: 0, y: 0, w: 4, h: 4 };
        positions.set(type, {
          ...prev,
          x: item.x ?? prev.x,
          y: item.y ?? prev.y,
          w: item.w ?? prev.w,
          h: item.h ?? prev.h,
        });
      }
    });

    return () => {
      g.destroy(false);
      grid = null;
    };
  });
</script>

<div class="dashboard-shell {`theme-${theme}`}">
  <!-- ── Top bar ──────────────────────────────────────────────────────────── -->
  <div class="top-bar">
    <h1 class="dash-title">{projectId ? "Project Dashboard" : "Dashboard"}</h1>
    <div class="controls">
      {#if !editMode}
        <label class="range-label" for="db-date-range">Date range</label>
        <select
          id="db-date-range"
          class="range-select"
          bind:value={dateRange}
          onchange={(e) => e.currentTarget.blur()}
        >
          {#each Object.entries(DATE_RANGE_LABELS) as [value, label] (value)}
            <option {value}>{label}</option>
          {/each}
        </select>
      {/if}
      {#if editMode}
        <button class="edit-btn cancel-btn" type="button" onclick={cancelEdit}>
          Cancel
        </button>
      {:else}
        <button class="edit-btn" type="button" onclick={enterEditMode}>
          Edit Layout
        </button>
      {/if}
    </div>
  </div>

  <!-- ── Main area ────────────────────────────────────────────────────────── -->
  <div class="main-area" class:with-sidebar={editMode}>

    <!-- ── View mode: CSS grid ──────────────────────────────────────────── -->
    {#if !editMode}
      <div class="view-grid">
        {#each widgets as w (w.widget_type)}
          <div class="widget-card" style={gridStyle(w)}>
            {#if w.widget_type === "completion_overview"}
              <CompletionOverviewWidget {dateRange} />
            {:else if w.widget_type === "project_scale"}
              <ProjectScaleWidget {dateRange} />
            {:else if w.widget_type === "status_by_project"}
              <StatusByProjectWidget {dateRange} />
            {:else if w.widget_type === "project_health"}
              <ProjectHealthWidget includeSubprojects={w.include_subprojects ?? false} />
            {:else if w.widget_type === "productivity"}
              <ProductivityWidget {dateRange} />
            {/if}
          </div>
        {/each}
        {#if widgets.length === 0}
          <p class="empty-msg">No widgets configured. Click "Edit Layout" to add widgets.</p>
        {/if}
      </div>

    <!-- ── Edit mode: GridStack ──────────────────────────────────────────── -->
    {:else}
      <div class="gs-grid-wrapper">
        <div class="grid-stack" bind:this={gridEl}>
          {#each localWidgetTypes as wt (wt)}
            {@const pos = positions.get(wt) ?? WIDGET_DEFAULTS[wt] ?? { x: 0, y: 0, w: 4, h: 4 }}
            <div
              class="grid-stack-item"
              {...{ "gs-x": pos.x, "gs-y": pos.y, "gs-w": pos.w, "gs-h": pos.h }}
              data-widget-type={wt}
            >
              <div class="grid-stack-item-content widget-card edit-card">
                <div class="edit-handle">
                  <span class="handle-icon" aria-hidden="true">⠿</span>
                  <span class="handle-label">{WIDGET_LABELS[wt] ?? wt}</span>
                  {#if wt === "project_health"}
                    <button
                      class="sub-toggle"
                      type="button"
                      onclick={toggleHealthSub}
                      title="Toggle subproject inclusion"
                    >
                      Sub: {healthSubprojectsDraft ? "On" : "Off"}
                    </button>
                  {/if}
                  <button
                    class="remove-btn"
                    type="button"
                    onclick={() => removeWidget(wt)}
                    title="Remove widget"
                    aria-label="Remove {WIDGET_LABELS[wt] ?? wt}"
                  >✕</button>
                </div>
                <div class="edit-preview">
                  {#if wt === "completion_overview"}
                    <CompletionOverviewWidget {dateRange} />
                  {:else if wt === "project_scale"}
                    <ProjectScaleWidget {dateRange} />
                  {:else if wt === "status_by_project"}
                    <StatusByProjectWidget {dateRange} />
                  {:else if wt === "project_health"}
                    <ProjectHealthWidget includeSubprojects={healthSubprojectsDraft} />
                  {:else if wt === "productivity"}
                    <ProductivityWidget {dateRange} />
                  {/if}
                </div>
              </div>
            </div>
          {/each}
        </div>
      </div>

      <!-- ── Sidebar ───────────────────────────────────────────────────── -->
      <aside class="edit-sidebar">
        <!-- Theme switcher -->
        <section class="sidebar-section">
          <h3 class="sidebar-heading">Theme</h3>
          <div class="theme-buttons">
            {#each (["dark", "app", "glass", "aurora"] as const) as t (t)}
              <button
                class="theme-btn"
                class:active={localTheme === t}
                type="button"
                onclick={() => { localTheme = t; }}
              >
                {t.charAt(0).toUpperCase() + t.slice(1)}
              </button>
            {/each}
          </div>
        </section>

        <!-- Widget library -->
        <section class="sidebar-section">
          <h3 class="sidebar-heading">Widgets</h3>
          <div class="widget-library">
            {#each ALL_WIDGET_TYPES as wt (wt)}
              {@const isAdded = localWidgetTypes.includes(wt)}
              <div class="widget-lib-card" class:is-added={isAdded}>
                <span class="lib-name">{WIDGET_LABELS[wt] ?? wt}</span>
                {#if isAdded}
                  <button
                    class="remove-lib-btn"
                    type="button"
                    onclick={() => removeWidget(wt)}
                  >Remove</button>
                {:else}
                  <button
                    class="add-btn"
                    type="button"
                    onclick={() => addWidget(wt)}
                  >+ Add</button>
                {/if}
              </div>
            {/each}
          </div>
        </section>

        <!-- Save -->
        <div class="sidebar-actions">
          {#if saveError}
            <p class="save-error">{saveError}</p>
          {/if}
          <button
            class="save-btn"
            type="button"
            onclick={saveLayout}
            disabled={isSaving}
          >
            {isSaving ? "Saving…" : "Save Layout"}
          </button>
        </div>
      </aside>
    {/if}
  </div>
</div>

<style>
  /* ── Theme tokens ──────────────────────────────────────────────────────── */
  .theme-dark {
    --db-bg: #0d1117;
    --db-card: #161b22;
    --db-border: rgba(255, 255, 255, 0.08);
    --db-ink: #e6edf3;
    --db-ink-muted: #8b949e;
    --db-accent: #58a6ff;
    --db-grid-line: rgba(255, 255, 255, 0.04);
  }

  .theme-app {
    --db-bg: var(--color-canvas);
    --db-card: var(--color-surface);
    --db-border: var(--color-border);
    --db-ink: var(--color-ink);
    --db-ink-muted: var(--color-ink-muted);
    --db-accent: var(--color-accent);
    --db-grid-line: var(--color-border);
  }

  /* Glass: aurora-style but all deep-blue tones */
  .theme-glass {
    --db-bg: #000000;
    --db-card: rgba(0, 10, 30, 0.55);
    --db-border: rgba(56, 189, 248, 0.18);
    --db-ink: #e0f2fe;
    --db-ink-muted: rgba(186, 230, 253, 0.55);
    --db-accent: #38bdf8;
    --db-grid-line: rgba(56, 189, 248, 0.06);
  }

  /* Aurora: true northern lights — blue, green, purple on black */
  .theme-aurora {
    --db-bg: #000000;
    --db-card: rgba(0, 10, 5, 0.60);
    --db-border: rgba(52, 211, 153, 0.18);
    --db-ink: #f0fdf4;
    --db-ink-muted: rgba(167, 243, 208, 0.55);
    --db-accent: #34d399;
    --db-grid-line: rgba(52, 211, 153, 0.06);
  }

  /* ── Shell ─────────────────────────────────────────────────────────────── */
  .dashboard-shell {
    min-height: 100%;
    background: var(--db-bg);
    color: var(--db-ink);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  /* Glass: deep-blue aurora radials, frosted cards */
  .theme-glass {
    background: #000000;
    position: relative;
    overflow: hidden;
  }

  .theme-glass::before {
    content: "";
    position: absolute;
    inset: 0;
    background:
      radial-gradient(ellipse 70% 45% at 15% 25%, rgba(29, 78, 216, 0.32) 0%, transparent 70%),
      radial-gradient(ellipse 60% 50% at 78% 18%, rgba(99, 102, 241, 0.26) 0%, transparent 65%),
      radial-gradient(ellipse 50% 38% at 50% 72%, rgba(14, 165, 233, 0.20) 0%, transparent 60%),
      radial-gradient(ellipse 75% 28% at 10% 82%, rgba(56, 189, 248, 0.16) 0%, transparent 55%);
    pointer-events: none;
    z-index: 0;
  }

  .theme-glass > * {
    position: relative;
    z-index: 1;
  }

  .theme-glass .widget-card {
    backdrop-filter: blur(14px);
    -webkit-backdrop-filter: blur(14px);
    border: 1px solid rgba(56, 189, 248, 0.18);
    box-shadow: 0 0 0 1px rgba(56, 189, 248, 0.06) inset;
  }

  /* Aurora: true northern lights — three-color patches on black */
  .theme-aurora {
    background: #000000;
    position: relative;
    overflow: hidden;
  }

  .theme-aurora::before {
    content: "";
    position: absolute;
    inset: 0;
    background:
      radial-gradient(ellipse 65% 50% at 12% 22%, rgba(29, 78, 216, 0.34) 0%, transparent 68%),
      radial-gradient(ellipse 55% 55% at 72% 14%, rgba(6, 95, 70, 0.38) 0%, transparent 65%),
      radial-gradient(ellipse 60% 42% at 48% 78%, rgba(109, 40, 217, 0.32) 0%, transparent 62%),
      radial-gradient(ellipse 42% 32% at 88% 58%, rgba(16, 185, 129, 0.24) 0%, transparent 55%),
      radial-gradient(ellipse 68% 28% at 28% 88%, rgba(79, 70, 229, 0.20) 0%, transparent 52%);
    pointer-events: none;
    z-index: 0;
  }

  .theme-aurora > * {
    position: relative;
    z-index: 1;
  }

  .theme-aurora .widget-card {
    backdrop-filter: blur(14px);
    -webkit-backdrop-filter: blur(14px);
    border: 1px solid rgba(52, 211, 153, 0.16);
    box-shadow: 0 0 0 1px rgba(52, 211, 153, 0.05) inset;
  }

  /* ── Top bar ───────────────────────────────────────────────────────────── */
  .top-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px 12px;
    border-bottom: 1px solid var(--db-border);
    gap: 12px;
    flex-wrap: wrap;
    flex-shrink: 0;
  }

  .dash-title {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
    color: var(--db-ink);
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .range-label {
    font-size: 12px;
    font-weight: 600;
    color: var(--db-ink-muted);
  }

  .range-select {
    padding: 4px 8px;
    border-radius: 6px;
    border: 1px solid var(--db-border);
    background: var(--db-card);
    color: var(--db-ink);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }

  .range-select:focus-visible {
    outline: 2px solid var(--db-accent);
    outline-offset: 1px;
  }

  .edit-btn {
    padding: 5px 16px;
    border-radius: 6px;
    border: 1px solid var(--db-border);
    background: transparent;
    color: var(--db-ink-muted);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
    transition: background 150ms, color 150ms, border-color 150ms;
  }

  .edit-btn:hover {
    background: var(--db-card);
    color: var(--db-ink);
    border-color: var(--db-accent);
  }

  .cancel-btn {
    color: var(--db-ink-muted);
    border-color: rgba(255, 255, 255, 0.12);
  }

  .cancel-btn:hover {
    background: rgba(255, 255, 255, 0.06);
    color: var(--db-ink);
    border-color: var(--db-border);
  }

  /* ── Main area ─────────────────────────────────────────────────────────── */
  .main-area {
    flex: 1;
    overflow: auto;
    padding: 16px;
    display: flex;
    gap: 0;
  }

  .main-area.with-sidebar {
    gap: 16px;
  }

  /* ── View mode grid ────────────────────────────────────────────────────── */
  .view-grid {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(12, 1fr);
    grid-auto-rows: 80px;
    gap: 8px;
    align-content: start;
  }

  .widget-card {
    background: var(--db-card);
    border: 1px solid var(--db-border);
    border-radius: 12px;
    padding: 10px 12px;
    overflow: hidden;
    box-shadow: 0 1px 8px rgba(0, 0, 0, 0.18), inset 0 1px 0 rgba(255, 255, 255, 0.05);
  }

  .empty-msg {
    grid-column: 1 / -1;
    text-align: center;
    color: var(--db-ink-muted);
    font-size: 13px;
    padding: 48px;
  }

  /* ── GridStack wrapper ─────────────────────────────────────────────────── */
  .gs-grid-wrapper {
    flex: 1;
    min-width: 0;
    overflow-x: auto;
  }

  .gs-grid-wrapper :global(.grid-stack) {
    background: transparent;
  }

  .gs-grid-wrapper :global(.grid-stack-item-content) {
    background: transparent;
    border-radius: 0;
    inset: 0;
    overflow: visible;
  }

  /* ── Edit card ─────────────────────────────────────────────────────────── */
  .edit-card {
    display: flex;
    flex-direction: column;
    gap: 0;
    padding: 0;
    overflow: hidden;
    height: 100%;
  }

  .edit-handle {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    background: rgba(0, 0, 0, 0.25);
    border-bottom: 1px solid var(--db-border);
    cursor: grab;
    flex-shrink: 0;
    min-height: 32px;
  }

  .handle-icon {
    color: var(--db-ink-muted);
    font-size: 14px;
    user-select: none;
  }

  .handle-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--db-ink-muted);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sub-toggle {
    padding: 2px 8px;
    border-radius: 999px;
    border: 1px solid var(--db-border);
    background: transparent;
    color: var(--db-ink-muted);
    font-size: 10px;
    cursor: pointer;
    flex-shrink: 0;
    transition: background 150ms;
  }

  .sub-toggle:hover {
    background: var(--db-card);
    color: var(--db-ink);
  }

  .remove-btn {
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--db-ink-muted);
    font-size: 11px;
    cursor: pointer;
    flex-shrink: 0;
    transition: background 150ms, color 150ms;
    line-height: 1;
  }

  .remove-btn:hover {
    background: rgba(239, 68, 68, 0.15);
    color: #ef4444;
    border-color: rgba(239, 68, 68, 0.3);
  }

  .edit-preview {
    flex: 1;
    padding: 10px 12px;
    overflow: hidden;
    min-height: 0;
  }

  /* ── Sidebar ───────────────────────────────────────────────────────────── */
  .edit-sidebar {
    width: 260px;
    flex-shrink: 0;
    background: var(--db-card);
    border: 1px solid var(--db-border);
    border-radius: 12px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    align-self: flex-start;
    position: sticky;
    top: 0;
    overflow-y: auto;
    max-height: calc(100vh - 120px);
  }

  .sidebar-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .sidebar-heading {
    margin: 0;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--db-ink-muted);
  }

  /* ── Theme switcher ────────────────────────────────────────────────────── */
  .theme-buttons {
    display: flex;
    gap: 6px;
  }

  .theme-btn {
    flex: 1;
    padding: 6px 0;
    border-radius: 6px;
    border: 1px solid var(--db-border);
    background: transparent;
    color: var(--db-ink-muted);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
    transition: background 150ms, color 150ms, border-color 150ms;
  }

  .theme-btn:hover {
    background: rgba(255, 255, 255, 0.06);
    color: var(--db-ink);
  }

  .theme-btn.active {
    background: var(--db-accent);
    color: #fff;
    border-color: var(--db-accent);
  }

  /* ── Widget library ────────────────────────────────────────────────────── */
  .widget-library {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .widget-lib-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 8px;
    border: 1px solid var(--db-border);
    background: rgba(255, 255, 255, 0.03);
  }

  .lib-name {
    font-size: 12px;
    color: var(--db-ink);
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .add-btn {
    padding: 3px 10px;
    border-radius: 6px;
    border: 1px solid var(--db-accent);
    background: transparent;
    color: var(--db-accent);
    font: inherit;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    flex-shrink: 0;
    transition: background 150ms;
    white-space: nowrap;
  }

  .add-btn:hover {
    background: var(--db-accent);
    color: #fff;
  }

  .remove-lib-btn {
    padding: 3px 10px;
    border-radius: 6px;
    border: 1px solid rgba(239, 68, 68, 0.35);
    background: transparent;
    color: #ef4444;
    font: inherit;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    flex-shrink: 0;
    transition: background 150ms;
    white-space: nowrap;
  }

  .remove-lib-btn:hover {
    background: rgba(239, 68, 68, 0.15);
  }

  /* ── Save button ───────────────────────────────────────────────────────── */
  .sidebar-actions {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .save-error {
    margin: 0;
    padding: 8px 10px;
    border-radius: 6px;
    background: rgba(239, 68, 68, 0.12);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: #ef4444;
    font-size: 12px;
    line-height: 1.4;
    word-break: break-word;
  }

  .save-btn {
    width: 100%;
    padding: 9px 0;
    border-radius: 8px;
    border: none;
    background: var(--db-accent);
    color: #fff;
    font: inherit;
    font-size: 14px;
    font-weight: 700;
    cursor: pointer;
    transition: opacity 150ms, filter 150ms;
  }

  .save-btn:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .save-btn:disabled {
    opacity: 0.55;
    cursor: wait;
  }
</style>
