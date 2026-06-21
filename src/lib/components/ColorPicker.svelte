<script lang="ts">
  import { cssColorToHex, isHexColor, PRESET_COLOR_NAMES, PRESET_COLORS, shadesOf } from "$lib/colorPresets";

  interface Props {
    value: string;
    label: string;
    /** When set (creating/editing a subproject), shows shade suggestions derived from this color ahead of the fixed presets. */
    parentColor?: string;
    /** The parent project's name, used to label the shade suggestions (e.g. "Shades of Homework"). Ignored if `parentColor` isn't set. */
    parentName?: string;
  }

  let { value = $bindable(), label, parentColor, parentName }: Props = $props();

  let isValid = $derived(value.trim() !== "" && CSS.supports("color", value));
  let shadeSuggestions = $derived(parentColor ? shadesOf(parentColor, 5) : []);

  // Migrates legacy non-hex (e.g. oklch) color values to hex on display, so
  // saving the form persists the hex form without further action.
  $effect(() => {
    if (value.trim() === "" || isHexColor(value)) return;
    const hex = cssColorToHex(value);
    if (hex !== value) value = hex;
  });
</script>

{#snippet swatchButton(color: string, swatchLabel: string)}
  <button
    type="button"
    class="color-swatch"
    class:selected={value === color}
    style="background: {color}"
    aria-pressed={value === color}
    aria-label={swatchLabel}
    onclick={() => (value = color)}
  ></button>
{/snippet}

<div class="color-picker">
  {#if shadeSuggestions.length > 0}
    <div class="color-group">
      <span class="group-label">Shades of {parentName ?? "parent"}</span>
      <div class="color-grid" role="group" aria-label={`Shades of ${parentName ?? "parent"}`}>
        {#each shadeSuggestions as shade, index (shade)}
          {@render swatchButton(shade, `Shade ${index + 1} (${shade})`)}
        {/each}
      </div>
    </div>
    <span class="group-label">Presets</span>
  {/if}
  <div class="color-grid" role="group" aria-label={label}>
    {#each PRESET_COLORS as preset, index (preset)}
      {@render swatchButton(preset, `${PRESET_COLOR_NAMES[index]} (${preset})`)}
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

  .color-group {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
  }

  .group-label {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--color-ink-muted);
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
