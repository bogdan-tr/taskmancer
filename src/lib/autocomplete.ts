import { parsePathSegments } from "./naturalLanguage";
import { childrenOf, findProjectByPath, formatProjectPathToken, projectPath } from "./projectTree";
import type { Project } from "./types";

/** Maximum number of suggestions shown in an autocomplete dropdown. */
export const MAX_SUGGESTIONS = 8;

/**
 * Matches a trailing `#tag`, `+project`, `!priority`, or `@status` token at
 * the end of a string. The text after the prefix may be empty (a bare
 * `#`/`+`/`!`/`@`), so typing just the prefix character immediately offers
 * every option to browse — the caller decides per-prefix whether showing
 * "everything" makes sense for that option list (e.g. tags suppress this
 * once there are too many to usefully browse — see `AddTaskModal`).
 */
const ACTIVE_TOKEN_PATTERN = /(?:^|\s)([#+!@])(\S*)$/;

export interface ActiveToken {
  /** The token's prefix character. */
  prefix: "#" | "+" | "!" | "@";
  /** The token's text after the prefix, e.g. "he" for "#he". */
  text: string;
  /** Index of the prefix character within the source string. */
  start: number;
  /** Index immediately after the token's last character (the cursor position). */
  end: number;
}

/**
 * Finds the `#tag`, `+project`, `!priority`, or `@status` token immediately
 * before `cursor`, for triggering autocomplete in a free-text input. Returns
 * `undefined` if the cursor isn't immediately after such a token.
 */
export function findActiveToken(value: string, cursor: number): ActiveToken | undefined {
  const beforeCursor = value.slice(0, cursor);
  const match = ACTIVE_TOKEN_PATTERN.exec(beforeCursor);
  if (!match) return undefined;

  const [, prefix, text] = match;
  return { prefix: prefix as "#" | "+" | "!" | "@", text, start: cursor - text.length - 1, end: cursor };
}

/**
 * Matches an in-progress `sub <partial-name>` keyword at the end of a
 * string, requiring at least one character of partial text. "sub" is a
 * whole word, not a single special character like `#+!@`, so it needs its
 * own pattern rather than fitting into `ACTIVE_TOKEN_PATTERN` — and unlike
 * those, a bare `sub ` shows nothing rather than browsing every task: the
 * task list can be far larger than the project/priority/status lists the
 * other prefixes browse-all for, the same "too many to usefully browse"
 * judgment `AddTaskModal` already applies to tags past a threshold.
 */
const ACTIVE_SUB_KEYWORD_PATTERN = /(?:^|\s)sub\s+(\S+)$/i;

export interface ActiveSubtaskToken {
  /** The partial parent-task name typed so far. */
  text: string;
  /** Index of the partial text's first character within the source string (after the `sub` keyword and its following space, neither of which get replaced on selection). */
  start: number;
  /** Index immediately after the partial text's last character (the cursor position). */
  end: number;
}

/**
 * Finds an in-progress `sub <partial-name>` keyword immediately before
 * `cursor`, for triggering subtask-parent-name autocomplete. Returns
 * `undefined` if the cursor isn't immediately after such a keyword, or
 * there's no partial text yet.
 */
export function findActiveSubtaskToken(value: string, cursor: number): ActiveSubtaskToken | undefined {
  const beforeCursor = value.slice(0, cursor);
  const match = ACTIVE_SUB_KEYWORD_PATTERN.exec(beforeCursor);
  if (!match) return undefined;

  const [, text] = match;
  return { text, start: cursor - text.length, end: cursor };
}

/**
 * Replaces an `ActiveSubtaskToken`'s partial text (not the `sub` keyword
 * itself, which is left as-is) with `suggestion`, mirroring
 * `applyTokenSuggestion`'s trailing-space behavior.
 */
export function applySubtaskTokenSuggestion(
  value: string,
  token: ActiveSubtaskToken,
  suggestion: string,
): { value: string; cursor: number } {
  const needsTrailingSpace = token.end >= value.length || !/\s/.test(value[token.end]);
  const replacement = `${suggestion}${needsTrailingSpace ? " " : ""}`;
  const newValue = value.slice(0, token.start) + replacement + value.slice(token.end);
  return { value: newValue, cursor: token.start + replacement.length };
}

/**
 * Prefers a human-readable `label` for an autocomplete suggestion, falling
 * back to `id` when the label contains whitespace — a multi-word value can't
 * round-trip through a single bare token like `@label`/`!label`, so offering
 * it would insert text that re-parses as something else (or nothing).
 * Avoids showing a status/priority's leftover auto-generated id (e.g.
 * "new-status") once the user has renamed its label to something real.
 */
export function preferredSuggestionText(id: string, label: string): string {
  return /\s/.test(label) ? id : label;
}

/**
 * Returns up to `MAX_SUGGESTIONS` values from `options` that start with
 * `prefix` (case-insensitive), deduped and sorted alphabetically. Returns an
 * empty list for an empty prefix so suggestions only appear once the user
 * has started typing.
 */
export function filterSuggestions(options: string[], prefix: string): string[] {
  if (prefix === "") return [];

  const lowerPrefix = prefix.toLowerCase();
  const seen = new Set<string>();
  const matches: string[] = [];

  for (const option of options) {
    if (!option.toLowerCase().startsWith(lowerPrefix)) continue;
    const key = option.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    matches.push(option);
  }

  return matches.sort((a, b) => a.localeCompare(b)).slice(0, MAX_SUGGESTIONS);
}

/**
 * Computes ready-to-insert `+Project` autocomplete suggestions for
 * `typedText` (the text already typed after the `+`), disambiguating
 * same-named projects with a quoted `/`-path instead of a bare name. Two
 * modes, chosen by whether `typedText` contains an (unquoted-or-not — any)
 * `/`:
 *
 * - **No `/` yet:** a flat leaf-name search across every project,
 *   regardless of nesting depth — matches today's behavior for the common
 *   case of knowing a project's name without caring where it's nested. A
 *   name that's unique across the whole list suggests its bare name
 *   (quoted only if it contains whitespace); a name shared by more than
 *   one project suggests each one's full, unambiguous path instead.
 * - **A `/` has been typed:** splits on the *last* `/`, parses everything
 *   before it as a path (via `parsePathSegments`, so an already-closed
 *   quoted segment in the parent portion is handled correctly) and
 *   resolves it via `findProjectByPath`. If it resolves, suggests that
 *   project's direct children matching whatever's typed after the last
 *   `/` (a leading unclosed quote on the in-progress leaf is stripped
 *   before matching) — always as the *full* path, since the user has
 *   already committed to path-typing. An unresolved parent path suggests
 *   nothing — there's nothing to suggest children of yet.
 */
export function projectPathSuggestions(projects: Project[], typedText: string): string[] {
  const lastSlash = typedText.lastIndexOf("/");

  if (lastSlash === -1) {
    const lowerTyped = typedText.toLowerCase();
    const nameCounts = new Map<string, number>();
    for (const project of projects) {
      const key = project.name.toLowerCase();
      nameCounts.set(key, (nameCounts.get(key) ?? 0) + 1);
    }

    const inserts = projects
      .filter((project) => project.name.toLowerCase().startsWith(lowerTyped))
      .map((project) => {
        const collides = (nameCounts.get(project.name.toLowerCase()) ?? 0) > 1;
        if (collides) return formatProjectPathToken(projectPath(projects, project.id).split("/"));
        return /\s/.test(project.name) ? `"${project.name}"` : project.name;
      });

    return [...new Set(inserts)].sort((a, b) => a.localeCompare(b)).slice(0, MAX_SUGGESTIONS);
  }

  const parentSegments = parsePathSegments(typedText.slice(0, lastSlash));
  if (!parentSegments) return [];
  const parent = findProjectByPath(projects, parentSegments);
  if (!parent) return [];

  const typedLeaf = typedText.slice(lastSlash + 1).replace(/^"/, "").toLowerCase();
  const inserts = childrenOf(projects, parent.id)
    .filter((project) => project.name.toLowerCase().startsWith(typedLeaf))
    .map((project) => formatProjectPathToken(projectPath(projects, project.id).split("/")));

  return [...new Set(inserts)].sort((a, b) => a.localeCompare(b)).slice(0, MAX_SUGGESTIONS);
}

/**
 * Replaces `token` in `value` with `suggestion` (re-adding the token's
 * prefix character) and returns the updated value plus the cursor position
 * just after the inserted text. A trailing space is appended unless one is
 * already present, so the user can continue typing immediately.
 */
export function applyTokenSuggestion(
  value: string,
  token: ActiveToken,
  suggestion: string,
): { value: string; cursor: number } {
  const needsTrailingSpace = token.end >= value.length || !/\s/.test(value[token.end]);
  const replacement = `${token.prefix}${suggestion}${needsTrailingSpace ? " " : ""}`;
  const newValue = value.slice(0, token.start) + replacement + value.slice(token.end);
  return { value: newValue, cursor: token.start + replacement.length };
}

export interface TagsInputState {
  /** Already-completed tags (and trailing separator), preserved verbatim. */
  prefix: string;
  /** The in-progress tag being typed, trimmed. */
  current: string;
}

/**
 * Splits a comma-separated tags input into the completed portion and the
 * in-progress tag currently being typed (the text after the last comma).
 */
export function splitTagsInput(value: string): TagsInputState {
  const lastComma = value.lastIndexOf(",");
  if (lastComma === -1) {
    return { prefix: "", current: value.trim() };
  }
  return {
    prefix: `${value.slice(0, lastComma + 1)} `,
    current: value.slice(lastComma + 1).trim(),
  };
}

/** Appends `suggestion` to `prefix`, formatted as a new completed tag. */
export function applyTagsSuggestion(prefix: string, suggestion: string): string {
  return `${prefix}${suggestion}, `;
}
