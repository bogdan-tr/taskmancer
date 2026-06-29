<script lang="ts">
  import { getProjectEffortBalance } from "$lib/api";
  import type { ProjectEffortBalance } from "$lib/types";

  interface Props {
    projectId: string;
    projectColor: string;
  }
  let { projectId, projectColor }: Props = $props();

  let data = $state<ProjectEffortBalance | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let clientW = $state(0);
  let clientH = $state(0);

  $effect(() => {
    if (!projectId) return;
    loading = true;
    error = null;
    getProjectEffortBalance(projectId)
      .then((d) => { data = d; })
      .catch((e) => { error = e instanceof Error ? e.message : String(e); })
      .finally(() => { loading = false; });
  });

  // Responsive layout — all constants derived from measured container size.
  const VB_W = 200;

  // VB_H scales with the container aspect ratio, clamped between 120 and 200.
  let VB_H = $derived(
    Math.min(200, Math.max(120, clientH > 0 ? Math.round((clientH / Math.max(clientW, 1)) * VB_W) : 140)),
  );

  // Bar geometry derived from viewbox height.
  let BAR_AREA_TOP = $derived(Math.round(VB_H * 0.14));
  let BAR_AREA_BOTTOM = $derived(Math.round(VB_H * 0.74));
  let BAR_AREA_H = $derived(BAR_AREA_BOTTOM - BAR_AREA_TOP);
  let LABEL_Y = $derived(BAR_AREA_BOTTOM + 6);
  let VALUE_OFFSET = $derived(Math.round(VB_H * 0.04));

  // Two bars centred in VB_W with even gaps.
  const BAR_W = 64;
  const GAP = (VB_W - 2 * BAR_W) / 3; // ~24

  // Font sizes scale slightly with VB_H.
  let FS_VALUE = $derived(Math.max(7, Math.round(VB_H * 0.065)));
  let FS_LABEL = $derived(Math.max(6.5, Math.round(VB_H * 0.058)));
  let FS_HEADER = $derived(Math.max(8, Math.round(VB_H * 0.068)));

  function barX(i: number): number {
    return GAP + i * (BAR_W + GAP);
  }

  function fmtMins(m: number): string {
    if (m <= 0) return "0m";
    const h = Math.floor(m / 60);
    const min = m % 60;
    if (h === 0) return `${min}m`;
    if (min === 0) return `${h}h`;
    return `${h}h ${min}m`;
  }

  let maxVal = $derived(
    data ? Math.max(1, data.tracked_total_mins, data.estimated_total_mins) : 1,
  );

  function barH(mins: number): number {
    return Math.max(2, (mins / maxVal) * BAR_AREA_H);
  }

  function barY(mins: number): number {
    return BAR_AREA_BOTTOM - barH(mins);
  }

  // Whether tracked minutes exceed the estimate (over-budget indicator).
  let isOver = $derived(
    data ? data.estimated_total_mins > 0 && data.tracked_total_mins > data.estimated_total_mins : false,
  );

  let trackedPct = $derived(
    data && data.estimated_total_mins > 0
      ? Math.round((data.tracked_total_mins / data.estimated_total_mins) * 100)
      : null,
  );

  // EST bar geometry (right, index 1).
  let estX = $derived(barX(1));
  let estMins = $derived(data ? data.estimated_total_mins : 0);
  let estH = $derived(barH(estMins));
  let estY = $derived(barY(estMins));

  // TRACKED bar geometry (left, index 0).
  let trkX = $derived(barX(0));
  let trkMins = $derived(data ? data.tracked_total_mins : 0);

  // Overflow bar: the excess tracked portion above the estimate bar ceiling.
  let overflowBarH = $derived(
    isOver && data
      ? Math.max(
          2,
          ((data.tracked_total_mins - data.estimated_total_mins) / maxVal) * BAR_AREA_H,
        )
      : 0,
  );
  // Tracked bar height is clamped to full area height when over-budget.
  let trackedBarH = $derived(data ? Math.min(barH(data.tracked_total_mins), BAR_AREA_H) : 0);
  let trackedBarY = $derived(BAR_AREA_BOTTOM - trackedBarH);

  let SVG_W = $derived(Math.max(160, clientW));
  let SVG_H = $derived((SVG_W / VB_W) * VB_H);
</script>

<div
  class="effort-balance"
  bind:clientWidth={clientW}
  bind:clientHeight={clientH}
  style="--project-accent: {projectColor}"
>
  {#if loading}
    <div class="skeleton">
      <div class="sk-bars">
        <div class="sk-bar" style="height:55px"></div>
        <div class="sk-bar" style="height:70px; animation-delay:0.15s"></div>
      </div>
      <div class="sk-labels">
        <div class="sk-label"></div>
        <div class="sk-label"></div>
      </div>
    </div>
  {:else if error}
    <p class="err">{error}</p>
  {:else if data}
    <svg
      width={SVG_W}
      height={SVG_H}
      viewBox="0 0 {VB_W} {VB_H}"
      xmlns="http://www.w3.org/2000/svg"
      role="img"
      aria-label="Effort balance: {fmtMins(data.tracked_total_mins)} tracked of {fmtMins(data.estimated_total_mins)} estimated"
    >
      <!-- Header: tracked percentage -->
      {#if trackedPct !== null}
        <text
          x={VB_W / 2}
          y={Math.round(BAR_AREA_TOP * 0.55)}
          text-anchor="middle"
          dominant-baseline="middle"
          font-size={FS_HEADER}
          font-weight="700"
          fill={isOver ? "#ef4444" : "var(--db-ink-muted)"}
          font-family="inherit"
          letter-spacing="0.05em"
        >
          {trackedPct}% TRACKED{isOver ? " — OVER" : ""}
        </text>
      {/if}

      <!-- Background tracks -->
      {#each [0, 1] as i (i)}
        <rect
          x={barX(i)}
          y={BAR_AREA_TOP}
          width={BAR_W}
          height={BAR_AREA_H}
          rx="5"
          fill="var(--db-border)"
          opacity="0.4"
        />
      {/each}

      <!-- EST filled bar -->
      {#if estMins > 0}
        <rect
          x={estX}
          y={estY}
          width={BAR_W}
          height={estH}
          rx="5"
          fill={projectColor}
          opacity="0.85"
        />
      {/if}

      <!-- TRACKED filled bar (capped at BAR_AREA_H) -->
      {#if trkMins > 0}
        <rect
          x={trkX}
          y={trackedBarY}
          width={BAR_W}
          height={trackedBarH}
          rx="5"
          fill="#3b82f6"
          opacity="0.85"
        />
      {/if}

      <!-- Overflow indicator: extra red section above the tracked bar when over-budget -->
      {#if isOver && overflowBarH > 0}
        <rect
          x={trkX}
          y={BAR_AREA_TOP - overflowBarH}
          width={BAR_W}
          height={overflowBarH}
          rx="5"
          fill="#ef4444"
          opacity="0.9"
        />
        <!-- Overflow arrow indicator at top of bar -->
        <text
          x={trkX + BAR_W / 2}
          y={BAR_AREA_TOP - overflowBarH - 3}
          text-anchor="middle"
          dominant-baseline="auto"
          font-size={FS_VALUE}
          font-weight="800"
          fill="#ef4444"
          font-family="inherit"
        >
          ↑
        </text>
      {/if}

      <!-- Value labels above bars -->
      {#if trkMins > 0}
        <text
          x={trkX + BAR_W / 2}
          y={trackedBarY - VALUE_OFFSET}
          text-anchor="middle"
          dominant-baseline="auto"
          font-size={FS_VALUE}
          font-weight="700"
          fill={isOver ? "#ef4444" : "#3b82f6"}
          font-family="inherit"
        >
          {fmtMins(trkMins)}
        </text>
      {/if}

      {#if estMins > 0}
        <text
          x={estX + BAR_W / 2}
          y={estY - VALUE_OFFSET}
          text-anchor="middle"
          dominant-baseline="auto"
          font-size={FS_VALUE}
          font-weight="700"
          fill={projectColor}
          font-family="inherit"
        >
          {fmtMins(estMins)}
        </text>
      {/if}

      <!-- Category labels below bars -->
      <text
        x={trkX + BAR_W / 2}
        y={LABEL_Y}
        text-anchor="middle"
        dominant-baseline="hanging"
        font-size={FS_LABEL}
        font-weight="700"
        fill="var(--db-ink-muted)"
        font-family="inherit"
        letter-spacing="0.04em"
      >
        TRACKED
      </text>
      <text
        x={estX + BAR_W / 2}
        y={LABEL_Y}
        text-anchor="middle"
        dominant-baseline="hanging"
        font-size={FS_LABEL}
        font-weight="700"
        fill="var(--db-ink-muted)"
        font-family="inherit"
        letter-spacing="0.04em"
      >
        EST
      </text>

      <!-- Baseline rule -->
      <line
        x1={GAP}
        y1={BAR_AREA_BOTTOM}
        x2={VB_W - GAP}
        y2={BAR_AREA_BOTTOM}
        stroke="var(--db-border)"
        stroke-width="1"
        opacity="0.6"
      />
    </svg>
  {:else}
    <p class="empty">No effort data.</p>
  {/if}
</div>

<style>
  .effort-balance {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  svg {
    display: block;
    flex-shrink: 0;
  }

  .skeleton {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 12px;
    width: 100%;
  }

  .sk-bars {
    display: flex;
    align-items: flex-end;
    gap: 18px;
    height: 80px;
    justify-content: center;
  }

  .sk-bar {
    width: 52px;
    border-radius: 5px;
    background: var(--db-border);
    animation: pulse 1.4s ease-in-out infinite;
  }

  .sk-labels {
    display: flex;
    gap: 18px;
    justify-content: center;
  }

  .sk-label {
    width: 52px;
    height: 8px;
    border-radius: 3px;
    background: var(--db-border);
    animation: pulse 1.4s ease-in-out infinite 0.4s;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 0.8; }
  }

  .err,
  .empty {
    margin: 0;
    font-size: 12px;
    color: var(--db-ink-muted);
    text-align: center;
    padding: 16px;
  }

  .err {
    color: #ef4444;
  }
</style>
