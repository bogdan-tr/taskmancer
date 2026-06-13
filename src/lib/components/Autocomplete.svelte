<script lang="ts">
  interface Props {
    id: string;
    items: string[];
    activeIndex: number;
    onSelect: (item: string) => void;
    onHover: (index: number) => void;
    /** Shown before each item, e.g. `#` or `+`. */
    prefix?: string;
  }

  let { id, items, activeIndex, onSelect, onHover, prefix = "" }: Props = $props();
</script>

{#if items.length > 0}
  <ul {id} class="autocomplete" role="listbox">
    {#each items as item, index (item)}
      <li
        id="{id}-option-{index}"
        role="option"
        aria-selected={index === activeIndex}
        class:active={index === activeIndex}
        onpointerdown={(event) => {
          event.preventDefault();
          onSelect(item);
        }}
        onpointerenter={() => onHover(index)}
      >
        {prefix}{item}
      </li>
    {/each}
  </ul>
{/if}

<style>
  .autocomplete {
    position: absolute;
    top: calc(100% + var(--space-2xs));
    left: 0;
    right: 0;
    z-index: 10;
    display: flex;
    flex-direction: column;
    max-height: 12rem;
    overflow-y: auto;
    margin: 0;
    padding: var(--space-2xs);
    list-style: none;
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface-raised);
    box-shadow: var(--shadow-md);
  }

  .autocomplete li {
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-sm);
    font-size: var(--text-sm);
    color: var(--color-ink);
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .autocomplete li.active {
    background: var(--color-accent-soft);
    color: var(--color-accent);
    font-weight: 600;
  }
</style>
