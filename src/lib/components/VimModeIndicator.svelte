<script lang="ts">
  import { vimState } from "$lib/vim.svelte";

  let visible = $derived(vimState.active);
  let label = $derived(
    vimState.mode === "normal"
      ? "NORMAL"
      : vimState.mode === "visual"
        ? "VISUAL"
        : "SPARSE",
  );
  let modeClass = $derived(vimState.mode);
</script>

{#if visible}
  <div
    class="vim-statusbar {modeClass}"
    role="status"
    aria-live="polite"
    aria-label="Vim mode: {label}"
  >
    <span class="mode-label">{label}</span>
  </div>
{/if}

<style>
  .vim-statusbar {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    height: 20px;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding: 0 10px;
    pointer-events: none;
    border-top: 1px solid transparent;
  }

  .mode-label {
    font-family: var(--font-mono, monospace);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.1em;
  }

  .normal {
    background: rgba(34, 197, 94, 0.07);
    border-top-color: rgba(34, 197, 94, 0.12);
    color: #22c55e;
  }

  .visual {
    background: rgba(59, 130, 246, 0.07);
    border-top-color: rgba(59, 130, 246, 0.12);
    color: #3b82f6;
  }

  .visual_sparse {
    background: rgba(147, 51, 234, 0.07);
    border-top-color: rgba(147, 51, 234, 0.12);
    color: #9333ea;
  }
</style>
