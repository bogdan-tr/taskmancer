import type { Priority } from "./types";

export interface ParsedTaskInput {
  title: string;
  tags: string[];
  project?: string;
  priority?: Priority;
  due?: string;
  scheduled?: string;
}

const PRIORITY_TOKENS: Record<string, Priority> = {
  high: "high",
  h: "high",
  medium: "medium",
  m: "medium",
  low: "low",
  l: "low",
};

const BARE_PRIORITY_WORDS: Record<string, Priority> = {
  high: "high",
  medium: "medium",
  low: "low",
};

const WEEKDAY_TOKENS: Record<string, number> = {
  sunday: 0,
  sun: 0,
  monday: 1,
  mon: 1,
  tuesday: 2,
  tue: 2,
  wednesday: 3,
  wed: 3,
  thursday: 4,
  thu: 4,
  friday: 5,
  fri: 5,
  saturday: 6,
  sat: 6,
};

const ISO_DATE_PATTERN = /^\d{4}-\d{2}-\d{2}$/;

/** Formats a date as `YYYY-MM-DD` using local date components. */
function formatDate(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

/** Returns a new date offset from `date` by `days` days. */
function addDays(date: Date, days: number): Date {
  const result = new Date(date);
  result.setDate(result.getDate() + days);
  return result;
}

/**
 * Resolves a date expression to a `YYYY-MM-DD` date string relative to
 * `now`, or `undefined` if the expression isn't recognized.
 *
 * `YYYY-MM-DD` literals are passed through without checking that the date
 * actually exists on the calendar (e.g. `2026-13-99`); the backend rejects
 * invalid calendar dates when the task is saved.
 *
 * A weekday name resolves to the next occurrence of that day, where "next"
 * includes today: `wednesday` said on a Wednesday resolves to today, not a
 * week from now.
 */
function resolveDateExpression(expression: string, now: Date): string | undefined {
  const lower = expression.toLowerCase();

  if (lower === "today") {
    return formatDate(now);
  }
  if (lower === "tomorrow") {
    return formatDate(addDays(now, 1));
  }
  if (ISO_DATE_PATTERN.test(expression)) {
    return expression;
  }
  if (lower in WEEKDAY_TOKENS) {
    const targetDay = WEEKDAY_TOKENS[lower];
    const daysUntil = (targetDay - now.getDay() + 7) % 7;
    return formatDate(addDays(now, daysUntil));
  }

  return undefined;
}

/**
 * Attempts to resolve a natural-language date phrase starting at
 * `tokens[startIndex]`, for use after a `due`/`sch` keyword.
 *
 * An optional leading "next" is treated as filler and skipped: "next friday"
 * resolves the same as "friday". If no date expression is recognized
 * (including when "next" is the last token), returns `undefined` and
 * consumes nothing, so the caller can leave the original tokens in the title.
 */
function tryResolveDatePhrase(
  tokens: string[],
  startIndex: number,
  now: Date,
): { resolved: string; consumed: number } | undefined {
  let index = startIndex;
  let consumed = 0;

  if (tokens[index]?.toLowerCase() === "next") {
    index += 1;
    consumed += 1;
  }

  const candidate = tokens[index];
  if (candidate === undefined) {
    return undefined;
  }

  const resolved = resolveDateExpression(candidate, now);
  if (!resolved) {
    return undefined;
  }

  return { resolved, consumed: consumed + 1 };
}

/**
 * Parses quick-add syntax out of a free-text task title:
 * - `#tag` adds a tag
 * - `+Project` sets the project (last one wins)
 * - `!high` / `!medium` / `!low` (or `!h`/`!m`/`!l`), or a bare `high` /
 *   `medium` / `low` word, sets the priority
 * - `due:<expr>` or `due <phrase>` sets the due date; `sch:<expr>` or
 *   `sch <phrase>` sets the scheduled date. `<expr>`/`<phrase>` is
 *   `today`, `tomorrow`, `YYYY-MM-DD`, or a weekday name, optionally
 *   preceded by "next" (treated as filler)
 *
 * Tokens that don't match a recognized pattern are left in the title
 * unchanged. The remaining words become the task title, with extra
 * whitespace collapsed.
 */
export function parseTaskInput(input: string, now: Date = new Date()): ParsedTaskInput {
  const tags: string[] = [];
  let project: string | undefined;
  let priority: Priority | undefined;
  let due: string | undefined;
  let scheduled: string | undefined;

  const titleTokens: string[] = [];
  const tokens = input.trim().split(/\s+/).filter((token) => token !== "");

  for (let i = 0; i < tokens.length; i++) {
    const token = tokens[i];
    const lowerToken = token.toLowerCase();

    if (token.startsWith("#") && token.length > 1) {
      tags.push(token.slice(1));
      continue;
    }

    if (token.startsWith("+") && token.length > 1) {
      project = token.slice(1);
      continue;
    }

    if (token.startsWith("!") && token.length > 1) {
      const candidate = PRIORITY_TOKENS[lowerToken.slice(1)];
      if (candidate) {
        priority = candidate;
        continue;
      }
    }

    if (lowerToken in BARE_PRIORITY_WORDS) {
      priority = BARE_PRIORITY_WORDS[lowerToken];
      continue;
    }

    if (lowerToken.startsWith("due:") && token.length > 4) {
      const resolved = resolveDateExpression(token.slice(4), now);
      if (resolved) {
        due = resolved;
        continue;
      }
    }

    if (lowerToken.startsWith("sch:") && token.length > 4) {
      const resolved = resolveDateExpression(token.slice(4), now);
      if (resolved) {
        scheduled = resolved;
        continue;
      }
    }

    if (lowerToken === "due") {
      const phrase = tryResolveDatePhrase(tokens, i + 1, now);
      if (phrase) {
        due = phrase.resolved;
        i += phrase.consumed;
        continue;
      }
    }

    if (lowerToken === "sch") {
      const phrase = tryResolveDatePhrase(tokens, i + 1, now);
      if (phrase) {
        scheduled = phrase.resolved;
        i += phrase.consumed;
        continue;
      }
    }

    titleTokens.push(token);
  }

  return {
    title: titleTokens.join(" "),
    tags: [...new Set(tags)],
    project,
    priority,
    due,
    scheduled,
  };
}
