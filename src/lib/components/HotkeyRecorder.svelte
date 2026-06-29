<script lang="ts">
  import { parseKeyCombo } from "$lib/vimKeybindings";

  interface Props {
    value: string[];
    onchange: (combos: string[]) => void;
    conflictsWith?: string;
    disabled?: boolean;
  }

  let { value, onchange, conflictsWith, disabled = false }: Props = $props();

  let listening = $state(false);

  function removeCombo(combo: string) {
    onchange(value.filter((c) => c !== combo));
  }

  function startListening() {
    if (disabled) return;
    listening = true;
  }

  function handleListeningKeydown(event: KeyboardEvent) {
    event.preventDefault();
    event.stopPropagation();
    if (event.key === "Escape") {
      listening = false;
      return;
    }
    if (["Control", "Alt", "Shift", "Meta"].includes(event.key)) return;
    const combo = parseKeyCombo(event);
    if (!value.includes(combo)) {
      onchange([...value, combo]);
    }
    listening = false;
  }
</script>

{#if listening}
  <!-- svelte-ignore a11y_autofocus -->
  <div
    class="recorder-listening"
    tabindex="0"
    autofocus
    role="button"
    aria-label="Press any key"
    onkeydown={handleListeningKeydown}
    onblur={() => (listening = false)}
  >Press any key… (Escape to cancel)</div>
{:else}
  <div class="recorder-row">
    {#each value as combo (combo)}
      <span class="combo-chip">
        <span class="combo-label">{combo}</span>
        {#if !disabled}
          <button class="combo-remove" onclick={() => removeCombo(combo)} aria-label="Remove {combo}">×</button>
        {/if}
      </span>
    {/each}
    {#if !disabled}
      <button class="add-shortcut" onclick={startListening}>+ Add shortcut</button>
    {/if}
  </div>
{/if}
{#if conflictsWith}
  <p class="conflict-error">⚠ Already used for: {conflictsWith}</p>
{/if}

<style>
  .recorder-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }

  .combo-chip {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    background: var(--color-canvas);
    border: 1px solid var(--color-border);
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 12px;
    font-family: var(--font-mono, monospace);
  }

  .combo-remove {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--color-ink-muted);
    padding: 0;
    font-size: 14px;
    line-height: 1;
  }

  .combo-remove:hover {
    color: var(--color-danger, #ef4444);
  }

  .add-shortcut {
    background: none;
    border: 1px dashed var(--color-border);
    border-radius: 4px;
    padding: 2px 8px;
    font-size: 12px;
    cursor: pointer;
    color: var(--color-ink-muted);
  }

  .add-shortcut:hover {
    border-color: var(--color-accent);
    color: var(--color-accent);
  }

  .recorder-listening {
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 12px;
    border: 2px solid var(--color-accent);
    color: var(--color-ink-muted);
    outline: none;
    cursor: text;
    animation: pulse-border 1.5s ease-in-out infinite;
  }

  @keyframes pulse-border {
    0%, 100% { border-color: var(--color-accent); }
    50% { border-color: color-mix(in oklch, var(--color-accent) 50%, transparent); }
  }

  .conflict-error {
    font-size: 11px;
    color: var(--color-danger, #ef4444);
    margin: 4px 0 0;
  }
</style>
