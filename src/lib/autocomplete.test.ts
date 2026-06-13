import { describe, expect, it } from "vitest";
import {
  applyTagsSuggestion,
  applyTokenSuggestion,
  filterSuggestions,
  findActiveToken,
  splitTagsInput,
} from "./autocomplete";

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

  it("returns undefined for a bare # with no characters yet", () => {
    expect(findActiveToken("buy milk #", 10)).toBeUndefined();
  });

  it("returns undefined when the cursor is not after a token", () => {
    expect(findActiveToken("hello world", 11)).toBeUndefined();
  });

  it("returns undefined when # is part of a larger word", () => {
    expect(findActiveToken("a#b", 3)).toBeUndefined();
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
