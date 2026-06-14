import { afterEach, describe, expect, it, vi } from "vitest";
import type { Project } from "./types";

const sampleProject: Project = {
  id: "1",
  name: "Inbox",
  color: "#3b82f6",
  order: 1,
  created: "2026-06-11T00:00:00+00:00",
  board: { statuses: [] },
  defaults: { tags: [] },
};

vi.mock("./api", () => ({
  listProjects: vi.fn(),
}));

describe("projects.svelte", () => {
  afterEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
  });

  it("refreshProjects populates state from listProjects", async () => {
    const { listProjects } = await import("./api");
    vi.mocked(listProjects).mockResolvedValue([sampleProject]);
    const { projectsState, refreshProjects } = await import("./projects.svelte");

    await refreshProjects();

    expect(projectsState.items).toEqual([sampleProject]);
  });

  it("refreshProjects preserves the prior list when the request fails", async () => {
    const { listProjects } = await import("./api");
    vi.mocked(listProjects)
      .mockResolvedValueOnce([sampleProject])
      .mockRejectedValueOnce(new Error("network error"));
    const { projectsState, refreshProjects } = await import("./projects.svelte");

    await refreshProjects();
    expect(projectsState.items).toEqual([sampleProject]);

    await expect(refreshProjects()).resolves.toBeUndefined();
    expect(projectsState.items).toEqual([sampleProject]);
  });
});
