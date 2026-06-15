import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const CONFIRM_TASK_DELETION_KEY = "taskmancer:confirm-task-deletion";

describe("generalSettings.svelte", () => {
  let store: Record<string, string>;

  beforeEach(() => {
    store = {};
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

  describe("setConfirmTaskDeletion", () => {
    it("updates reactive state and storage", async () => {
      const { generalState, setConfirmTaskDeletion } = await import("./generalSettings.svelte");

      setConfirmTaskDeletion(false);

      expect(generalState.confirmTaskDeletion).toBe(false);
      expect(localStorage.getItem(CONFIRM_TASK_DELETION_KEY)).toBe("false");

      setConfirmTaskDeletion(true);

      expect(generalState.confirmTaskDeletion).toBe(true);
      expect(localStorage.getItem(CONFIRM_TASK_DELETION_KEY)).toBe("true");
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
      const { generalState, setConfirmTaskDeletion } = await import("./generalSettings.svelte");

      expect(() => setConfirmTaskDeletion(false)).not.toThrow();
      expect(generalState.confirmTaskDeletion).toBe(false);
    });
  });

  describe("initGeneral", () => {
    it("restores a previously persisted value", async () => {
      store[CONFIRM_TASK_DELETION_KEY] = "false";
      const { generalState, initGeneral } = await import("./generalSettings.svelte");

      initGeneral();

      expect(generalState.confirmTaskDeletion).toBe(false);
    });

    it("falls back to the default when storage is empty", async () => {
      const { generalState, initGeneral, DEFAULT_CONFIRM_TASK_DELETION } = await import(
        "./generalSettings.svelte"
      );

      initGeneral();

      expect(generalState.confirmTaskDeletion).toBe(DEFAULT_CONFIRM_TASK_DELETION);
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
      const { generalState, initGeneral, DEFAULT_CONFIRM_TASK_DELETION } = await import(
        "./generalSettings.svelte"
      );

      expect(() => initGeneral()).not.toThrow();
      expect(generalState.confirmTaskDeletion).toBe(DEFAULT_CONFIRM_TASK_DELETION);
    });
  });
});
