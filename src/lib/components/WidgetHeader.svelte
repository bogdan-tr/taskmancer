<script lang="ts">
  /**
   * The standardized header every dashboard widget renders: title, a
   * date-range badge declaring the widget's time behavior, and an ⓘ button
   * opening a definition popover sourced from `widgetCatalog.ts`.
   *
   * The popover uses `position: fixed` (computed from the button's rect on
   * open) because widget cards clip overflow — an absolutely-positioned
   * popover would be cut off at the card edge.
   */
  import type { DashboardDateRange } from "$lib/api";
  import { WIDGET_CATALOG, rangeBadgeText } from "$lib/widgetCatalog";
  import type { WidgetTypeId } from "$lib/widgetCatalog";

  interface Props {
    widgetType: WidgetTypeId;
    /** The dashboard's currently selected picker range — only used when the
     *  catalog says this widget follows the picker. */
    pickerRange?: DashboardDateRange;
  }
  let { widgetType, pickerRange }: Props = $props();

  let meta = $derived(WIDGET_CATALOG[widgetType]);
  let badge = $derived(rangeBadgeText(meta, pickerRange));
  let followsPicker = $derived(meta.range.kind === "picker");

  let open = $state(false);
  let pinned = $state(false);
  let popX = $state(0);
  let popY = $state(0);
  let alignRight = $state(false);
  let btnEl = $state<HTMLButtonElement | null>(null);
  let popEl = $state<HTMLDivElement | null>(null);
  let closeTimer: ReturnType<typeof setTimeout> | undefined;

  /**
   * Moves the popover into <body>. Widget cards use `overflow: hidden`, and
   * the glass/aurora theme cards use `backdrop-filter`, which makes the card
   * the containing block for `position: fixed` descendants — the popover was
   * being positioned card-relative and clipped away entirely. Portaling out
   * escapes both. (Svelte's scoped-style class travels with the node, so the
   * component styles still apply.)
   */
  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return { destroy: () => node.remove() };
  }

  function show() {
    clearTimeout(closeTimer);
    const rect = btnEl?.getBoundingClientRect();
    if (rect) {
      // Prefer opening below-left-aligned; flip when it would run off-screen.
      alignRight = rect.left + 280 > window.innerWidth;
      popX = alignRight ? rect.right : rect.left;
      popY = rect.bottom + 6;
    }
    open = true;
  }

  function hide() {
    clearTimeout(closeTimer);
    open = false;
    pinned = false;
  }

  /** Grace delay so the pointer can travel from the button into the popover. */
  function scheduleHide() {
    if (pinned) return;
    clearTimeout(closeTimer);
    closeTimer = setTimeout(() => (open = false), 250);
  }

  function cancelHide() {
    clearTimeout(closeTimer);
  }

  function toggle() {
    pinned = !pinned;
    if (pinned) show();
    else hide();
  }

  function onWindowEvent(e: Event) {
    if (!open) return;
    if (e instanceof KeyboardEvent) {
      if (e.key === "Escape") hide();
      return;
    }
    const t = e.target as Node | null;
    if (t && (btnEl?.contains(t) || popEl?.contains(t))) return;
    hide();
  }
</script>

<svelte:window onkeydown={onWindowEvent} onpointerdown={onWindowEvent} onresize={hide} />

<div class="widget-header">
  <span class="widget-title">{meta.title}</span>
  <span class="header-spacer"></span>
  <span class="range-badge" class:follows-picker={followsPicker} title="Date-range behavior">
    {badge}
  </span>
  <button
    class="info-btn"
    type="button"
    bind:this={btnEl}
    onclick={toggle}
    onmouseenter={show}
    onmouseleave={scheduleHide}
    aria-expanded={open}
    aria-label="What does this widget show?"
  >i</button>
</div>

{#if open}
  <div
    class="info-popover"
    role="note"
    use:portal
    bind:this={popEl}
    onmouseenter={cancelHide}
    onmouseleave={scheduleHide}
    style="top: {popY}px; {alignRight ? `right: ${window.innerWidth - popX}px` : `left: ${popX}px`}"
  >
    <p class="pop-title">{meta.title}</p>
    <p class="pop-section-label">What it shows</p>
    <p class="pop-text">{meta.what}</p>
    <p class="pop-section-label">How it's computed</p>
    <p class="pop-text">{meta.how}</p>
    <p class="pop-range">
      <span class="pop-range-badge" class:follows-picker={followsPicker}>{badge}</span>
      {#if meta.range.kind === "picker"}
        Follows the dashboard's date-range picker.
      {:else if meta.range.kind === "all_time"}
        Always shows all of history — ignores the date-range picker.
      {:else if meta.range.kind === "now"}
        A snapshot of the current state — ignores the date-range picker.
      {:else}
        Has its own timeline — ignores the date-range picker.
      {/if}
    </p>
  </div>
{/if}

<style>
  .widget-header {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
    min-width: 0;
  }

  .widget-title {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--db-ink-muted, #8b949e);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .header-spacer {
    flex: 1;
  }

  .range-badge {
    font-size: 8.5px;
    font-weight: 700;
    letter-spacing: 0.1em;
    padding: 2px 6px;
    border-radius: 999px;
    border: 1px solid var(--db-border, rgba(255, 255, 255, 0.08));
    color: var(--db-ink-muted, #8b949e);
    white-space: nowrap;
    flex-shrink: 0;
    line-height: 1;
    user-select: none;
  }

  .range-badge.follows-picker {
    border-color: color-mix(in srgb, var(--project-accent, var(--db-accent, #58a6ff)) 45%, transparent);
    color: color-mix(in srgb, var(--project-accent, var(--db-accent, #58a6ff)) 80%, var(--db-ink, #fff));
    background: color-mix(in srgb, var(--project-accent, var(--db-accent, #58a6ff)) 10%, transparent);
  }

  .info-btn {
    width: 15px;
    height: 15px;
    padding: 0;
    border-radius: 50%;
    border: 1px solid var(--db-border, rgba(255, 255, 255, 0.12));
    background: transparent;
    color: var(--db-ink-muted, #8b949e);
    font: inherit;
    font-size: 9.5px;
    font-weight: 700;
    font-style: italic;
    font-family: Georgia, "Times New Roman", serif;
    line-height: 1;
    cursor: pointer;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: color 150ms, border-color 150ms, background 150ms;
  }

  .info-btn:hover,
  .info-btn[aria-expanded="true"] {
    color: var(--db-ink, #e6edf3);
    border-color: var(--project-accent, var(--db-accent, #58a6ff));
    background: color-mix(in srgb, var(--project-accent, var(--db-accent, #58a6ff)) 12%, transparent);
  }

  .info-popover {
    position: fixed;
    z-index: 1000;
    width: 264px;
    max-width: calc(100vw - 24px);
    padding: 12px 14px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--db-card, #161b22) 88%, #000);
    border: 1px solid var(--db-border, rgba(255, 255, 255, 0.1));
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
  }

  .pop-title {
    margin: 0 0 8px;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--db-ink, #e6edf3);
  }

  .pop-section-label {
    margin: 8px 0 2px;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: color-mix(in srgb, var(--project-accent, var(--db-accent, #58a6ff)) 75%, var(--db-ink-muted, #8b949e));
  }

  .pop-text {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--db-ink-muted, #b0b8c0);
  }

  .pop-range {
    margin: 10px 0 0;
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 10.5px;
    line-height: 1.4;
    color: var(--db-ink-muted, #8b949e);
    border-top: 1px solid var(--db-border, rgba(255, 255, 255, 0.08));
    padding-top: 8px;
  }

  .pop-range-badge {
    font-size: 8.5px;
    font-weight: 700;
    letter-spacing: 0.1em;
    padding: 2px 6px;
    border-radius: 999px;
    border: 1px solid var(--db-border, rgba(255, 255, 255, 0.12));
    white-space: nowrap;
    flex-shrink: 0;
  }

  .pop-range-badge.follows-picker {
    border-color: color-mix(in srgb, var(--project-accent, var(--db-accent, #58a6ff)) 45%, transparent);
    color: color-mix(in srgb, var(--project-accent, var(--db-accent, #58a6ff)) 80%, var(--db-ink, #fff));
  }
</style>
