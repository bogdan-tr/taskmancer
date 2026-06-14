<script lang="ts">
  import {
    COLUMN_WIDTH_STEP,
    displayState,
    FONT_SCALE_STEP,
    MAX_COLUMN_WIDTH,
    MAX_FONT_SCALE,
    MIN_COLUMN_WIDTH,
    MIN_FONT_SCALE,
    setColumnWidth,
    setFontScale,
    setShowPriorityGroups,
  } from "$lib/displaySettings.svelte";

  function handleFontScaleInput(event: Event) {
    setFontScale(Number((event.currentTarget as HTMLInputElement).value));
  }

  function handleColumnWidthInput(event: Event) {
    setColumnWidth(Number((event.currentTarget as HTMLInputElement).value));
  }

  function handleShowPriorityGroupsChange(event: Event) {
    setShowPriorityGroups((event.currentTarget as HTMLInputElement).checked);
  }
</script>

<section aria-labelledby="display-heading">
  <div class="section-header">
    <h2 id="display-heading">Display</h2>
  </div>

  <div class="control-row">
    <div class="control-label">
      <label for="font-scale-input">Font size</label>
      <span class="control-value">{displayState.fontScale}%</span>
    </div>
    <input
      id="font-scale-input"
      type="range"
      min={MIN_FONT_SCALE}
      max={MAX_FONT_SCALE}
      step={FONT_SCALE_STEP}
      value={displayState.fontScale}
      oninput={handleFontScaleInput}
    />
  </div>

  <div class="control-row">
    <div class="control-label">
      <label for="column-width-input">Status column width</label>
      <span class="control-value">{displayState.columnWidth}px</span>
    </div>
    <input
      id="column-width-input"
      type="range"
      min={MIN_COLUMN_WIDTH}
      max={MAX_COLUMN_WIDTH}
      step={COLUMN_WIDTH_STEP}
      value={displayState.columnWidth}
      oninput={handleColumnWidthInput}
    />
  </div>

  <label class="toggle-row">
    <input
      type="checkbox"
      checked={displayState.showPriorityGroups}
      onchange={handleShowPriorityGroupsChange}
    />
    <span class="toggle-text">
      <span class="toggle-label">Group tasks by priority</span>
      <span class="toggle-description">
        Show separate priority sections inside each status column. When off, tasks stay sorted by
        priority but the column isn't divided into separate blocks.
      </span>
    </span>
  </label>
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

  .control-row input[type="range"] {
    width: 100%;
    accent-color: var(--color-accent);
  }

  .toggle-row {
    display: flex;
    align-items: flex-start;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    cursor: pointer;
  }

  .toggle-row input[type="checkbox"] {
    margin-top: 0.2rem;
    width: 1.1rem;
    height: 1.1rem;
    flex-shrink: 0;
    accent-color: var(--color-accent);
    cursor: pointer;
  }

  .toggle-text {
    display: flex;
    flex-direction: column;
    gap: var(--space-3xs);
  }

  .toggle-label {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--color-ink);
  }

  .toggle-description {
    font-size: var(--text-xs);
    color: var(--color-ink-muted);
  }
</style>
