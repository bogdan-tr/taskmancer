import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const STORAGE_KEY = "taskmancer:project-tree-expanded";

describe("projectTree.svelte", () => {
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

  describe("isExpanded", () => {
    it("defaults to false for a project never toggled", async () => {
      const { isExpanded } = await import("./projectTree.svelte");

      expect(isExpanded("unset-project")).toBe(false);
    });
  });

  describe("setExpanded", () => {
    it("updates state and persists to localStorage", async () => {
      const { isExpanded, setExpanded } = await import("./projectTree.svelte");

      setExpanded("project-1", true);

      expect(isExpanded("project-1")).toBe(true);
      expect(JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "{}")).toEqual({ "project-1": true });
    });

    it("does not affect other projects' state", async () => {
      const { isExpanded, setExpanded } = await import("./projectTree.svelte");

      setExpanded("project-1", true);
      setExpanded("project-2", false);

      expect(isExpanded("project-1")).toBe(true);
      expect(isExpanded("project-2")).toBe(false);
    });
  });

  describe("toggleExpanded", () => {
    it("flips from the default (false) to true", async () => {
      const { isExpanded, toggleExpanded } = await import("./projectTree.svelte");

      toggleExpanded("project-1");

      expect(isExpanded("project-1")).toBe(true);
    });

    it("flips back to false on a second call", async () => {
      const { isExpanded, toggleExpanded } = await import("./projectTree.svelte");

      toggleExpanded("project-1");
      toggleExpanded("project-1");

      expect(isExpanded("project-1")).toBe(false);
    });
  });

  describe("expandIfUnset", () => {
    it("expands a project with no recorded preference", async () => {
      const { expandIfUnset, isExpanded } = await import("./projectTree.svelte");

      expandIfUnset("project-1");

      expect(isExpanded("project-1")).toBe(true);
    });

    it("does not override an explicit prior collapse", async () => {
      const { expandIfUnset, isExpanded, setExpanded } = await import("./projectTree.svelte");

      setExpanded("project-1", false);
      expandIfUnset("project-1");

      expect(isExpanded("project-1")).toBe(false);
    });

    it("does not override an explicit prior expand", async () => {
      const { expandIfUnset, isExpanded, setExpanded } = await import("./projectTree.svelte");

      setExpanded("project-1", true);
      expandIfUnset("project-1");

      expect(isExpanded("project-1")).toBe(true);
    });
  });

  describe("initProjectTree", () => {
    it("restores previously persisted state", async () => {
      store[STORAGE_KEY] = JSON.stringify({ "project-1": true });
      const { initProjectTree, isExpanded } = await import("./projectTree.svelte");

      initProjectTree();

      expect(isExpanded("project-1")).toBe(true);
    });

    it("defaults to no expanded projects when nothing is stored", async () => {
      const { initProjectTree, isExpanded } = await import("./projectTree.svelte");

      initProjectTree();

      expect(isExpanded("project-1")).toBe(false);
    });

    it("falls back to defaults when localStorage throws", async () => {
      vi.stubGlobal("localStorage", {
        getItem: vi.fn(() => {
          throw new Error("blocked");
        }),
        setItem: vi.fn(),
      });
      const { initProjectTree } = await import("./projectTree.svelte");

      expect(() => initProjectTree()).not.toThrow();
    });
  });
});
