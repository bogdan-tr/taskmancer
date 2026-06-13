<script lang="ts">
  import { setTheme, themeState } from "$lib/theme.svelte";
  import { THEMES, THEME_LABELS, type Theme } from "$lib/theme";

  const THEME_DESCRIPTIONS: Record<Theme, string> = {
    light: "Ivory canvas with a toned-down neon blue/green/pink accent triad.",
    dark: "Graphite canvas with full-vibrancy neon blue, green, and pink.",
    "dark-blue": "Indigo-tinted canvas with violet-leaning neon accents.",
  };
</script>

<main class="page">
  <header class="page-header">
    <a class="back-link" href="/">← Back to board</a>
    <h1 class="page-title">Settings</h1>
  </header>

  <section aria-labelledby="theme-heading">
    <h2 id="theme-heading">Theme</h2>
    <div class="theme-grid" role="radiogroup" aria-labelledby="theme-heading">
      {#each THEMES as option (option)}
        <label class="theme-card" class:selected={themeState.current === option}>
          <input
            type="radio"
            class="sr-only"
            name="theme"
            value={option}
            checked={themeState.current === option}
            onchange={() => setTheme(option)}
          />
          <div class="theme-preview" data-theme={option}>
            <div class="preview-surface">
              <span class="preview-accent-bar"></span>
              <span class="preview-dot" style="background: var(--color-neon-blue)"></span>
              <span class="preview-dot" style="background: var(--color-neon-green)"></span>
              <span class="preview-dot" style="background: var(--color-neon-pink)"></span>
            </div>
          </div>
          <span class="theme-name">
            {THEME_LABELS[option]}
            {#if themeState.current === option}
              <span class="theme-active-badge">Active</span>
            {/if}
          </span>
          <span class="theme-description">{THEME_DESCRIPTIONS[option]}</span>
        </label>
      {/each}
    </div>
  </section>
</main>

<style>
  .page {
    max-width: 720px;
    margin: 0 auto;
    padding: var(--space-xl) var(--space-lg) var(--space-2xl);
  }

  .page-header {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    padding-bottom: var(--space-lg);
    margin-bottom: var(--space-xl);
    border-bottom: 1px solid var(--color-border);
  }

  .back-link {
    color: var(--color-ink-muted);
    text-decoration: none;
    font-size: var(--text-sm);
  }

  .back-link:hover {
    color: var(--color-accent);
  }

  .page-title {
    font-size: var(--text-xl);
    font-weight: 600;
    letter-spacing: var(--tracking-tight);
  }

  h2 {
    margin: 0 0 var(--space-md);
    font-size: var(--text-sm);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
    color: var(--color-ink-muted);
  }

  .theme-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: var(--space-lg);
  }

  .theme-card {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    padding: var(--space-md);
    border-radius: var(--radius-lg);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    cursor: pointer;
    transition:
      border-color var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo),
      transform var(--duration-fast) var(--ease-out-expo);
  }

  .theme-card:hover {
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .theme-card.selected {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-soft);
  }

  .theme-card:focus-within {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: -1px;
    overflow: hidden;
    clip-path: inset(50%);
  }

  .theme-preview {
    padding: var(--space-sm);
    border-radius: var(--radius-md);
    background: var(--color-canvas);
  }

  .preview-surface {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding: var(--space-sm);
    border-radius: var(--radius-sm);
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    box-shadow: var(--shadow-sm);
  }

  .preview-accent-bar {
    flex: 1;
    height: var(--space-sm);
    border-radius: var(--radius-pill);
    /* --color-accent resolves through :root, so a non-active theme's
       override here would be ignored (custom properties resolve `var()` at
       computed-value time before inheriting). Use --color-neon-blue
       directly so the swatch reflects *this* card's theme. */
    background: var(--color-neon-blue);
  }

  .preview-dot {
    width: var(--space-sm);
    height: var(--space-sm);
    border-radius: var(--radius-pill);
    flex-shrink: 0;
  }

  .theme-name {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    font-weight: 600;
  }

  .theme-active-badge {
    padding: var(--space-3xs) var(--space-xs);
    border-radius: var(--radius-pill);
    background: var(--color-accent-soft);
    color: var(--color-accent);
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
  }

  .theme-description {
    font-size: var(--text-sm);
    color: var(--color-ink-muted);
  }
</style>
