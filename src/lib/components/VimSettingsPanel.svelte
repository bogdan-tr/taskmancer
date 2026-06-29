<script lang="ts">
  import { getErrorMessage } from "$lib/errors";
  import { persistSettings, settingsState } from "$lib/settings.svelte";
  import { DEFAULT_BINDINGS, DEFAULT_STATUS_SHORTCUTS, effectiveCombos, type VimActionId } from "$lib/vimKeybindings";
  import type { VimKeybinding, VimSettings } from "$lib/types";
  import HotkeyRecorder from "$lib/components/HotkeyRecorder.svelte";

  interface ActionEntry {
    id: VimActionId;
    label: string;
  }

  const NAVIGATION_ACTIONS: ActionEntry[] = [
    { id: "move_down", label: "Move down" },
    { id: "move_up", label: "Move up" },
    { id: "move_left", label: "Move left (prev column / day)" },
    { id: "move_right", label: "Move right (next column / day)" },
    { id: "go_to_top", label: "Go to top (gg)" },
    { id: "go_to_bottom", label: "Go to bottom (G)" },
    { id: "next_tab", label: "Next tab" },
    { id: "prev_tab", label: "Previous tab" },
    { id: "next_project", label: "Next project (sidebar)" },
    { id: "prev_project", label: "Previous project (sidebar)" },
    { id: "next_period", label: "Next week / month" },
    { id: "prev_period", label: "Previous week / month" },
    { id: "enter_visual", label: "Visual mode (contiguous)" },
    { id: "enter_sparse", label: "Sparse selection (vv)" },
    { id: "toggle", label: "Toggle (select / expand)" },
    { id: "confirm", label: "Confirm (open / edit)" },
  ];

  const ACTION_KEYS: ActionEntry[] = [
    { id: "edit_task", label: "Edit task" },
    { id: "toggle_timer", label: "Start/stop timer" },
    { id: "new_task", label: "New task" },
    { id: "delete_task", label: "Delete task" },
  ];

  const ALL_ACTIONS = [...NAVIGATION_ACTIONS, ...ACTION_KEYS];

  let draftEnabled = $state(false);
  let draftCombos = $state<Record<string, string[]>>({});
  let draftStatusCombos = $state<Record<string, string[]>>({});
  let initialized = $state(false);
  let isSaving = $state(false);
  let errorMessage = $state("");

  $effect(() => {
    if (settingsState.current && !initialized) {
      const vim = settingsState.current.vim;
      draftEnabled = vim?.enabled ?? false;

      const initCombos: Record<string, string[]> = {};
      for (const action of ALL_ACTIONS) {
        const override = vim?.keybindings.find((b) => b.action_id === action.id);
        initCombos[action.id] = override
          ? [...override.combos]
          : [...(DEFAULT_BINDINGS[action.id] ?? [])];
      }
      draftCombos = initCombos;

      const initStatusCombos: Record<string, string[]> = {};
      for (const status of settingsState.current.statuses) {
        const override = vim?.status_keybindings.find((b) => b.action_id === status.id);
        if (override) {
          initStatusCombos[status.id] = [...override.combos];
        } else {
          const defaultCombo = DEFAULT_STATUS_SHORTCUTS[status.label.toLowerCase()];
          initStatusCombos[status.id] = defaultCombo ? [defaultCombo] : [];
        }
      }
      draftStatusCombos = initStatusCombos;

      initialized = true;
    }
  });

  let isDirty = $derived.by(() => {
    if (!settingsState.current) return false;
    const vim = settingsState.current.vim;

    if ((vim?.enabled ?? false) !== draftEnabled) return true;

    for (const action of ALL_ACTIONS) {
      const saved = effectiveCombos(action.id, vim?.keybindings ?? []);
      const current = draftCombos[action.id] ?? [];
      if (JSON.stringify(saved) !== JSON.stringify(current)) return true;
    }

    for (const status of settingsState.current.statuses) {
      const savedBinding = vim?.status_keybindings.find((b) => b.action_id === status.id);
      const defaultCombo = DEFAULT_STATUS_SHORTCUTS[status.label.toLowerCase()];
      const effective = savedBinding ? savedBinding.combos : (defaultCombo ? [defaultCombo] : []);
      const current = draftStatusCombos[status.id] ?? [];
      if (JSON.stringify(effective) !== JSON.stringify(current)) return true;
    }

    return false;
  });

  /** Map of combo → every label that currently uses it, across all action and status bindings. */
  let comboUsage = $derived.by(() => {
    const map = new Map<string, string[]>();

    function record(combo: string, label: string) {
      const existing = map.get(combo) ?? [];
      map.set(combo, [...existing, label]);
    }

    for (const action of ALL_ACTIONS) {
      for (const combo of draftCombos[action.id] ?? []) {
        record(combo, action.label);
      }
    }
    for (const status of settingsState.current?.statuses ?? []) {
      for (const combo of draftStatusCombos[status.id] ?? []) {
        record(combo, status.label);
      }
    }

    return map;
  });

  function findConflict(combos: string[], ownerLabel: string): string | undefined {
    for (const combo of combos) {
      const labels = comboUsage.get(combo) ?? [];
      const others = labels.filter((l) => l !== ownerLabel);
      if (others.length > 0) return others[0];
    }
    return undefined;
  }

  let hasConflicts = $derived.by(() => {
    for (const action of ALL_ACTIONS) {
      if (findConflict(draftCombos[action.id] ?? [], action.label)) return true;
    }
    for (const status of settingsState.current?.statuses ?? []) {
      if (findConflict(draftStatusCombos[status.id] ?? [], status.label)) return true;
    }
    return false;
  });

  function updateActionCombos(actionId: string, combos: string[]) {
    draftCombos = { ...draftCombos, [actionId]: combos };
  }

  function updateStatusCombos(statusId: string, combos: string[]) {
    draftStatusCombos = { ...draftStatusCombos, [statusId]: combos };
  }

  function discardChanges() {
    initialized = false;
    errorMessage = "";
  }

  async function save() {
    if (!settingsState.current) return;

    const keybindings: VimKeybinding[] = [];
    for (const action of ALL_ACTIONS) {
      const combos = draftCombos[action.id] ?? [];
      const defaultCombos = DEFAULT_BINDINGS[action.id] ?? [];
      if (JSON.stringify(combos) !== JSON.stringify(defaultCombos)) {
        keybindings.push({ action_id: action.id, combos });
      }
    }

    const statusKeybindings: VimKeybinding[] = [];
    for (const [statusId, combos] of Object.entries(draftStatusCombos)) {
      const status = settingsState.current.statuses.find((s) => s.id === statusId);
      const defaultCombo = status ? (DEFAULT_STATUS_SHORTCUTS[status.label.toLowerCase()] ?? null) : null;
      const defaultCombos = defaultCombo ? [defaultCombo] : [];
      // Only persist when the user has diverged from the built-in default
      if (combos.length > 0 && JSON.stringify(combos) !== JSON.stringify(defaultCombos)) {
        statusKeybindings.push({ action_id: statusId, combos });
      }
    }

    const newVim: VimSettings = {
      enabled: draftEnabled,
      keybindings,
      status_keybindings: statusKeybindings,
    };

    isSaving = true;
    try {
      await persistSettings({ ...settingsState.current, vim: newVim });
      errorMessage = "";
    } catch (error) {
      errorMessage = getErrorMessage(error, "Failed to save vim settings");
    } finally {
      isSaving = false;
    }
  }
</script>

<section aria-labelledby="vim-enable-heading">
  <div class="section-header">
    <h2 id="vim-enable-heading">Vim Mode</h2>
  </div>

  {#if !settingsState.current}
    <p class="loading">Loading…</p>
  {:else}
    <label class="toggle-row">
      <input
        type="checkbox"
        checked={draftEnabled}
        onchange={(e) => (draftEnabled = (e.currentTarget as HTMLInputElement).checked)}
        disabled={isSaving}
      />
      <span class="toggle-text">
        <span class="toggle-label">Enable Vim Navigation</span>
        <span class="toggle-description">
          Use hjkl and other vim-style bindings to navigate tasks and columns without touching
          the mouse.
        </span>
      </span>
    </label>

    <div class="keybinding-sections" class:dimmed={!draftEnabled}>
      <section aria-labelledby="vim-nav-heading">
        <div class="section-header subsection-header">
          <h2 id="vim-nav-heading">Navigation Keys</h2>
        </div>
        <ul class="binding-list">
          {#each NAVIGATION_ACTIONS as action (action.id)}
            {@const combos = draftCombos[action.id] ?? []}
            {@const conflict = findConflict(combos, action.label)}
            <li class="binding-row">
              <span class="binding-label">{action.label}</span>
              <div class="binding-recorder">
                <HotkeyRecorder
                  value={combos}
                  onchange={(c) => updateActionCombos(action.id, c)}
                  conflictsWith={conflict}
                  disabled={isSaving || !draftEnabled}
                />
              </div>
            </li>
          {/each}
        </ul>
      </section>

      <section aria-labelledby="vim-action-heading">
        <div class="section-header subsection-header">
          <h2 id="vim-action-heading">Action Keys</h2>
        </div>
        <ul class="binding-list">
          {#each ACTION_KEYS as action (action.id)}
            {@const combos = draftCombos[action.id] ?? []}
            {@const conflict = findConflict(combos, action.label)}
            <li class="binding-row">
              <span class="binding-label">{action.label}</span>
              <div class="binding-recorder">
                <HotkeyRecorder
                  value={combos}
                  onchange={(c) => updateActionCombos(action.id, c)}
                  conflictsWith={conflict}
                  disabled={isSaving || !draftEnabled}
                />
              </div>
            </li>
          {/each}
        </ul>
      </section>

      {#if (settingsState.current?.statuses ?? []).length > 0}
        <section aria-labelledby="vim-status-heading">
          <div class="section-header subsection-header">
            <h2 id="vim-status-heading">Status Keys</h2>
          </div>
          <ul class="binding-list">
            {#each settingsState.current.statuses as status (status.id)}
              {@const combos = draftStatusCombos[status.id] ?? []}
              {@const conflict = findConflict(combos, status.label)}
              <li class="binding-row">
                <span class="binding-label status-label">
                  <span class="status-dot" style="background: {status.color}"></span>
                  {status.label}
                </span>
                <div class="binding-recorder">
                  <HotkeyRecorder
                    value={combos}
                    onchange={(c) => updateStatusCombos(status.id, c)}
                    conflictsWith={conflict}
                    disabled={isSaving || !draftEnabled}
                  />
                </div>
              </li>
            {/each}
          </ul>
        </section>
      {/if}
    </div>

    {#if errorMessage}
      <p class="error" role="alert">{errorMessage}</p>
    {/if}

    <div class="actions">
      <button type="button" class="secondary" disabled={!isDirty || isSaving} onclick={discardChanges}>
        Discard changes
      </button>
      <button type="button" disabled={!isDirty || isSaving || hasConflicts} onclick={save}>
        {isSaving ? "Saving…" : "Save changes"}
      </button>
    </div>
  {/if}
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

  .subsection-header {
    margin-top: var(--space-xl);
    margin-bottom: var(--space-sm);
  }

  .loading {
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
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

  .keybinding-sections {
    transition: opacity var(--duration-normal, 200ms) ease;
  }

  .keybinding-sections.dimmed {
    opacity: 0.5;
    pointer-events: none;
  }

  .binding-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .binding-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-md);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
  }

  .binding-label {
    flex-shrink: 0;
    min-width: 10rem;
    font-size: var(--text-sm);
    font-weight: 500;
    color: var(--color-ink);
    padding-top: 3px;
  }

  .status-label {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
  }

  .status-dot {
    width: 10px;
    height: 10px;
    border-radius: var(--radius-pill);
    flex-shrink: 0;
  }

  .binding-recorder {
    flex: 1;
    min-width: 0;
  }

  .error {
    margin-top: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    background: var(--color-danger-soft);
    color: var(--color-danger);
    font-weight: 600;
    font-size: var(--text-sm);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-xs);
    margin-top: var(--space-lg);
  }

  .actions button {
    padding: var(--space-sm) var(--space-lg);
    border-radius: var(--radius-md);
    border: none;
    font-weight: 600;
    font-size: var(--text-base);
    cursor: pointer;
    box-shadow: var(--shadow-sm);
    transition:
      background var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo),
      transform var(--duration-fast) var(--ease-out-expo);
  }

  .actions button:not(.secondary) {
    background: var(--color-accent);
    color: var(--color-accent-ink);
  }

  .actions button:not(.secondary):hover:not(:disabled) {
    background: var(--color-accent-hover);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .actions button:disabled {
    background: var(--color-border);
    color: var(--color-ink-muted);
    cursor: not-allowed;
    box-shadow: none;
    transform: none;
  }

  .actions button.secondary {
    background: var(--color-surface);
    color: var(--color-ink);
    border: 1px solid var(--color-border);
    box-shadow: none;
  }

  .actions button.secondary:hover:not(:disabled) {
    background: var(--color-canvas);
  }
</style>
