import { describe, expect, test } from "vitest";
import { boardsEqual, effectiveBoardStatuses } from "./projectBoardSettings";
import type { ProjectBoard } from "./types";

describe("boardsEqual", () => {
  test("returns true for two boards with identical statuses and default_status", () => {
    const a: ProjectBoard = { statuses: ["backlog", "done"], default_status: "backlog" };
    const b: ProjectBoard = { statuses: ["backlog", "done"], default_status: "backlog" };

    expect(boardsEqual(a, b)).toBe(true);
  });

  test("returns true for two boards with empty statuses and no default_status", () => {
    const a: ProjectBoard = { statuses: [] };
    const b: ProjectBoard = { statuses: [] };

    expect(boardsEqual(a, b)).toBe(true);
  });

  test("returns false when statuses have different lengths", () => {
    const a: ProjectBoard = { statuses: ["backlog"] };
    const b: ProjectBoard = { statuses: ["backlog", "done"] };

    expect(boardsEqual(a, b)).toBe(false);
  });

  test("returns false when statuses are in a different order", () => {
    const a: ProjectBoard = { statuses: ["backlog", "done"] };
    const b: ProjectBoard = { statuses: ["done", "backlog"] };

    expect(boardsEqual(a, b)).toBe(false);
  });

  test("returns false when default_status differs", () => {
    const a: ProjectBoard = { statuses: ["backlog"], default_status: "backlog" };
    const b: ProjectBoard = { statuses: ["backlog"], default_status: "done" };

    expect(boardsEqual(a, b)).toBe(false);
  });

  test("returns false when only one board has a default_status", () => {
    const a: ProjectBoard = { statuses: ["backlog"], default_status: "backlog" };
    const b: ProjectBoard = { statuses: ["backlog"] };

    expect(boardsEqual(a, b)).toBe(false);
  });
});

describe("effectiveBoardStatuses", () => {
  test("returns board.statuses when the board has been customized", () => {
    const board: ProjectBoard = { statuses: ["backlog", "done"] };

    expect(effectiveBoardStatuses(board, ["backlog", "do", "done"])).toEqual(["backlog", "done"]);
  });

  test("returns allStatusIds when board.statuses is empty", () => {
    const board: ProjectBoard = { statuses: [] };

    expect(effectiveBoardStatuses(board, ["backlog", "do", "done"])).toEqual([
      "backlog",
      "do",
      "done",
    ]);
  });

  test("returns board.statuses even when it's a reordering of allStatusIds", () => {
    const board: ProjectBoard = { statuses: ["done", "backlog"] };

    expect(effectiveBoardStatuses(board, ["backlog", "done"])).toEqual(["done", "backlog"]);
  });
});
