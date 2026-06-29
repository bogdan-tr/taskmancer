<script lang="ts">
  import { getProjectDashboardLayout, saveProjectDashboardLayout } from "$lib/api";
  import type { DashboardWidget, StatLayout } from "$lib/types";
  import { GridStack, type GridStackNode } from "gridstack";
  import "gridstack/dist/gridstack.min.css";
  import W1ScoreboardWidget from "./W1ScoreboardWidget.svelte";
  import W2HealthPulseWidget from "./W2HealthPulseWidget.svelte";
  import W3VelocityWidget from "./W3VelocityWidget.svelte";
  import W4CompletionDialWidget from "./W4CompletionDialWidget.svelte";
  import W5FuelGaugeWidget from "./W5FuelGaugeWidget.svelte";
  import W6EffortBalanceWidget from "./W6EffortBalanceWidget.svelte";
  import W7WeeklyRhythmWidget from "./W7WeeklyRhythmWidget.svelte";
  import W9TimeBreakdownWidget from "./W9TimeBreakdownWidget.svelte";
  import W10StatusRadialWidget from "./W10StatusRadialWidget.svelte";
  import W12DueTimelineWidget from "./W12DueTimelineWidget.svelte";
  import W13BurndownWidget from "./W13BurndownWidget.svelte";
  import W14CompletionTrendWidget from "./W14CompletionTrendWidget.svelte";
  import W16SubprojectTreeWidget from "./W16SubprojectTreeWidget.svelte";
  import W17SubprojectBarsWidget from "./W17SubprojectBarsWidget.svelte";
  import W18SubprojectSunburstWidget from "./W18SubprojectSunburstWidget.svelte";

  interface Props {
    projectId: string;
    projectColor: string;
    projectName: string;
  }
  let { projectId, projectColor, projectName }: Props = $props();

  // ── Layout load ─────────────────────────────────────────────────────────
  let layout = $state<StatLayout | null>(null);

  $effect(() => {
    if (!projectId) return;
    layout = null;
    getProjectDashboardLayout(projectId)
      .then((l) => { layout = l; })
      .catch(() => {});
  });

  // ── Defaults ─────────────────────────────────────────────────────────────
  const DEFAULT_PROJECT_WIDGETS: DashboardWidget[] = [
    { widget_type: "p_scoreboard",      x: 0, y: 0, w: 6, h: 3 },
    { widget_type: "p_health_pulse",    x: 6, y: 0, w: 3, h: 3 },
    { widget_type: "p_completion_dial", x: 9, y: 0, w: 3, h: 3 },
    { widget_type: "p_velocity",        x: 0, y: 3, w: 5, h: 3 },
  ];

  const WIDGET_LABELS: Partial<Record<DashboardWidget["widget_type"], string>> = {
    p_scoreboard:         "Scoreboard",
    p_health_pulse:       "Health Pulse",
    p_velocity:           "Velocity",
    p_completion_dial:    "Completion Dial",
    p_fuel_gauge:         "Fuel Gauge",
    p_effort_balance:     "Effort Balance",
    p_weekly_rhythm:      "Weekly Rhythm",
    p_time_donut:         "Time Breakdown",
    p_status_radial:      "Status Radial",
    p_due_timeline:       "Due Timeline",
    p_burndown:           "Burndown",
    p_completion_trend:   "Completion Trend",
    p_subproject_tree:    "Subproject Tree",
    p_subproject_bars:    "Subproject Bars",
    p_subproject_sunburst:"Subproject Sunburst",
  };

  type ProjectWidgetPos = { x: number; y: number; w: number; h: number };
  const PROJECT_WIDGET_DEFAULTS: Partial<Record<DashboardWidget["widget_type"], ProjectWidgetPos>> = {
    p_scoreboard:          { x: 0, y: 100, w: 6, h: 3 },
    p_health_pulse:        { x: 0, y: 100, w: 3, h: 3 },
    p_velocity:            { x: 0, y: 100, w: 4, h: 3 },
    p_completion_dial:     { x: 0, y: 100, w: 4, h: 3 },
    p_fuel_gauge:          { x: 0, y: 100, w: 2, h: 4 },
    p_effort_balance:      { x: 0, y: 100, w: 5, h: 3 },
    p_weekly_rhythm:       { x: 0, y: 100, w: 6, h: 4 },
    p_time_donut:          { x: 0, y: 100, w: 4, h: 4 },
    p_status_radial:       { x: 0, y: 100, w: 4, h: 4 },
    p_due_timeline:        { x: 0, y: 100, w: 8, h: 3 },
    p_burndown:            { x: 0, y: 100, w: 8, h: 4 },
    p_completion_trend:    { x: 0, y: 100, w: 6, h: 4 },
    p_subproject_tree:     { x: 0, y: 100, w: 6, h: 6 },
    p_subproject_bars:     { x: 0, y: 100, w: 4, h: 5 },
    p_subproject_sunburst: { x: 0, y: 100, w: 4, h: 4 },
  };

  const ALL_PROJECT_WIDGET_TYPES: DashboardWidget["widget_type"][] = [
    "p_scoreboard",
    "p_health_pulse",
    "p_velocity",
    "p_completion_dial",
    "p_fuel_gauge",
    "p_effort_balance",
    "p_weekly_rhythm",
    "p_time_donut",
    "p_status_radial",
    "p_due_timeline",
    "p_burndown",
    "p_completion_trend",
    "p_subproject_tree",
    "p_subproject_bars",
    "p_subproject_sunburst",
  ];

  // ── View-mode derived values ─────────────────────────────────────────────
  let widgets = $derived(layout?.dashboard_widgets ?? DEFAULT_PROJECT_WIDGETS);

  function gridStyle(w: DashboardWidget): string {
    return [
      `grid-column: ${w.x + 1} / span ${w.w}`,
      `grid-row: ${w.y + 1} / span ${w.h}`,
    ].join("; ");
  }

  function viewConfig(wt: DashboardWidget["widget_type"]): Record<string, unknown> | undefined {
    return widgets.find((w) => w.widget_type === wt)?.config as Record<string, unknown> | undefined;
  }

  // ── Edit mode state ──────────────────────────────────────────────────────
  let editMode = $state(false);
  let localWidgetTypes = $state<DashboardWidget["widget_type"][]>([]);
  let localTheme = $state<"dark" | "app" | "glass" | "aurora">("dark");
  // Configs keyed by widget_type — tracked as plain object so reassignment is reactive.
  let widgetConfigs = $state<Record<string, Record<string, unknown>>>({});
  let isSaving = $state(false);
  let saveError = $state<string | null>(null);

  // Non-reactive position store — gridstack events write here; Svelte never reads it directly.
  const positions = new Map<DashboardWidget["widget_type"], ProjectWidgetPos>();

  function enterEditMode() {
    const src = layout?.dashboard_widgets ?? DEFAULT_PROJECT_WIDGETS;
    localWidgetTypes = src.map((w) => w.widget_type);
    localTheme = layout?.dashboard_theme ?? "dark";
    positions.clear();
    const cfgs: Record<string, Record<string, unknown>> = {};
    for (const w of src) {
      positions.set(w.widget_type, { x: w.x, y: w.y, w: w.w, h: w.h });
      if (w.config) cfgs[w.widget_type] = w.config as Record<string, unknown>;
    }
    widgetConfigs = cfgs;
    saveError = null;
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

    if (grid) {
      for (const el of grid.getGridItems()) {
        const type = el.dataset.widgetType as DashboardWidget["widget_type"] | undefined;
        if (!type) continue;
        const prev = positions.get(type) ?? { x: 0, y: 0, w: 4, h: 3 };
        const x = parseInt(el.getAttribute("gs-x") ?? "");
        const y = parseInt(el.getAttribute("gs-y") ?? "");
        const w = parseInt(el.getAttribute("gs-w") ?? "");
        const h = parseInt(el.getAttribute("gs-h") ?? "");
        positions.set(type, {
          x: isNaN(x) ? prev.x : x,
          y: isNaN(y) ? prev.y : y,
          w: isNaN(w) ? prev.w : w,
          h: isNaN(h) ? prev.h : h,
        });
      }
    }

    const newWidgets: DashboardWidget[] = localWidgetTypes.map((type) => {
      const pos = positions.get(type) ?? PROJECT_WIDGET_DEFAULTS[type] ?? { x: 0, y: 100, w: 4, h: 3 };
      const cfg = widgetConfigs[type];
      return {
        widget_type: type,
        ...pos,
        ...(cfg ? { config: cfg } : {}),
      };
    });

    const toSave: StatLayout = layout
      ? { ...layout, dashboard_widgets: newWidgets, dashboard_theme: localTheme }
      : {
          id: crypto.randomUUID(),
          name: `${projectName} Dashboard`,
          kind: "project_dashboard",
          project_id: projectId,
          stat_ids: [],
          dashboard_widgets: newWidgets,
          dashboard_theme: localTheme,
        };

    try {
      await saveProjectDashboardLayout(toSave);
      layout = toSave;
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
    const def = PROJECT_WIDGET_DEFAULTS[wt] ?? { x: 0, y: 100, w: 4, h: 3 };
    positions.set(wt, def);
    localWidgetTypes = [...localWidgetTypes, wt];
  }

  function removeWidget(wt: DashboardWidget["widget_type"]) {
    positions.delete(wt);
    const { [wt]: _removed, ...rest } = widgetConfigs;
    widgetConfigs = rest;
    localWidgetTypes = localWidgetTypes.filter((t) => t !== wt);
  }

  function handleWidgetConfigChange(wt: DashboardWidget["widget_type"], cfg: Record<string, unknown>) {
    widgetConfigs = { ...widgetConfigs, [wt]: cfg };
  }

  // ── Theme: optimistic in edit mode ───────────────────────────────────────
  let theme = $derived(editMode ? localTheme : (layout?.dashboard_theme ?? "dark"));

  // ── GridStack ────────────────────────────────────────────────────────────
  let grid: GridStack | null = null;
  let gridEl = $state<HTMLElement | null>(null);

  let editKey = $derived(editMode ? localWidgetTypes.join(",") : "");

  $effect(() => {
    const _key = editKey;
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
        const prev = positions.get(type) ?? { x: 0, y: 0, w: 4, h: 3 };
        positions.set(type, {
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

<div
  class="dashboard-shell {`theme-${theme}`}"
  style="--project-accent: {projectColor}"
>
  <!-- ── Top bar ──────────────────────────────────────────────────────────── -->
  <div class="top-bar">
    <h1 class="dash-title">Dashboard</h1>
    <div class="controls">
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
            {#if w.widget_type === "p_scoreboard"}
              <W1ScoreboardWidget {projectId} {projectColor} />
            {:else if w.widget_type === "p_health_pulse"}
              <W2HealthPulseWidget
                {projectId}
                {projectColor}
                config={w.config as { style?: "static" | "ecg" | "pulse" } | undefined}
              />
            {:else if w.widget_type === "p_velocity"}
              <W3VelocityWidget {projectId} {projectColor} />
            {:else if w.widget_type === "p_completion_dial"}
              <W4CompletionDialWidget {projectId} {projectColor} />
            {:else if w.widget_type === "p_fuel_gauge"}
              <W5FuelGaugeWidget {projectId} {projectColor} />
            {:else if w.widget_type === "p_effort_balance"}
              <W6EffortBalanceWidget {projectId} {projectColor} />
            {:else if w.widget_type === "p_weekly_rhythm"}
              <W7WeeklyRhythmWidget {projectId} {projectColor} />
            {:else if w.widget_type === "p_time_donut"}
              <W9TimeBreakdownWidget {projectId} {projectColor} />
            {:else if w.widget_type === "p_status_radial"}
              <W10StatusRadialWidget {projectId} {projectColor} />
            {:else if w.widget_type === "p_due_timeline"}
              <W12DueTimelineWidget {projectId} {projectColor} />
            {:else if w.widget_type === "p_burndown"}
              <W13BurndownWidget {projectId} {projectColor} />
            {:else if w.widget_type === "p_completion_trend"}
              <W14CompletionTrendWidget {projectId} {projectColor} />
            {:else if w.widget_type === "p_subproject_tree"}
              <W16SubprojectTreeWidget {projectId} {projectColor} />
            {:else if w.widget_type === "p_subproject_bars"}
              <W17SubprojectBarsWidget {projectId} {projectColor} />
            {:else if w.widget_type === "p_subproject_sunburst"}
              <W18SubprojectSunburstWidget
                {projectId}
                {projectColor}
                config={w.config as Record<string, unknown> | undefined}
              />
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
            {@const pos = positions.get(wt) ?? PROJECT_WIDGET_DEFAULTS[wt] ?? { x: 0, y: 0, w: 4, h: 3 }}
            <div
              class="grid-stack-item"
              {...{ "gs-x": pos.x, "gs-y": pos.y, "gs-w": pos.w, "gs-h": pos.h }}
              data-widget-type={wt}
            >
              <div class="grid-stack-item-content widget-card edit-card">
                <div class="edit-handle">
                  <span class="handle-icon" aria-hidden="true">⠿</span>
                  <span class="handle-label">{WIDGET_LABELS[wt] ?? wt}</span>
                  <button
                    class="remove-btn"
                    type="button"
                    onclick={() => removeWidget(wt)}
                    title="Remove widget"
                    aria-label="Remove {WIDGET_LABELS[wt] ?? wt}"
                  >✕</button>
                </div>
                <div class="edit-preview">
                  {#if wt === "p_scoreboard"}
                    <W1ScoreboardWidget {projectId} {projectColor} />
                  {:else if wt === "p_health_pulse"}
                    <W2HealthPulseWidget
                      {projectId}
                      {projectColor}
                      editMode={true}
                      config={widgetConfigs[wt] as { style?: "static" | "ecg" | "pulse" } | undefined}
                      onConfigChange={(cfg) => handleWidgetConfigChange(wt, cfg)}
                    />
                  {:else if wt === "p_velocity"}
                    <W3VelocityWidget {projectId} {projectColor} />
                  {:else if wt === "p_completion_dial"}
                    <W4CompletionDialWidget {projectId} {projectColor} />
                  {:else if wt === "p_fuel_gauge"}
                    <W5FuelGaugeWidget {projectId} {projectColor} />
                  {:else if wt === "p_effort_balance"}
                    <W6EffortBalanceWidget {projectId} {projectColor} />
                  {:else if wt === "p_weekly_rhythm"}
                    <W7WeeklyRhythmWidget {projectId} {projectColor} />
                  {:else if wt === "p_time_donut"}
                    <W9TimeBreakdownWidget {projectId} {projectColor} />
                  {:else if wt === "p_status_radial"}
                    <W10StatusRadialWidget {projectId} {projectColor} />
                  {:else if wt === "p_due_timeline"}
                    <W12DueTimelineWidget {projectId} {projectColor} />
                  {:else if wt === "p_burndown"}
                    <W13BurndownWidget {projectId} {projectColor} />
                  {:else if wt === "p_completion_trend"}
                    <W14CompletionTrendWidget {projectId} {projectColor} />
                  {:else if wt === "p_subproject_tree"}
                    <W16SubprojectTreeWidget {projectId} {projectColor} />
                  {:else if wt === "p_subproject_bars"}
                    <W17SubprojectBarsWidget {projectId} {projectColor} />
                  {:else if wt === "p_subproject_sunburst"}
                    <W18SubprojectSunburstWidget
                      {projectId}
                      {projectColor}
                      config={widgetConfigs[wt] as Record<string, unknown> | undefined}
                      editMode={true}
                    />
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
            {#each ALL_PROJECT_WIDGET_TYPES as wt (wt)}
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

  .theme-glass {
    --db-bg: #000000;
    --db-card: rgba(0, 10, 30, 0.55);
    --db-border: rgba(56, 189, 248, 0.18);
    --db-ink: #e0f2fe;
    --db-ink-muted: rgba(186, 230, 253, 0.55);
    --db-accent: #38bdf8;
    --db-grid-line: rgba(56, 189, 248, 0.06);
  }

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

  /* Project accent underline on title */
  .dash-title::after {
    content: "";
    display: block;
    margin-top: 3px;
    height: 2px;
    width: 40px;
    border-radius: 999px;
    background: var(--project-accent);
    opacity: 0.8;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 8px;
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
    border-color: var(--project-accent);
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
    background: var(--project-accent);
    color: #fff;
    border-color: var(--project-accent);
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
    border: 1px solid var(--project-accent);
    background: transparent;
    color: var(--project-accent);
    font: inherit;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    flex-shrink: 0;
    transition: background 150ms;
    white-space: nowrap;
  }

  .add-btn:hover {
    background: var(--project-accent);
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
    background: var(--project-accent);
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
