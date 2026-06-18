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
