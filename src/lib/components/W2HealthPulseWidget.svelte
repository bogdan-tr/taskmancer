<script lang="ts">
  import { getProjectHealthPulse } from "$lib/api";
  import type { ProjectHealthPulse } from "$lib/types";

  interface Props {
    projectId: string;
    projectColor: string;
    editMode?: boolean;
    config?: { style?: "static" | "ecg" | "pulse" };
    onConfigChange?: (cfg: { style: "static" | "ecg" | "pulse" }) => void;
  }
  let {
    projectId,
    projectColor,
    editMode = false,
    config,
    onConfigChange,
  }: Props = $props();

  let data = $state<ProjectHealthPulse | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let showSettings = $state(false);

  let animStyle = $derived<"static" | "ecg" | "pulse">(config?.style ?? "pulse");

  $effect(() => {
    if (!projectId) return;
    loading = true;
    error = null;
    getProjectHealthPulse(projectId)
      .then((d) => { data = d; })
      .catch((e) => { error = e instanceof Error ? e.message : String(e); })
      .finally(() => { loading = false; });
  });

  // Animation period in seconds based on tier severity
  const TIER_PERIOD: Record<string, number> = {
    Great: 2,
    "On Track": 1.5,
    "Needs Attention": 1,
    Critical: 0.6,
    Severe: 0.4,
  };

  let period = $derived(data ? (TIER_PERIOD[data.tier] ?? 1.5) : 1.5);
  let tierColor = $derived(data?.tier_color ?? projectColor);
  let tierLabel = $derived(data?.tier?.toUpperCase() ?? "");

  function handleStyleChange(s: "static" | "ecg" | "pulse") {
    onConfigChange?.({ style: s });
    showSettings = false;
  }
</script>

<div class="health-pulse" style="--project-accent: {projectColor}; --tier-color: {tierColor}; --period: {period}s">
  {#if loading}
    <div class="skeleton">
      <div class="skeleton-heart"></div>
      <div class="skeleton-label"></div>
    </div>
  {:else if error}
    <p class="err">{error}</p>
  {:else if data}
    <div class="widget-body">
      <!-- ECG background wave -->
      {#if animStyle === "ecg"}
        <svg class="ecg-wave" viewBox="0 0 400 60" preserveAspectRatio="none" aria-hidden="true">
          <polyline
            points="0,30 60,30 80,5 100,55 120,30 160,30 180,10 200,50 220,30 260,30 280,8 300,52 320,30 360,30 380,12 400,48"
            fill="none"
            stroke={tierColor}
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      {/if}

      <!-- Heart SVG -->
      <div class="heart-wrap" class:anim-pulse={animStyle === "pulse"}>
        <svg
          class="heart-svg"
          viewBox="-10 0 120 100"
          xmlns="http://www.w3.org/2000/svg"
          aria-hidden="true"
        >
          {#if animStyle === "static"}
            <filter id="glow">
              <feGaussianBlur stdDeviation="3" result="blur" />
              <feMerge>
                <feMergeNode in="blur" />
                <feMergeNode in="SourceGraphic" />
              </feMerge>
            </filter>
          {/if}
          <path
            d="M 50,30 C 50,30 20,10 10,30 C 0,50 20,70 50,90 C 80,70 100,50 90,30 C 80,10 50,30 50,30 Z"
            fill={tierColor}
            filter={animStyle === "static" ? "url(#glow)" : "none"}
            opacity="0.92"
          />
        </svg>
      </div>

      <!-- Tier label -->
      <p class="tier-label">{tierLabel}</p>

      <!-- Due pills -->
      <div class="pill-row">
        <span class="pill">Today: {data.due_today}</span>
        <span class="pill">Tomorrow: {data.due_tomorrow}</span>
      </div>

      <!-- Gear button (edit mode only) -->
      {#if editMode}
        <button
          class="gear-btn"
          type="button"
          onclick={() => { showSettings = !showSettings; }}
          title="Widget settings"
          aria-label="Health pulse widget settings"
        >
          ⚙
        </button>
        {#if showSettings}
          <div class="settings-popover">
            <p class="popover-label">Animation</p>
            {#each (["pulse", "ecg", "static"] as const) as s (s)}
              <label class="radio-row">
                <input
                  type="radio"
                  name="hp-style"
                  value={s}
                  checked={animStyle === s}
                  onchange={() => handleStyleChange(s)}
                />
                {s.charAt(0).toUpperCase() + s.slice(1)}
              </label>
            {/each}
          </div>
        {/if}
      {/if}
    </div>
  {:else}
    <p class="empty">No health data.</p>
  {/if}
</div>

<style>
  .health-pulse {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    overflow: hidden;
  }

  .widget-body {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    width: 100%;
    height: 100%;
    position: relative;
  }

  /* ECG wave */
  .ecg-wave {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    opacity: 0.22;
    animation: ecg-scroll var(--period) linear infinite;
  }

  @keyframes ecg-scroll {
    from { transform: translateX(0); }
    to { transform: translateX(-50%); }
  }

  /* Heart */
  .heart-wrap {
    width: clamp(48px, 50%, 90px);
    aspect-ratio: 1;
    position: relative;
    z-index: 1;
    flex-shrink: 0;
  }

  .heart-svg {
    width: 100%;
    height: 100%;
    overflow: visible;
  }

  .anim-pulse {
    animation: heartbeat var(--period) ease-in-out infinite;
  }

  @keyframes heartbeat {
    0%, 100% { transform: scale(1); }
    40% { transform: scale(1.12); }
    60% { transform: scale(1.06); }
  }

  .tier-label {
    margin: 0;
    font-size: clamp(11px, 1.8vw, 14px);
    font-weight: 800;
    letter-spacing: 0.12em;
    color: var(--tier-color);
    text-transform: uppercase;
    position: relative;
    z-index: 1;
  }

  .pill-row {
    display: flex;
    gap: 6px;
    position: relative;
    z-index: 1;
  }

  .pill {
    font-size: 10px;
    font-weight: 600;
    color: var(--db-ink-muted);
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--db-border);
    border-radius: 999px;
    padding: 2px 8px;
  }

  /* Gear button */
  .gear-btn {
    position: absolute;
    top: 6px;
    right: 6px;
    background: transparent;
    border: 1px solid var(--db-border);
    border-radius: 5px;
    color: var(--db-ink-muted);
    font-size: 12px;
    padding: 2px 5px;
    cursor: pointer;
    line-height: 1;
    transition: background 150ms, color 150ms;
    z-index: 10;
  }

  .gear-btn:hover {
    background: var(--db-card);
    color: var(--db-ink);
    border-color: var(--db-accent);
  }

  /* Settings popover */
  .settings-popover {
    position: absolute;
    top: 30px;
    right: 6px;
    background: var(--db-card, #161b22);
    border: 1px solid var(--db-border);
    border-radius: 8px;
    padding: 10px 12px;
    z-index: 20;
    min-width: 130px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  }

  .popover-label {
    margin: 0 0 4px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--db-ink-muted);
  }

  .radio-row {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--db-ink);
    cursor: pointer;
  }

  .radio-row input[type="radio"] {
    accent-color: var(--tier-color);
    cursor: pointer;
  }

  /* Skeleton */
  .skeleton {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 16px;
  }

  .skeleton-heart {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    background: var(--db-border);
    animation: pulse 1.4s ease-in-out infinite;
  }

  .skeleton-label {
    width: 70px;
    height: 12px;
    border-radius: 4px;
    background: var(--db-border);
    animation: pulse 1.4s ease-in-out infinite 0.2s;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 0.8; }
  }

  .err, .empty {
    margin: 0;
    font-size: 12px;
    color: var(--db-ink-muted);
    text-align: center;
    padding: 16px;
  }
  .err { color: #ef4444; }
</style>
