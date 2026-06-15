import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const FONT_SCALE_KEY = "taskmancer:font-scale";
const COLUMN_WIDTH_KEY = "taskmancer:column-width";
const BOARD_WIDTH_KEY = "taskmancer:board-width";
const SHOW_PRIORITY_GROUPS_KEY = "taskmancer:show-priority-groups";
const SHOW_PRIORITY_CHIP_KEY = "taskmancer:show-priority-chip";

describe("displaySettings.svelte", () => {
  let store: Record<string, string>;
  let setPropertyMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    store = {};
    setPropertyMock = vi.fn();
    vi.stubGlobal("document", {
      documentElement: {
        style: {
          fontSize: "",
          setProperty: setPropertyMock,
        },
      },
    });
    vi.stubGlobal("localStorage", {
      getItem: vi.fn((key: string) => (key in store ? store[key] : null)),
      setItem: vi.fn((key: string, value: string) => {
        store[key] = value;
      }),
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.resetModules();
  });

  describe("setFontScale", () => {
    it("updates reactive state, the root font-size, and storage", async () => {
      const { displayState, setFontScale } = await import("./displaySettings.svelte");

      setFontScale(110);

      expect(displayState.fontScale).toBe(110);
      expect(document.documentElement.style.fontSize).toBe("110%");
      expect(localStorage.getItem(FONT_SCALE_KEY)).toBe("110");
    });

    it("clamps values below the minimum", async () => {
      const { displayState, setFontScale, MIN_FONT_SCALE } = await import("./displaySettings.svelte");

      setFontScale(10);

      expect(displayState.fontScale).toBe(MIN_FONT_SCALE);
    });

    it("clamps values above the maximum", async () => {
      const { displayState, setFontScale, MAX_FONT_SCALE } = await import("./displaySettings.svelte");

      setFontScale(1000);

      expect(displayState.fontScale).toBe(MAX_FONT_SCALE);
    });

    it("does not throw when storage access fails", async () => {
      vi.stubGlobal("localStorage", {
        getItem: vi.fn(() => {
          throw new Error("storage disabled");
        }),
        setItem: vi.fn(() => {
          throw new Error("storage disabled");
        }),
      });
      const { displayState, setFontScale } = await import("./displaySettings.svelte");

      expect(() => setFontScale(110)).not.toThrow();
      expect(displayState.fontScale).toBe(110);
    });
  });

  describe("setColumnWidth", () => {
    it("updates reactive state, the --column-width custom property, and storage", async () => {
      const { displayState, setColumnWidth } = await import("./displaySettings.svelte");

      setColumnWidth(300);

      expect(displayState.columnWidth).toBe(300);
      expect(setPropertyMock).toHaveBeenCalledWith("--column-width", "300px");
      expect(localStorage.getItem(COLUMN_WIDTH_KEY)).toBe("300");
    });

    it("clamps values below the minimum", async () => {
      const { displayState, setColumnWidth, MIN_COLUMN_WIDTH } = await import("./displaySettings.svelte");

      setColumnWidth(10);

      expect(displayState.columnWidth).toBe(MIN_COLUMN_WIDTH);
    });

    it("clamps values above the maximum", async () => {
      const { displayState, setColumnWidth, MAX_COLUMN_WIDTH } = await import("./displaySettings.svelte");

      setColumnWidth(10000);

      expect(displayState.columnWidth).toBe(MAX_COLUMN_WIDTH);
    });

    it("does not throw when storage access fails", async () => {
      vi.stubGlobal("localStorage", {
        getItem: vi.fn(() => {
          throw new Error("storage disabled");
        }),
        setItem: vi.fn(() => {
          throw new Error("storage disabled");
        }),
      });
      const { displayState, setColumnWidth } = await import("./displaySettings.svelte");

      expect(() => setColumnWidth(300)).not.toThrow();
      expect(displayState.columnWidth).toBe(300);
    });
  });

  describe("setBoardWidth", () => {
    it("updates reactive state, the --board-width custom property, and storage", async () => {
      const { displayState, setBoardWidth } = await import("./displaySettings.svelte");

      setBoardWidth(1400);

      expect(displayState.boardWidth).toBe(1400);
      expect(setPropertyMock).toHaveBeenCalledWith("--board-width", "1400px");
      expect(localStorage.getItem(BOARD_WIDTH_KEY)).toBe("1400");
    });

    it("clamps values below the minimum", async () => {
      const { displayState, setBoardWidth, MIN_BOARD_WIDTH } = await import("./displaySettings.svelte");

      setBoardWidth(10);

      expect(displayState.boardWidth).toBe(MIN_BOARD_WIDTH);
    });

    it("clamps values above the maximum", async () => {
      const { displayState, setBoardWidth, MAX_BOARD_WIDTH } = await import("./displaySettings.svelte");

      setBoardWidth(10000);

      expect(displayState.boardWidth).toBe(MAX_BOARD_WIDTH);
    });

    it("does not throw when storage access fails", async () => {
      vi.stubGlobal("localStorage", {
        getItem: vi.fn(() => {
          throw new Error("storage disabled");
        }),
        setItem: vi.fn(() => {
          throw new Error("storage disabled");
        }),
      });
      const { displayState, setBoardWidth } = await import("./displaySettings.svelte");

      expect(() => setBoardWidth(1400)).not.toThrow();
      expect(displayState.boardWidth).toBe(1400);
    });
  });

  describe("setShowPriorityGroups", () => {
    it("updates reactive state and storage", async () => {
      const { displayState, setShowPriorityGroups } = await import("./displaySettings.svelte");

      setShowPriorityGroups(false);

      expect(displayState.showPriorityGroups).toBe(false);
      expect(localStorage.getItem(SHOW_PRIORITY_GROUPS_KEY)).toBe("false");

      setShowPriorityGroups(true);

      expect(displayState.showPriorityGroups).toBe(true);
      expect(localStorage.getItem(SHOW_PRIORITY_GROUPS_KEY)).toBe("true");
    });

    it("does not throw when storage access fails", async () => {
      vi.stubGlobal("localStorage", {
        getItem: vi.fn(() => {
          throw new Error("storage disabled");
        }),
        setItem: vi.fn(() => {
          throw new Error("storage disabled");
        }),
      });
      const { displayState, setShowPriorityGroups } = await import("./displaySettings.svelte");

      expect(() => setShowPriorityGroups(false)).not.toThrow();
      expect(displayState.showPriorityGroups).toBe(false);
    });
  });

  describe("setShowPriorityChip", () => {
    it("updates reactive state and storage", async () => {
      const { displayState, setShowPriorityChip } = await import("./displaySettings.svelte");

      setShowPriorityChip(false);

      expect(displayState.showPriorityChip).toBe(false);
      expect(localStorage.getItem(SHOW_PRIORITY_CHIP_KEY)).toBe("false");

      setShowPriorityChip(true);

      expect(displayState.showPriorityChip).toBe(true);
      expect(localStorage.getItem(SHOW_PRIORITY_CHIP_KEY)).toBe("true");
    });

    it("does not throw when storage access fails", async () => {
      vi.stubGlobal("localStorage", {
        getItem: vi.fn(() => {
          throw new Error("storage disabled");
        }),
        setItem: vi.fn(() => {
          throw new Error("storage disabled");
        }),
      });
      const { displayState, setShowPriorityChip } = await import("./displaySettings.svelte");

      expect(() => setShowPriorityChip(false)).not.toThrow();
      expect(displayState.showPriorityChip).toBe(false);
    });
  });

  describe("initDisplay", () => {
    it("restores previously persisted values", async () => {
      store[FONT_SCALE_KEY] = "120";
      store[COLUMN_WIDTH_KEY] = "320";
      store[BOARD_WIDTH_KEY] = "1500";
      store[SHOW_PRIORITY_GROUPS_KEY] = "false";
      store[SHOW_PRIORITY_CHIP_KEY] = "false";
      const { displayState, initDisplay } = await import("./displaySettings.svelte");

      initDisplay();

      expect(displayState.fontScale).toBe(120);
      expect(displayState.columnWidth).toBe(320);
      expect(displayState.boardWidth).toBe(1500);
      expect(displayState.showPriorityGroups).toBe(false);
      expect(displayState.showPriorityChip).toBe(false);
      expect(document.documentElement.style.fontSize).toBe("120%");
      expect(setPropertyMock).toHaveBeenCalledWith("--column-width", "320px");
      expect(setPropertyMock).toHaveBeenCalledWith("--board-width", "1500px");
    });

    it("falls back to defaults when storage is empty", async () => {
      const {
        displayState,
        initDisplay,
        DEFAULT_FONT_SCALE,
        DEFAULT_COLUMN_WIDTH,
        DEFAULT_BOARD_WIDTH,
        DEFAULT_SHOW_PRIORITY_GROUPS,
        DEFAULT_SHOW_PRIORITY_CHIP,
      } = await import("./displaySettings.svelte");

      initDisplay();

      expect(displayState.fontScale).toBe(DEFAULT_FONT_SCALE);
      expect(displayState.columnWidth).toBe(DEFAULT_COLUMN_WIDTH);
      expect(displayState.boardWidth).toBe(DEFAULT_BOARD_WIDTH);
      expect(displayState.showPriorityGroups).toBe(DEFAULT_SHOW_PRIORITY_GROUPS);
      expect(displayState.showPriorityChip).toBe(DEFAULT_SHOW_PRIORITY_CHIP);
    });

    it("falls back to defaults when storage holds non-numeric values", async () => {
      store[FONT_SCALE_KEY] = "not-a-number";
      store[COLUMN_WIDTH_KEY] = "also-not-a-number";
      store[BOARD_WIDTH_KEY] = "not-a-number-either";
      const { displayState, initDisplay, DEFAULT_FONT_SCALE, DEFAULT_COLUMN_WIDTH, DEFAULT_BOARD_WIDTH } =
        await import("./displaySettings.svelte");

      initDisplay();

      expect(displayState.fontScale).toBe(DEFAULT_FONT_SCALE);
      expect(displayState.columnWidth).toBe(DEFAULT_COLUMN_WIDTH);
      expect(displayState.boardWidth).toBe(DEFAULT_BOARD_WIDTH);
    });

    it("clamps out-of-range persisted values", async () => {
      store[FONT_SCALE_KEY] = "5";
      store[COLUMN_WIDTH_KEY] = "999999";
      store[BOARD_WIDTH_KEY] = "999999";
      const { displayState, initDisplay, MIN_FONT_SCALE, MAX_COLUMN_WIDTH, MAX_BOARD_WIDTH } = await import(
        "./displaySettings.svelte"
      );

      initDisplay();

      expect(displayState.fontScale).toBe(MIN_FONT_SCALE);
      expect(displayState.columnWidth).toBe(MAX_COLUMN_WIDTH);
      expect(displayState.boardWidth).toBe(MAX_BOARD_WIDTH);
    });

    it("does not throw when storage access fails", async () => {
      vi.stubGlobal("localStorage", {
        getItem: vi.fn(() => {
          throw new Error("storage disabled");
        }),
        setItem: vi.fn(() => {
          throw new Error("storage disabled");
        }),
      });
      const { displayState, initDisplay, DEFAULT_FONT_SCALE } = await import("./displaySettings.svelte");

      expect(() => initDisplay()).not.toThrow();
      expect(displayState.fontScale).toBe(DEFAULT_FONT_SCALE);
    });
  });
});
