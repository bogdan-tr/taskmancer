import { describe, expect, test } from "vitest";
import { parseMarkdown, toggleCheckboxAt, type Block } from "./markdown";

describe("parseMarkdown — blocks", () => {
  test("returns an empty array for empty input", () => {
    expect(parseMarkdown("")).toEqual([]);
  });

  test("returns an empty array for whitespace-only input", () => {
    expect(parseMarkdown("   \n\n  \n")).toEqual([]);
  });

  test("parses a single plain paragraph", () => {
    const blocks = parseMarkdown("Just some text");
    expect(blocks).toEqual<Block[]>([
      { type: "paragraph", content: [{ type: "text", value: "Just some text" }] },
    ]);
  });

  test("groups consecutive plain lines into one paragraph joined by spaces", () => {
    const blocks = parseMarkdown("line one\nline two");
    expect(blocks).toHaveLength(1);
    expect(blocks[0]).toEqual<Block>({
      type: "paragraph",
      content: [{ type: "text", value: "line one line two" }],
    });
  });

  test("a blank line separates two paragraphs", () => {
    const blocks = parseMarkdown("first\n\nsecond");
    expect(blocks).toHaveLength(2);
    expect(blocks[0].type).toBe("paragraph");
    expect(blocks[1].type).toBe("paragraph");
  });
});

describe("parseMarkdown — headings", () => {
  test.each([
    ["# Title", 1, "Title"],
    ["## Subtitle", 2, "Subtitle"],
    ["### Section", 3, "Section"],
    ["#### Deep", 4, "Deep"],
    ["##### Deeper", 5, "Deeper"],
    ["###### Deepest", 6, "Deepest"],
  ])("parses %s as a level-%i heading", (input, level, text) => {
    const blocks = parseMarkdown(input);
    expect(blocks[0]).toEqual<Block>({
      type: "heading",
      level,
      content: [{ type: "text", value: text }],
    });
  });

  test("a hash with no following space is a plain paragraph, not a heading", () => {
    const blocks = parseMarkdown("#nothashtag");
    expect(blocks[0].type).toBe("paragraph");
  });

  test("seven hashes is not a heading (max level 6)", () => {
    const blocks = parseMarkdown("####### too deep");
    expect(blocks[0].type).toBe("paragraph");
  });
});

describe("parseMarkdown — inline emphasis", () => {
  test("parses bold with double asterisks", () => {
    const blocks = parseMarkdown("a **bold** word");
    expect(blocks[0]).toEqual<Block>({
      type: "paragraph",
      content: [
        { type: "text", value: "a " },
        { type: "bold", value: "bold" },
        { type: "text", value: " word" },
      ],
    });
  });

  test("parses italic with single asterisks", () => {
    const blocks = parseMarkdown("an *italic* word");
    expect(blocks[0]).toEqual<Block>({
      type: "paragraph",
      content: [
        { type: "text", value: "an " },
        { type: "italic", value: "italic" },
        { type: "text", value: " word" },
      ],
    });
  });

  test("parses italic with underscores", () => {
    const blocks = parseMarkdown("an _italic_ word");
    expect(blocks[0]).toEqual<Block>({
      type: "paragraph",
      content: [
        { type: "text", value: "an " },
        { type: "italic", value: "italic" },
        { type: "text", value: " word" },
      ],
    });
  });

  test("bold and italic in the same line", () => {
    const blocks = parseMarkdown("**b** and *i*");
    expect(blocks[0]).toEqual<Block>({
      type: "paragraph",
      content: [
        { type: "bold", value: "b" },
        { type: "text", value: " and " },
        { type: "italic", value: "i" },
      ],
    });
  });

  test("an unmatched single asterisk stays literal", () => {
    const blocks = parseMarkdown("2 * 3 = 6");
    expect(blocks[0]).toEqual<Block>({
      type: "paragraph",
      content: [{ type: "text", value: "2 * 3 = 6" }],
    });
  });

  test("empty bold markers stay literal", () => {
    // Four asterisks are an empty bold marker — must not yield an empty node.
    const blocks = parseMarkdown("****");
    expect(blocks[0]).toEqual<Block>({
      type: "paragraph",
      content: [{ type: "text", value: "****" }],
    });
  });

  test("an unmatched double asterisk stays literal", () => {
    const blocks = parseMarkdown("trailing **");
    expect(blocks[0]).toEqual<Block>({
      type: "paragraph",
      content: [{ type: "text", value: "trailing **" }],
    });
  });

  test("html-like text is preserved literally (no injection, output is plain text)", () => {
    const blocks = parseMarkdown("<script>alert(1)</script>");
    expect(blocks[0]).toEqual<Block>({
      type: "paragraph",
      content: [{ type: "text", value: "<script>alert(1)</script>" }],
    });
  });
});

describe("parseMarkdown — bullet lists", () => {
  test("parses a dash bullet list", () => {
    const blocks = parseMarkdown("- one\n- two");
    expect(blocks[0]).toEqual<Block>({
      type: "bullet_list",
      items: [
        [{ type: "text", value: "one" }],
        [{ type: "text", value: "two" }],
      ],
    });
  });

  test("parses an asterisk bullet list", () => {
    const blocks = parseMarkdown("* a\n* b");
    expect(blocks[0].type).toBe("bullet_list");
    const list = blocks[0] as Extract<Block, { type: "bullet_list" }>;
    expect(list.items).toHaveLength(2);
  });

  test("bullet items support inline emphasis", () => {
    const blocks = parseMarkdown("- a **bold** item");
    const list = blocks[0] as Extract<Block, { type: "bullet_list" }>;
    expect(list.items[0]).toEqual([
      { type: "text", value: "a " },
      { type: "bold", value: "bold" },
      { type: "text", value: " item" },
    ]);
  });
});

describe("parseMarkdown — checklists", () => {
  test("parses unchecked and checked items", () => {
    const blocks = parseMarkdown("- [ ] todo\n- [x] done");
    expect(blocks[0]).toEqual<Block>({
      type: "checklist",
      items: [
        { checked: false, content: [{ type: "text", value: "todo" }], checkboxIndex: 0 },
        { checked: true, content: [{ type: "text", value: "done" }], checkboxIndex: 1 },
      ],
    });
  });

  test("accepts uppercase X as checked", () => {
    const blocks = parseMarkdown("- [X] done");
    const list = blocks[0] as Extract<Block, { type: "checklist" }>;
    expect(list.items[0].checked).toBe(true);
  });

  test("checkboxIndex counts across the whole document in order", () => {
    const raw = ["- [ ] a", "", "para", "", "- [x] b", "- [ ] c"].join("\n");
    const blocks = parseMarkdown(raw);
    const checklists = blocks.filter((b) => b.type === "checklist") as Extract<
      Block,
      { type: "checklist" }
    >[];
    expect(checklists[0].items[0].checkboxIndex).toBe(0);
    expect(checklists[1].items[0].checkboxIndex).toBe(1);
    expect(checklists[1].items[1].checkboxIndex).toBe(2);
  });

  test("a checklist and a bullet list are distinct adjacent blocks", () => {
    const blocks = parseMarkdown("- [ ] task\n- plain bullet");
    expect(blocks).toHaveLength(2);
    expect(blocks[0].type).toBe("checklist");
    expect(blocks[1].type).toBe("bullet_list");
  });
});

describe("parseMarkdown — mixed document", () => {
  test("parses headings, paragraphs, lists, and checklists together", () => {
    const raw = [
      "# Heading",
      "",
      "Some **intro** text.",
      "",
      "- bullet one",
      "- bullet two",
      "",
      "- [ ] open task",
      "- [x] closed task",
    ].join("\n");
    const blocks = parseMarkdown(raw);
    expect(blocks.map((b) => b.type)).toEqual([
      "heading",
      "paragraph",
      "bullet_list",
      "checklist",
    ]);
  });
});

describe("toggleCheckboxAt", () => {
  test("flips an unchecked box to checked", () => {
    const raw = "- [ ] todo";
    expect(toggleCheckboxAt(raw, 0)).toBe("- [x] todo");
  });

  test("flips a checked box to unchecked", () => {
    const raw = "- [x] done";
    expect(toggleCheckboxAt(raw, 0)).toBe("- [ ] done");
  });

  test("only flips the targeted checkbox, leaving others untouched", () => {
    const raw = ["- [ ] a", "- [ ] b", "- [ ] c"].join("\n");
    const result = toggleCheckboxAt(raw, 1);
    expect(result).toBe(["- [ ] a", "- [x] b", "- [ ] c"].join("\n"));
  });

  test("counts checkboxes across non-checkbox lines", () => {
    const raw = ["- [ ] a", "some text", "- [ ] b"].join("\n");
    const result = toggleCheckboxAt(raw, 1);
    expect(result).toBe(["- [ ] a", "some text", "- [x] b"].join("\n"));
  });

  test("returns the input unchanged when the index is out of range", () => {
    const raw = "- [ ] only";
    expect(toggleCheckboxAt(raw, 5)).toBe(raw);
  });

  test("preserves uppercase-X boxes elsewhere when toggling a different one", () => {
    const raw = ["- [X] a", "- [ ] b"].join("\n");
    const result = toggleCheckboxAt(raw, 1);
    expect(result).toBe(["- [X] a", "- [x] b"].join("\n"));
  });

  test("preserves trailing content after the checkbox marker", () => {
    const raw = "- [ ] buy milk #shopping";
    expect(toggleCheckboxAt(raw, 0)).toBe("- [x] buy milk #shopping");
  });
});
