<script lang="ts">
  import { PRESET_COLOR_NAMES, PRESET_COLORS } from "$lib/colorPresets";

  interface Props {
    value: string;
    label: string;
  }

  let { value = $bindable(), label }: Props = $props();

  let isValid = $derived(value.trim() !== "" && CSS.supports("color", value));
</script>

<div class="color-picker">
  <div class="color-grid" role="group" aria-label={label}>
    {#each PRESET_COLORS as preset, index (preset)}
      <button
        type="button"
        class="color-swatch"
        class:selected={value === preset}
        style="background: {preset}"
        aria-pressed={value === preset}
        aria-label={`${PRESET_COLOR_NAMES[index]} (${preset})`}
        onclick={() => (value = preset)}
      ></button>
    {/each}
  </div>
  <div class="custom-color">
    <span class="preview-swatch" style="background: {value}" aria-hidden="true"></span>
    <input
      type="text"
      class="color-input"
      class:invalid={!isValid}
      bind:value
      aria-label={`${label} (custom value)`}
      aria-invalid={!isValid}
    />
  </div>
</div>

<style>
  .color-picker {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-sm);
  }

  .color-grid {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-xs);
  }

  .color-swatch {
    width: 1.5rem;
    height: 1.5rem;
    border-radius: var(--radius-pill);
    border: 1px solid var(--color-border);
    cursor: pointer;
    box-shadow: var(--shadow-sm);
    transition:
      box-shadow var(--duration-fast) var(--ease-out-expo),
      transform var(--duration-fast) var(--ease-out-expo);
  }

  .color-swatch:hover {
    transform: translateY(-1px);
  }

  .color-swatch.selected {
    box-shadow:
      0 0 0 2px var(--color-surface-raised),
      0 0 0 4px var(--color-accent);
  }

  .color-swatch:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .custom-color {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
  }

  .preview-swatch {
    width: 1.25rem;
    height: 1.25rem;
    border-radius: var(--radius-pill);
    border: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .color-input {
    width: 9rem;
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
    font-size: var(--text-xs);
    box-shadow: var(--shadow-sm);
    transition:
      border-color var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo);
  }

  .color-input:focus-visible {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-soft);
    outline: none;
  }

  .color-input.invalid {
    border-color: var(--color-danger);
  }

  .color-input.invalid:focus-visible {
    box-shadow: 0 0 0 3px var(--color-danger-soft);
  }
</style>
