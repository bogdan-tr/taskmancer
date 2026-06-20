import { describe, expect, test } from "vitest";
import { computeClampedPopoverPosition } from "./popoverPosition";

const VIEWPORT = { viewportWidth: 1000, viewportHeight: 800 };

describe("computeClampedPopoverPosition", () => {
  test("opens below the anchor when there's room", () => {
    const anchor = { top: 100, bottom: 120, left: 50, right: 150 };
    const popover = { width: 200, height: 100 };

    const position = computeClampedPopoverPosition(anchor, popover, VIEWPORT);

    expect(position).toEqual({ top: 126, left: 50 });
  });

  test("flips above the anchor when below would overflow but above fits", () => {
    const anchor = { top: 700, bottom: 720, left: 50, right: 150 };
    const popover = { width: 200, height: 100 };

    const position = computeClampedPopoverPosition(anchor, popover, VIEWPORT);

    expect(position.top).toBe(700 - 6 - 100);
  });

  test("clamps to the bottom margin when neither side fully fits", () => {
    // Below overflows (76 + 780 > 792) and above doesn't fit either
    // (50 - 6 - 780 is deeply negative, far below the 8px margin), so the
    // final clamp step is what determines the result, not either branch.
    const anchor = { top: 50, bottom: 70, left: 50, right: 150 };
    const popover = { width: 200, height: 780 };

    const position = computeClampedPopoverPosition(anchor, popover, VIEWPORT);

    expect(position.top).toBe(800 - 780 - 8);
  });

  test("clamps to the top margin for an anchor near the very top with a tall popover", () => {
    const anchor = { top: 5, bottom: 20, left: 50, right: 150 };
    const popover = { width: 200, height: 790 };

    const position = computeClampedPopoverPosition(anchor, popover, VIEWPORT);

    expect(position.top).toBe(8);
  });

  test("left-aligns to the anchor's left edge by default", () => {
    const anchor = { top: 100, bottom: 120, left: 500, right: 600 };
    const popover = { width: 150, height: 80 };

    const position = computeClampedPopoverPosition(anchor, popover, VIEWPORT);

    expect(position.left).toBe(500);
  });

  test("right-aligns to the anchor's right edge when rightAlign is set", () => {
    const anchor = { top: 100, bottom: 120, left: 500, right: 600 };
    const popover = { width: 150, height: 80 };

    const position = computeClampedPopoverPosition(anchor, popover, { ...VIEWPORT, rightAlign: true });

    expect(position.left).toBe(600 - 150);
  });

  test("clamps left so the popover never overflows the right edge of the viewport", () => {
    const anchor = { top: 100, bottom: 120, left: 950, right: 990 };
    const popover = { width: 200, height: 80 };

    const position = computeClampedPopoverPosition(anchor, popover, VIEWPORT);

    expect(position.left).toBe(1000 - 200 - 8);
  });

  test("clamps left so the popover never overflows the left edge of the viewport", () => {
    const anchor = { top: 100, bottom: 120, left: -50, right: 10 };
    const popover = { width: 200, height: 80 };

    const position = computeClampedPopoverPosition(anchor, popover, VIEWPORT);

    expect(position.left).toBe(8);
  });

  test("accepts custom margin/gap overrides", () => {
    const anchor = { top: 100, bottom: 120, left: 50, right: 150 };
    const popover = { width: 200, height: 100 };

    const position = computeClampedPopoverPosition(anchor, popover, {
      ...VIEWPORT,
      marginPx: 20,
      gapPx: 12,
    });

    expect(position.top).toBe(132);
  });
});
