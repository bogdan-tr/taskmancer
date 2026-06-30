/**
 * A deliberately small, hand-rolled markdown renderer for task notes.
 *
 * Supports exactly the limited subset the Notes feature promises — headings
 * (`#`..`######`), bullet lists (`-`/`*`), checkboxes (`- [ ]` / `- [x]`),
 * **bold**, and *italic* / _italic_ — and nothing else. It produces a plain
 * token model (`Block[]`) that a Svelte component renders with ordinary
 * `{#each}` / `{#if}` markup. There is no HTML string output and no `@html`
 * anywhere, so untrusted note content can never inject markup — every value
 * lands in the DOM as text. That is the whole reason this is hand-rolled
 * rather than delegated to a markdown library + sanitizer.
 *
 * Emphasis does not nest (a `bold`/`italic` span carries a plain string), and
 * unmatched or empty markers are left as literal text — acceptable for a notes
 * field, and far simpler to reason about than a full CommonMark parser.
 */

export interface TextNode {
  type: "text";
  value: string;
}
export interface BoldNode {
  type: "bold";
  value: string;
}
export interface ItalicNode {
  type: "italic";
  value: string;
}
export type Inline = TextNode | BoldNode | ItalicNode;

export interface HeadingBlock {
  type: "heading";
  level: number;
  content: Inline[];
}
export interface ParagraphBlock {
  type: "paragraph";
  content: Inline[];
}
export interface BulletListBlock {
  type: "bullet_list";
  items: Inline[][];
}
export interface ChecklistItem {
  checked: boolean;
  content: Inline[];
  /** Zero-based position of this checkbox among all checkboxes in the source,
   *  in document order — the key passed to {@link toggleCheckboxAt}. */
  checkboxIndex: number;
}
export interface ChecklistBlock {
  type: "checklist";
  items: ChecklistItem[];
}
export type Block = HeadingBlock | ParagraphBlock | BulletListBlock | ChecklistBlock;

const HEADING_RE = /^(#{1,6}) +(.*)$/;
const CHECKBOX_RE = /^- \[([ xX])\] ?(.*)$/;
const BULLET_RE = /^[-*] +(.*)$/;

/** Parses raw markdown into a flat list of block tokens. */
export function parseMarkdown(raw: string): Block[] {
  const lines = raw.split("\n");
  const blocks: Block[] = [];

  // Running counter so every checkbox gets a stable, document-order index that
  // matches what `toggleCheckboxAt` computes over the same source.
  let checkboxCounter = 0;

  // Buffers for the run of like-kind lines currently being accumulated.
  let paragraphBuf: string[] = [];
  let bulletBuf: string[] = [];
  let checklistBuf: { checked: boolean; text: string }[] = [];

  const flushParagraph = () => {
    if (paragraphBuf.length === 0) return;
    blocks.push({ type: "paragraph", content: parseInline(paragraphBuf.join(" ")) });
    paragraphBuf = [];
  };
  const flushBullets = () => {
    if (bulletBuf.length === 0) return;
    blocks.push({ type: "bullet_list", items: bulletBuf.map(parseInline) });
    bulletBuf = [];
  };
  const flushChecklist = () => {
    if (checklistBuf.length === 0) return;
    blocks.push({
      type: "checklist",
      items: checklistBuf.map((item) => ({
        checked: item.checked,
        content: parseInline(item.text),
        checkboxIndex: checkboxCounter++,
      })),
    });
    checklistBuf = [];
  };
  const flushAll = () => {
    flushParagraph();
    flushBullets();
    flushChecklist();
  };

  for (const line of lines) {
    if (line.trim() === "") {
      flushAll();
      continue;
    }

    const heading = HEADING_RE.exec(line);
    if (heading) {
      flushAll();
      blocks.push({
        type: "heading",
        level: heading[1].length,
        content: parseInline(heading[2]),
      });
      continue;
    }

    const checkbox = CHECKBOX_RE.exec(line);
    if (checkbox) {
      // A checkbox ends any pending paragraph/bullet run, but extends a
      // checklist run.
      flushParagraph();
      flushBullets();
      checklistBuf.push({ checked: checkbox[1].toLowerCase() === "x", text: checkbox[2] });
      continue;
    }

    const bullet = BULLET_RE.exec(line);
    if (bullet) {
      flushParagraph();
      flushChecklist();
      bulletBuf.push(bullet[1]);
      continue;
    }

    // Plain text line: ends any list run, extends a paragraph run.
    flushBullets();
    flushChecklist();
    paragraphBuf.push(line.trim());
  }

  flushAll();
  return blocks;
}

/**
 * Splits a single line of text into inline nodes, resolving `**bold**` first
 * and then `*italic*` / `_italic_` on the remaining plain runs. Unmatched or
 * empty markers are emitted as literal text.
 */
export function parseInline(text: string): Inline[] {
  if (text === "") return [];

  const nodes: Inline[] = [];
  let i = 0;
  let plainStart = 0;

  const pushPlain = (end: number) => {
    if (end > plainStart) {
      nodes.push({ type: "text", value: text.slice(plainStart, end) });
    }
  };

  while (i < text.length) {
    // Bold: **...**  (non-empty, no greedy run past the next closing **)
    if (text.startsWith("**", i)) {
      const close = text.indexOf("**", i + 2);
      if (close > i + 2) {
        pushPlain(i);
        nodes.push({ type: "bold", value: text.slice(i + 2, close) });
        i = close + 2;
        plainStart = i;
        continue;
      }
    }

    // Italic: *...* or _..._  (single marker, non-empty)
    const ch = text[i];
    if (ch === "*" || ch === "_") {
      const close = text.indexOf(ch, i + 1);
      if (close > i + 1) {
        pushPlain(i);
        nodes.push({ type: "italic", value: text.slice(i + 1, close) });
        i = close + 1;
        plainStart = i;
        continue;
      }
    }

    i++;
  }

  pushPlain(text.length);
  return mergeAdjacentText(nodes);
}

/** Coalesces consecutive text nodes so output stays minimal and predictable. */
function mergeAdjacentText(nodes: Inline[]): Inline[] {
  const merged: Inline[] = [];
  for (const node of nodes) {
    const last = merged[merged.length - 1];
    if (node.type === "text" && last && last.type === "text") {
      last.value += node.value;
    } else {
      merged.push(node);
    }
  }
  return merged;
}

/**
 * Returns `raw` with the checkbox at document-order position `checkboxIndex`
 * toggled between `[ ]` and `[x]`. Out-of-range indices return `raw`
 * unchanged. Preserves the rest of the line (and the original casing of every
 * other checkbox) verbatim, so this is safe to use for in-place checkbox
 * toggling without re-serializing the whole note.
 */
export function toggleCheckboxAt(raw: string, checkboxIndex: number): string {
  const lines = raw.split("\n");
  let seen = -1;
  for (let i = 0; i < lines.length; i++) {
    const match = CHECKBOX_RE.exec(lines[i]);
    if (!match) continue;
    seen++;
    if (seen === checkboxIndex) {
      const checked = match[1].toLowerCase() === "x";
      lines[i] = lines[i].replace(/^- \[[ xX]\]/, checked ? "- [ ]" : "- [x]");
      return lines.join("\n");
    }
  }
  return raw;
}
