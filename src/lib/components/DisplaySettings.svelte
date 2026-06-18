<script lang="ts">
  import {
    BOARD_WIDTH_STEP,
    COLUMN_WIDTH_STEP,
    displayState,
    FONT_SCALE_STEP,
    MAX_BOARD_WIDTH,
    MAX_COLUMN_WIDTH,
    MAX_FONT_SCALE,
    MIN_BOARD_WIDTH,
    MIN_COLUMN_WIDTH,
    MIN_FONT_SCALE,
    setBoardWidth,
    setCardColorMode,
    setColumnWidth,
    setDueDateGlow,
    setFontScale,
    setNlDueDates,
    setShowPriorityChip,
    setShowPriorityGroups,
    setWeekStartsOn,
    type CardColorMode,
    type WeekStartsOn,
  } from "$lib/displaySettings.svelte";

  function handleFontScaleInput(event: Event) {
    setFontScale(Number((event.currentTarget as HTMLInputElement).value));
  }

  function handleColumnWidthInput(event: Event) {
    setColumnWidth(Number((event.currentTarget as HTMLInputElement).value));
  }

  function handleBoardWidthInput(event: Event) {
    setBoardWidth(Number((event.currentTarget as HTMLInputElement).value));
  }

  function handleShowPriorityGroupsChange(event: Event) {
    setShowPriorityGroups((event.currentTarget as HTMLInputElement).checked);
  }

  function handleShowPriorityChipChange(event: Event) {
    setShowPriorityChip((event.currentTarget as HTMLInputElement).checked);
  }

  function handleWeekStartsOnChange(event: Event) {
    setWeekStartsOn((event.currentTarget as HTMLSelectElement).value as WeekStartsOn);
  }

  function handleNlDueDatesChange(event: Event) {
    setNlDueDates((event.currentTarget as HTMLInputElement).checked);
  }

  function handleCardColorModeChange(event: Event) {
    setCardColorMode((event.currentTarget as HTMLInputElement).value as CardColorMode);
  }

  function handleDueDateGlowChange(event: Event) {
    setDueDateGlow((event.currentTarget as HTMLInputElement).checked);
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

  <div class="control-row">
    <div class="control-label">
      <label for="board-width-input">Board width</label>
      <span class="control-value">{displayState.boardWidth}px</span>
    </div>
    <input
      id="board-width-input"
      type="range"
      min={MIN_BOARD_WIDTH}
      max={MAX_BOARD_WIDTH}
      step={BOARD_WIDTH_STEP}
      value={displayState.boardWidth}
      oninput={handleBoardWidthInput}
    />
  </div>

  <div class="control-row">
    <div class="control-label">
      <label for="week-starts-on-select">Week starts on</label>
    </div>
    <select id="week-starts-on-select" value={displayState.weekStartsOn} onchange={handleWeekStartsOnChange}>
      <option value="monday">Monday</option>
      <option value="sunday">Sunday</option>
    </select>
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

  <label class="toggle-row">
    <input
      type="checkbox"
      checked={displayState.showPriorityChip}
      onchange={handleShowPriorityChipChange}
    />
    <span class="toggle-text">
      <span class="toggle-label">Show priority labels on task cards</span>
      <span class="toggle-description">
        Display the priority name and dot on each task card. When off, the priority chip is
        hidden, but the card's priority accent color is still shown.
      </span>
    </span>
  </label>

  <label class="toggle-row">
    <input type="checkbox" checked={displayState.nlDueDates} onchange={handleNlDueDatesChange} />
    <span class="toggle-text">
      <span class="toggle-label">Natural language due dates</span>
      <span class="toggle-description">
        Show due dates 2-13 days out as "Due this Wednesday" / "Due next Saturday" instead of
        YYYY-MM-DD. Today, tomorrow, and overdue labels are unaffected by this setting.
      </span>
    </span>
  </label>

  <fieldset class="control-row card-color-mode">
    <legend class="control-label">Card color mode</legend>
    <label class="radio-option">
      <input
        type="radio"
        name="card-color-mode"
        value="project_tag"
        checked={displayState.cardColorMode === "project_tag"}
        onchange={handleCardColorModeChange}
      />
      <span class="toggle-text">
        <span class="toggle-label">Project tag</span>
        <span class="toggle-description">
          Show the project as a colored chip on each card. The card itself keeps a neutral
          background.
        </span>
      </span>
    </label>
    <label class="radio-option">
      <input
        type="radio"
        name="card-color-mode"
        value="color_code"
        checked={displayState.cardColorMode === "color_code"}
        onchange={handleCardColorModeChange}
      />
      <span class="toggle-text">
        <span class="toggle-label">Color code</span>
        <span class="toggle-description">
          Color the whole card background with the project's color instead of showing a project
          chip. Text color automatically switches to stay readable on any project color.
        </span>
      </span>
    </label>
  </fieldset>

  <label class="toggle-row">
    <input type="checkbox" checked={displayState.dueDateGlow} onchange={handleDueDateGlowChange} />
    <span class="toggle-text">
      <span class="toggle-label">Due-date glow</span>
      <span class="toggle-description">
        Surround overdue and due-today task cards with a fading red glow, in addition to the due
        chip.
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

  .control-row select {
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
    font-size: var(--text-sm);
    box-shadow: var(--shadow-sm);
    transition:
      border-color var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo);
  }

  .control-row select:focus-visible {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-soft);
    outline: none;
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
    margin-bottom: var(--space-2xs);
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

  fieldset.card-color-mode {
    border: none;
    padding: 0;
    margin: 0 0 var(--space-md) 0;
    gap: var(--space-2xs);
  }

  fieldset.card-color-mode legend {
    padding: 0;
    margin-bottom: var(--space-2xs);
  }

  .radio-option {
    display: flex;
    align-items: flex-start;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    cursor: pointer;
    margin-bottom: var(--space-2xs);
  }

  .radio-option input[type="radio"] {
    margin-top: 0.2rem;
    width: 1.1rem;
    height: 1.1rem;
    flex-shrink: 0;
    accent-color: var(--color-accent);
    cursor: pointer;
  }
</style>
