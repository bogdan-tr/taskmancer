<script lang="ts">
  import { selectValueToOptional } from "$lib/statusTierRuleOverrides";
  import type { PriorityLevel, StatusTierRule } from "$lib/types";

  interface Props {
    /** The tier condition set these 3 inputs edit. */
    rule: StatusTierRule;
    /** Every available priority level, for the "Minimum priority" select's options. */
    priorities: PriorityLevel[];
    /** Called with a full replacement `StatusTierRule` whenever any of the 3 fields changes — this component holds no state of its own, the caller's `rule` prop is the single source of truth. */
    onChange: (rule: StatusTierRule) => void;
    /** Disables every input, e.g. while a parent save is in flight. */
    disabled?: boolean;
    /** Distinguishes this group's `<input>`/`<select>` `id`s when multiple tiers render side by side on the same page (global settings renders 4 of these). */
    idPrefix: string;
  }

  let { rule, priorities, onChange, disabled = false, idPrefix }: Props = $props();

  /** Empty string means unset, matching `due_within_days`'s `number | undefined` — deliberately no `min` attribute since negative values are valid (overdue/due-today thresholds). */
  function handleDueWithinDaysChange(event: Event) {
    const raw = (event.currentTarget as HTMLInputElement).value;
    if (raw.trim() === "") {
      onChange({ ...rule, due_within_days: undefined });
      return;
    }
    const parsed = Number.parseInt(raw, 10);
    if (!Number.isInteger(parsed)) return;
    onChange({ ...rule, due_within_days: parsed });
  }

  function handleMinPriorityChange(event: Event) {
    const raw = (event.currentTarget as HTMLSelectElement).value;
    onChange({ ...rule, min_priority: selectValueToOptional(raw) });
  }

  /** Ignores anything that doesn't parse to a non-negative integer (a typed/pasted value can still bypass the input's own constraints), leaving the field at its current value rather than saving garbage. */
  function handleEstimatedTimeLeftChange(event: Event) {
    const raw = (event.currentTarget as HTMLInputElement).value;
    if (raw.trim() === "") {
      onChange({ ...rule, estimated_time_left_exceeds_minutes: undefined });
      return;
    }
    const parsed = Number.parseInt(raw, 10);
    if (!Number.isInteger(parsed) || parsed < 0) return;
    onChange({ ...rule, estimated_time_left_exceeds_minutes: parsed });
  }
</script>

<div class="tier-rule-fields">
  <div class="field">
    <label for="{idPrefix}-due-within-days">Due within (days)</label>
    <input
      id="{idPrefix}-due-within-days"
      type="number"
      step="1"
      placeholder="Not set"
      value={rule.due_within_days ?? ""}
      onchange={handleDueWithinDaysChange}
      {disabled}
    />
  </div>

  <div class="field">
    <label for="{idPrefix}-min-priority">Minimum priority</label>
    <select id="{idPrefix}-min-priority" value={rule.min_priority ?? ""} onchange={handleMinPriorityChange} {disabled}>
      <option value="">Not set</option>
      {#each priorities as level (level.id)}
        <option value={level.id}>{level.label}</option>
      {/each}
    </select>
  </div>

  <div class="field">
    <label for="{idPrefix}-estimated-time-left">Estimated time left exceeds (minutes)</label>
    <input
      id="{idPrefix}-estimated-time-left"
      type="number"
      min="0"
      step="1"
      placeholder="Not set"
      value={rule.estimated_time_left_exceeds_minutes ?? ""}
      onchange={handleEstimatedTimeLeftChange}
      {disabled}
    />
  </div>
</div>

<style>
  .tier-rule-fields {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-sm);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
    min-width: 9rem;
    flex: 1;
  }

  .field label {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--color-ink-muted);
  }

  .field input,
  .field select {
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
    font-size: var(--text-sm);
    box-shadow: var(--shadow-sm);
  }

  .field input:focus-visible,
  .field select:focus-visible {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-soft);
    outline: none;
  }

  .field input:disabled,
  .field select:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
