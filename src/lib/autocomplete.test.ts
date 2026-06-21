import { describe, expect, it } from "vitest";
import {
  applyTagsSuggestion,
  applyTokenSuggestion,
  filterSuggestions,
  findActiveToken,
  preferredSuggestionText,
  projectPathSuggestions,
  splitTagsInput,
} from "./autocomplete";
import type { Project } from "./types";

function project(id: string, name: string, parentId?: string): Project {
  return {
    id,
    name,
    color: "#111111",
    parent_id: parentId,
    order: 1,
    created: "2026-06-11T10:00:00+00:00",
    board: { statuses: [] },
    defaults: { tags: [] },
  };
}

describe("findActiveToken", () => {
  it("finds a #tag token at the end of the string", () => {
    expect(findActiveToken("buy milk #gro", 13)).toEqual({
      prefix: "#",
      text: "gro",
      start: 9,
      end: 13,
    });
  });

  it("finds a +project token at the start of the string", () => {
    expect(findActiveToken("+Pro", 4)).toEqual({
      prefix: "+",
      text: "Pro",
      start: 0,
      end: 4,
    });
  });

  it("finds the token at the cursor even when more text follows it", () => {
    expect(findActiveToken("buy #gro task", 8)).toEqual({
      prefix: "#",
      text: "gro",
      start: 4,
      end: 8,
    });
  });

  it("finds a bare # with no characters yet (browse-all gesture)", () => {
    expect(findActiveToken("buy milk #", 10)).toEqual({
      prefix: "#",
      text: "",
      start: 9,
      end: 10,
    });
  });

  it("finds a bare + with no characters yet", () => {
    expect(findActiveToken("+", 1)).toEqual({
      prefix: "+",
      text: "",
      start: 0,
      end: 1,
    });
  });

  it("returns undefined when the cursor is not after a token", () => {
    expect(findActiveToken("hello world", 11)).toBeUndefined();
  });

  it("returns undefined when # is part of a larger word", () => {
    expect(findActiveToken("a#b", 3)).toBeUndefined();
  });

  it("finds an @status token at the end of the string", () => {
    expect(findActiveToken("Fix bug @d", 10)).toEqual({
      prefix: "@",
      text: "d",
      start: 8,
      end: 10,
    });
  });

  it("finds a !priority token at the end of the string", () => {
    expect(findActiveToken("Fix bug !hi", 11)).toEqual({
      prefix: "!",
      text: "hi",
      start: 8,
      end: 11,
    });
  });

  it("finds a bare ! with no characters yet", () => {
    expect(findActiveToken("Fix bug !", 9)).toEqual({
      prefix: "!",
      text: "",
      start: 8,
      end: 9,
    });
  });

  it("finds a bare @ with no characters yet", () => {
    expect(findActiveToken("Fix bug @", 9)).toEqual({
      prefix: "@",
      text: "",
      start: 8,
      end: 9,
    });
  });

  it("finds a bare @ at the start of the string", () => {
    expect(findActiveToken("@", 1)).toEqual({
      prefix: "@",
      text: "",
      start: 0,
      end: 1,
    });
  });

  it("returns undefined when @ is part of a larger word", () => {
    expect(findActiveToken("a@b", 3)).toBeUndefined();
  });

  it("returns undefined when ! is part of a larger word", () => {
    expect(findActiveToken("a!b", 3)).toBeUndefined();
  });
});

describe("preferredSuggestionText", () => {
  it("returns the label when it has no whitespace", () => {
    expect(preferredSuggestionText("in-progress", "Cancelled")).toBe("Cancelled");
  });

  it("falls back to the id when the label contains whitespace", () => {
    expect(preferredSuggestionText("in-progress", "In Progress")).toBe("in-progress");
  });
});

describe("filterSuggestions", () => {
  it("returns options starting with the prefix, case-insensitively", () => {
    expect(filterSuggestions(["hello", "help", "world"], "he")).toEqual(["hello", "help"]);
  });

  it("matches regardless of casing", () => {
    expect(filterSuggestions(["Hello"], "he")).toEqual(["Hello"]);
  });

  it("dedupes case-insensitive duplicates, keeping the first occurrence", () => {
    expect(filterSuggestions(["hello", "Hello"], "he")).toEqual(["hello"]);
  });

  it("returns an empty list for an empty prefix", () => {
    expect(filterSuggestions(["hello"], "")).toEqual([]);
  });

  it("returns an empty list when nothing matches", () => {
    expect(filterSuggestions(["world"], "he")).toEqual([]);
  });

  it("sorts matches alphabetically and caps at 8", () => {
    const options = ["h8", "h2", "h7", "h1", "h6", "h3", "h5", "h4", "h9"];
    expect(filterSuggestions(options, "h")).toEqual(["h1", "h2", "h3", "h4", "h5", "h6", "h7", "h8"]);
  });
});

describe("applyTokenSuggestion", () => {
  it("replaces the token at the end of the string and adds a trailing space", () => {
    const token = findActiveToken("buy milk #gro", 13)!;
    expect(applyTokenSuggestion("buy milk #gro", token, "groceries")).toEqual({
      value: "buy milk #groceries ",
      cursor: 20,
    });
  });

  it("replaces the token without doubling an existing trailing space", () => {
    const token = findActiveToken("buy #gro task", 8)!;
    expect(applyTokenSuggestion("buy #gro task", token, "groceries")).toEqual({
      value: "buy #groceries task",
      cursor: 14,
    });
  });

  it("preserves the + prefix", () => {
    const token = findActiveToken("+Pro", 4)!;
    expect(applyTokenSuggestion("+Pro", token, "Project")).toEqual({
      value: "+Project ",
      cursor: 9,
    });
  });
});

describe("projectPathSuggestions", () => {
  it("suggests a unique name bare, unquoted", () => {
    const projects = [project("p1", "Vacation")];

    expect(projectPathSuggestions(projects, "Vac")).toEqual(["Vacation"]);
  });

  it("quotes a unique multi-word name", () => {
    const projects = [project("p1", "My Project")];

    expect(projectPathSuggestions(projects, "My")).toEqual(['"My Project"']);
  });

  it("suggests the full disambiguating path for a colliding name", () => {
    const projects = [
      project("work", "Work"),
      project("personal", "Personal"),
      project("hw1", "Homework", "work"),
      project("hw2", "Homework", "personal"),
    ];

    expect(projectPathSuggestions(projects, "Home")).toEqual(["Personal/Homework", "Work/Homework"]);
  });

  it("quotes a multi-word segment within a disambiguating path", () => {
    const projects = [
      project("work", "Work"),
      project("personal", "Personal"),
      project("hw1", "Client A", "work"),
      project("hw2", "Client A", "personal"),
    ];

    expect(projectPathSuggestions(projects, "Client")).toContain('Personal/"Client A"');
  });

  it("browses every project for an empty typed prefix", () => {
    const projects = [project("p1", "Alpha"), project("p2", "Beta")];

    expect(projectPathSuggestions(projects, "")).toEqual(["Alpha", "Beta"]);
  });

  it("matches case-insensitively", () => {
    const projects = [project("p1", "Vacation")];

    expect(projectPathSuggestions(projects, "vac")).toEqual(["Vacation"]);
  });

  it("drills down into a resolved parent's children", () => {
    const projects = [project("work", "Work"), project("a1", "Client A", "work"), project("a2", "Client B", "work")];

    expect(projectPathSuggestions(projects, "Work/Cli")).toEqual(['Work/"Client A"', 'Work/"Client B"']);
  });

  it("drills down through a quoted parent segment", () => {
    const projects = [
      project("client", "Client A"),
      project("p1", "Phase 1", "client"),
      project("p2", "Phase 2", "client"),
    ];

    expect(projectPathSuggestions(projects, '"Client A"/Phase 1')).toEqual(['"Client A"/"Phase 1"']);
  });

  it("suggests nothing when the parent path before the last slash doesn't resolve", () => {
    const projects = [project("work", "Work"), project("a1", "Client A", "work")];

    expect(projectPathSuggestions(projects, "Bogus/Cli")).toEqual([]);
  });

  it("suggests nothing for an unparseable parent path", () => {
    const projects = [project("work", "Work")];

    expect(projectPathSuggestions(projects, 'Work/"Unclosed/Cli')).toEqual([]);
  });
});

describe("splitTagsInput", () => {
  it("treats the whole value as the current tag when there is no comma", () => {
    expect(splitTagsInput("he")).toEqual({ prefix: "", current: "he" });
  });

  it("splits completed tags from the in-progress tag", () => {
    expect(splitTagsInput("urgent, he")).toEqual({ prefix: "urgent, ", current: "he" });
  });

  it("normalizes spacing around the separator", () => {
    expect(splitTagsInput("urgent,he")).toEqual({ prefix: "urgent, ", current: "he" });
  });
});

describe("applyTagsSuggestion", () => {
  it("appends the suggestion as a new completed tag", () => {
    expect(applyTagsSuggestion("", "hello")).toBe("hello, ");
  });

  it("appends after existing completed tags", () => {
    expect(applyTagsSuggestion("urgent, ", "hello")).toBe("urgent, hello, ");
  });
});
