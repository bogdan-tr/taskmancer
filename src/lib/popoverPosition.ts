export interface PopoverAnchorRect {
  top: number;
  bottom: number;
  left: number;
  right: number;
}

export interface PopoverSize {
  width: number;
  height: number;
}

export interface ComputeClampedPopoverPositionOptions {
  /** Right-align the popover's right edge to the anchor's right edge instead of left-aligning. */
  rightAlign?: boolean;
  viewportWidth: number;
  viewportHeight: number;
  /** Minimum distance kept from any viewport edge. Defaults to 8. */
  marginPx?: number;
  /** Gap between the anchor and the popover when there's room. Defaults to 6. */
  gapPx?: number;
}

const DEFAULT_MARGIN_PX = 8;
const DEFAULT_GAP_PX = 6;

/**
 * Computes `position: fixed` pixel coordinates for a popover anchored to
 * `anchor`: tries below the anchor first, flips above if that would
 * overflow the viewport bottom, then — as an unconditional final step —
 * clamps the result into `[marginPx, viewport size - popover size - marginPx]`
 * on both axes. This guarantees the popover is always fully on-screen, even
 * when neither side fully fits (it ends up pinned to the best available
 * position, possibly overlapping the anchor, rather than ever extending past
 * the window edge). Originally proven out in `WeekBarItem.svelte`'s bar
 * popover; extracted here so the calendar-popup date picker can reuse the
 * exact same positioning behavior instead of re-deriving it.
 */
export function computeClampedPopoverPosition(
  anchor: PopoverAnchorRect,
  popover: PopoverSize,
  options: ComputeClampedPopoverPositionOptions,
): { top: number; left: number } {
  const margin = options.marginPx ?? DEFAULT_MARGIN_PX;
  const gap = options.gapPx ?? DEFAULT_GAP_PX;

  let top = anchor.bottom + gap;
  if (top + popover.height > options.viewportHeight - margin) {
    const above = anchor.top - gap - popover.height;
    if (above >= margin) top = above;
  }
  top = Math.max(margin, Math.min(top, options.viewportHeight - popover.height - margin));

  let left = options.rightAlign ? anchor.right - popover.width : anchor.left;
  left = Math.max(margin, Math.min(left, options.viewportWidth - popover.width - margin));

  return { top, left };
}
