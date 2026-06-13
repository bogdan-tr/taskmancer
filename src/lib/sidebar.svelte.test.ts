import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const STORAGE_KEY = "taskmancer:sidebar-collapsed";

describe("sidebar.svelte", () => {
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

  it("setSidebarCollapsed updates reactive state and storage", async () => {
    const { sidebarState, setSidebarCollapsed } = await import("./sidebar.svelte");

    setSidebarCollapsed(true);

    expect(sidebarState.collapsed).toBe(true);
    expect(localStorage.getItem(STORAGE_KEY)).toBe("true");
  });

  it("toggleSidebar flips the collapsed state", async () => {
    const { sidebarState, toggleSidebar } = await import("./sidebar.svelte");

    expect(sidebarState.collapsed).toBe(false);
    toggleSidebar();
    expect(sidebarState.collapsed).toBe(true);
    toggleSidebar();
    expect(sidebarState.collapsed).toBe(false);
  });

  it("initSidebar restores a previously persisted collapsed state", async () => {
    store[STORAGE_KEY] = "true";
    const { sidebarState, initSidebar } = await import("./sidebar.svelte");

    initSidebar();

    expect(sidebarState.collapsed).toBe(true);
  });

  it("initSidebar defaults to expanded when storage is empty", async () => {
    const { sidebarState, initSidebar } = await import("./sidebar.svelte");

    initSidebar();

    expect(sidebarState.collapsed).toBe(false);
  });

  it("setSidebarCollapsed and initSidebar do not throw when storage access fails", async () => {
    vi.stubGlobal("localStorage", {
      getItem: vi.fn(() => {
        throw new Error("storage disabled");
      }),
      setItem: vi.fn(() => {
        throw new Error("storage disabled");
      }),
    });
    const { sidebarState, setSidebarCollapsed, initSidebar } = await import("./sidebar.svelte");

    expect(() => setSidebarCollapsed(true)).not.toThrow();
    expect(sidebarState.collapsed).toBe(true);

    expect(() => initSidebar()).not.toThrow();
    expect(sidebarState.collapsed).toBe(false);
  });
});
