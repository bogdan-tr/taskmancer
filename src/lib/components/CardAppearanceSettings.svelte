<script lang="ts">
  import {
    legibleInkColor,
    NEON_CARD_CHROMA_BOOST,
    WEEK_BAR_CHROMA_BOOST,
    neonCardColor,
    type InkMode,
  } from "$lib/colorPresets";
  import { getErrorMessage } from "$lib/errors";
  import { persistSettings, settingsState } from "$lib/settings.svelte";
  import { DEFAULT_PROJECT_COLOR } from "$lib/types";

  /** A representative project color for the live preview swatches below — not tied to any real project. */
  const PREVIEW_COLOR = DEFAULT_PROJECT_COLOR;

  const INK_MODE_OPTIONS: { value: InkMode; label: string }[] = [
    { value: "auto", label: "Auto" },
    { value: "white", label: "White" },
    { value: "black", label: "Black" },
  ];

  let draftCardLightness = $state(50);
  let draftBarLightness = $state(38);
  let draftInkMode: InkMode = $state("auto");
  let initialized = $state(false);
  let errorMessage = $state("");
  let isSaving = $state(false);

  let baselineCardLightness = $derived(Math.round((settingsState.current?.card_lightness ?? 0.5) * 100));
  let baselineBarLightness = $derived(Math.round((settingsState.current?.bar_lightness ?? 0.38) * 100));
  let baselineInkMode = $derived(settingsState.current?.ink_mode ?? "auto");

  /** Seeds the draft from settings once they finish loading; later edits live only in the draft. */
  $effect(() => {
    if (settingsState.current && !initialized) {
      draftCardLightness = baselineCardLightness;
      draftBarLightness = baselineBarLightness;
      draftInkMode = baselineInkMode;
      initialized = true;
    }
  });

  let isDirty = $derived(
    draftCardLightness !== baselineCardLightness ||
      draftBarLightness !== baselineBarLightness ||
      draftInkMode !== baselineInkMode,
  );

  let cardPreviewBg = $derived(neonCardColor(PREVIEW_COLOR, draftCardLightness / 100, NEON_CARD_CHROMA_BOOST));
  let cardPreviewText = $derived(legibleInkColor(cardPreviewBg, draftInkMode));
  let barPreviewBg = $derived(neonCardColor(PREVIEW_COLOR, draftBarLightness / 100, WEEK_BAR_CHROMA_BOOST));
  let barPreviewText = $derived(legibleInkColor(barPreviewBg, draftInkMode));

  function discardChanges() {
    draftCardLightness = baselineCardLightness;
    draftBarLightness = baselineBarLightness;
    draftInkMode = baselineInkMode;
    errorMessage = "";
  }

  async function save() {
    if (!settingsState.current) return;

    isSaving = true;
    try {
      await persistSettings({
        ...settingsState.current,
        card_lightness: draftCardLightness / 100,
        bar_lightness: draftBarLightness / 100,
        ink_mode: draftInkMode,
      });
      errorMessage = "";
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to save appearance settings");
    } finally {
      isSaving = false;
    }
  }
</script>

<section aria-labelledby="card-appearance-heading">
  <div class="section-header">
    <h2 id="card-appearance-heading">Card & bar appearance</h2>
  </div>
  <p class="description">
    Only applies in "Color code" card mode (see Display settings above). Controls how bright/dark
    the project-color background is on Kanban cards, and separately on week/calendar-view bars, plus
    whether the text on top of it is computed for contrast ("Auto") or always one fixed color. Any
    project can override these individually from its own settings page.
  </p>

  {#if !settingsState.current}
    <p class="loading">Loading…</p>
  {:else}
    <div class="control-row">
      <div class="control-label">
        <label for="card-lightness-input">Kanban card lightness</label>
        <span class="control-value">{draftCardLightness}%</span>
      </div>
      <div class="control-with-preview">
        <input
          id="card-lightness-input"
          type="range"
          min="0"
          max="100"
          step="1"
          bind:value={draftCardLightness}
        />
        <span
          class="preview-swatch"
          style="background: {cardPreviewBg}; color: {cardPreviewText}"
          aria-hidden="true"
        >
          Sample
        </span>
      </div>
    </div>

    <div class="control-row">
      <div class="control-label">
        <label for="bar-lightness-input">Week/calendar bar lightness</label>
        <span class="control-value">{draftBarLightness}%</span>
      </div>
      <div class="control-with-preview">
        <input
          id="bar-lightness-input"
          type="range"
          min="0"
          max="100"
          step="1"
          bind:value={draftBarLightness}
        />
        <span
          class="preview-swatch"
          style="background: {barPreviewBg}; color: {barPreviewText}"
          aria-hidden="true"
        >
          Sample
        </span>
      </div>
    </div>

    <div class="control-row">
      <div class="control-label">
        <span id="ink-mode-label">Card &amp; bar text color</span>
      </div>
      <div class="ink-mode-group" role="radiogroup" aria-labelledby="ink-mode-label">
        {#each INK_MODE_OPTIONS as option (option.value)}
          <button
            type="button"
            role="radio"
            aria-checked={draftInkMode === option.value}
            class:selected={draftInkMode === option.value}
            onclick={() => (draftInkMode = option.value)}
          >
            {option.label}
          </button>
        {/each}
      </div>
    </div>

    {#if errorMessage}
      <p class="error" role="alert">{errorMessage}</p>
    {/if}

    <div class="actions">
      <button type="button" class="secondary" disabled={!isDirty || isSaving} onclick={discardChanges}>
        Discard changes
      </button>
      <button type="button" disabled={!isDirty || isSaving} onclick={save}>
        {isSaving ? "Saving…" : "Save changes"}
      </button>
    </div>
  {/if}
</section>

<style>
  section {
    margin-top: var(--space-2xl);
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-md);
  }

  .section-header h2 {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
  }

  .description {
    margin: 0 0 var(--space-md);
    font-size: var(--text-sm);
    color: var(--color-ink-muted);
  }

  .loading {
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
  }

  .control-row {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
    margin-bottom: var(--space-md);
  }

  .control-label {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-ink);
  }

  .control-value {
    font-variant-numeric: tabular-nums;
    font-weight: 400;
    color: var(--color-ink-muted);
  }

  .control-with-preview {
    display: flex;
    align-items: center;
    gap: var(--space-md);
  }

  .control-with-preview input[type="range"] {
    flex: 1;
    accent-color: var(--color-accent);
  }

  .ink-mode-group {
    display: flex;
    gap: var(--space-2xs);
  }

  .ink-mode-group button {
    flex: 1;
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
    font-weight: 600;
    cursor: pointer;
    transition:
      background var(--duration-fast) var(--ease-out-expo),
      color var(--duration-fast) var(--ease-out-expo),
      border-color var(--duration-fast) var(--ease-out-expo);
  }

  .ink-mode-group button.selected {
    background: var(--color-accent);
    border-color: var(--color-accent);
    color: var(--color-accent-ink);
  }

  .ink-mode-group button:not(.selected):hover {
    background: var(--color-canvas);
    color: var(--color-ink);
  }

  .preview-swatch {
    flex-shrink: 0;
    width: 5rem;
    padding: var(--space-2xs) 0;
    border-radius: var(--radius-md);
    text-align: center;
    font-size: var(--text-xs);
    font-weight: 600;
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
    gap: var(--space-xs);
  }

  .actions button {
    padding: var(--space-sm) var(--space-lg);
    border-radius: var(--radius-md);
    border: none;
    font-weight: 600;
    font-size: var(--text-base);
    cursor: pointer;
    box-shadow: var(--shadow-sm);
    transition:
      background var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo),
      transform var(--duration-fast) var(--ease-out-expo);
  }

  .actions button:not(.secondary) {
    background: var(--color-accent);
    color: var(--color-accent-ink);
  }

  .actions button:not(.secondary):hover:not(:disabled) {
    background: var(--color-accent-hover);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .actions button:disabled {
    background: var(--color-border);
    color: var(--color-ink-muted);
    cursor: not-allowed;
    box-shadow: none;
    transform: none;
  }

  .actions button.secondary {
    background: var(--color-surface);
    color: var(--color-ink);
    border: 1px solid var(--color-border);
    box-shadow: none;
  }

  .actions button.secondary:hover:not(:disabled) {
    background: var(--color-canvas);
  }
</style>
