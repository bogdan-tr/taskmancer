import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_THEME } from "./theme";

const STORAGE_KEY = "taskmancer:theme";

describe("theme.svelte", () => {
  let store: Record<string, string>;

  beforeEach(() => {
    store = {};
    vi.stubGlobal("document", {
      documentElement: { dataset: {} as Record<string, string> },
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

  it("setTheme updates reactive state, the document dataset, and storage", async () => {
    const { themeState, setTheme } = await import("./theme.svelte");

    setTheme("dark");

    expect(themeState.current).toBe("dark");
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(localStorage.getItem(STORAGE_KEY)).toBe("dark");
  });

  it("initTheme restores a previously persisted theme", async () => {
    store[STORAGE_KEY] = "dark-blue";
    const { themeState, initTheme } = await import("./theme.svelte");

    initTheme();

    expect(themeState.current).toBe("dark-blue");
    expect(document.documentElement.dataset.theme).toBe("dark-blue");
  });

  it("initTheme falls back to the default theme when storage is empty", async () => {
    const { themeState, initTheme } = await import("./theme.svelte");

    initTheme();

    expect(themeState.current).toBe(DEFAULT_THEME);
  });

  it("initTheme falls back to the default theme when storage holds an invalid value", async () => {
    store[STORAGE_KEY] = "not-a-real-theme";
    const { themeState, initTheme } = await import("./theme.svelte");

    initTheme();

    expect(themeState.current).toBe(DEFAULT_THEME);
  });

  it("setTheme does not throw when storage access fails", async () => {
    vi.stubGlobal("localStorage", {
      getItem: vi.fn(() => {
        throw new Error("storage disabled");
      }),
      setItem: vi.fn(() => {
        throw new Error("storage disabled");
      }),
    });
    const { themeState, setTheme, initTheme } = await import("./theme.svelte");

    expect(() => setTheme("dark")).not.toThrow();
    expect(themeState.current).toBe("dark");

    expect(() => initTheme()).not.toThrow();
    expect(themeState.current).toBe(DEFAULT_THEME);
  });
});
