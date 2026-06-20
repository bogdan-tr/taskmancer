export interface ParsedTaskInput {
  title: string;
  tags: string[];
  project?: string;
  /** The id of a `PriorityLevel` (see `Settings.priorities`). */
  priority?: string;
  /** The id of a `StatusDefinition` (see `Settings.statuses`), from an `@status` quick-add token. */
  status?: string;
  /**
   * An absolute `YYYY-MM-DD` date from a `due:`/`due` quick-add token, or the
   * `"none"` sentinel from `due:na`/`due na` meaning "never due". `undefined`
   * if no due-date token was present.
   */
  due?: string;
  scheduled?: string;
  /** Minutes from an `est <n>h <n>m` quick-add token. `undefined` if no estimate token was present. */
  estimatedMinutes?: number;
}

/** The subset of `PriorityLevel` needed to match quick-add priority tokens. */
export interface KnownPriority {
  id: string;
  label: string;
}

/** The subset of `StatusDefinition` needed to match `@status` quick-add tokens. */
export interface KnownStatus {
  id: string;
  label: string;
}

const PRIORITY_TOKENS: Record<string, string> = {
  high: "high",
  h: "high",
  medium: "medium",
  m: "medium",
  low: "low",
  l: "low",
};

const BARE_PRIORITY_WORDS: Record<string, string> = {
  high: "high",
  medium: "medium",
  low: "low",
};

/**
 * Looks up `word` (already lowercased) against `knownPriorities` by `id` or
 * `label`, case-insensitively. Returns the matching level's `id`, or
 * `undefined` if `knownPriorities` is omitted or no level matches.
 */
function matchKnownPriority(word: string, knownPriorities?: KnownPriority[]): string | undefined {
  return knownPriorities?.find(
    (level) => level.id.toLowerCase() === word || level.label.toLowerCase() === word,
  )?.id;
}

/**
 * Looks up `word` (already lowercased) against `knownStatuses` by `id` or
 * `label`, case-insensitively. Returns the matching status's `id`, or
 * `undefined` if `knownStatuses` is omitted or no status matches. Unlike
 * priority, there's no built-in word list to fall back to — statuses are
 * fully user-defined (see Phase 4), so `@status` only ever resolves through
 * the caller's current `Settings.statuses`.
 */
function matchKnownStatus(word: string, knownStatuses?: KnownStatus[]): string | undefined {
  return knownStatuses?.find(
    (status) => status.id.toLowerCase() === word || status.label.toLowerCase() === word,
  )?.id;
}

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

const MONTH_TOKENS: Record<string, number> = {
  january: 1,
  jan: 1,
  february: 2,
  feb: 2,
  march: 3,
  mar: 3,
  april: 4,
  apr: 4,
  may: 5,
  june: 6,
  jun: 6,
  july: 7,
  jul: 7,
  august: 8,
  aug: 8,
  september: 9,
  sep: 9,
  sept: 9,
  october: 10,
  oct: 10,
  november: 11,
  nov: 11,
  december: 12,
  dec: 12,
};

const ISO_DATE_PATTERN = /^\d{4}-\d{2}-\d{2}$/;

const DAY_TOKEN_PATTERN = /^([1-9]|[12]\d|3[01])(?:st|nd|rd|th)?$/i;

/** Parses a day-of-month token like `11`, `11th`, or `1st`, returning the numeric day. */
function parseDayToken(token: string): number | undefined {
  const match = DAY_TOKEN_PATTERN.exec(token);
  return match ? Number(match[1]) : undefined;
}

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
 * Returns the next occurrence of `targetDay` (0=Sunday..6=Saturday) on or
 * after `now`, where "next" includes today: a weekday matching `now`'s own
 * day returns `now` itself.
 */
function nextWeekdayOccurrence(targetDay: number, now: Date): Date {
  const daysUntil = (targetDay - now.getDay() + 7) % 7;
  return addDays(now, daysUntil);
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
 * week from now. `tryResolveDatePhrase` handles the `next <weekday>` prefix
 * (which skips this occurrence) separately.
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
    return formatDate(nextWeekdayOccurrence(WEEKDAY_TOKENS[lower], now));
  }

  return undefined;
}

/**
 * Attempts to resolve an absolute calendar date of the form `<month> <day>`
 * or `<day> <month>` (month names may be full or abbreviated, e.g.
 * "june"/"jun"; day numbers may carry an ordinal suffix, e.g. "11th"),
 * optionally followed by a 4-digit year.
 *
 * When no year is given, the current year is used, rolling over to next
 * year if that date has already passed (compared at day resolution). An
 * explicit year is used as-is, even if it's in the past. Returns
 * `undefined` (consuming nothing) if the tokens don't form a real calendar
 * date (e.g. "february 30").
 */
function tryResolveAbsoluteDate(
  tokens: string[],
  startIndex: number,
  now: Date,
): { resolved: string; consumed: number } | undefined {
  const first = tokens[startIndex]?.toLowerCase();
  const second = tokens[startIndex + 1]?.toLowerCase();
  if (first === undefined || second === undefined) {
    return undefined;
  }

  let month: number | undefined;
  let day: number | undefined;
  if (first in MONTH_TOKENS) {
    month = MONTH_TOKENS[first];
    day = parseDayToken(second);
  } else if (second in MONTH_TOKENS) {
    month = MONTH_TOKENS[second];
    day = parseDayToken(first);
  }

  // If both tokens happen to be month names (e.g. "may june"), the first
  // branch above wins and `day` stays undefined since "june" isn't a valid
  // day token - falling through to undefined below, as intended.
  if (month === undefined || day === undefined) {
    return undefined;
  }

  let consumed = 2;
  let year = now.getFullYear();
  let yearSpecified = false;
  const yearToken = tokens[startIndex + consumed];
  if (yearToken !== undefined && /^\d{4}$/.test(yearToken)) {
    year = Number(yearToken);
    yearSpecified = true;
    consumed += 1;
  }

  let candidate = new Date(year, month - 1, day);
  if (candidate.getMonth() !== month - 1 || candidate.getDate() !== day) {
    return undefined;
  }

  if (!yearSpecified) {
    const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    if (candidate < today) {
      candidate = new Date(year + 1, month - 1, day);
    }
  }

  return { resolved: formatDate(candidate), consumed };
}

/**
 * Attempts to resolve a natural-language date phrase starting at
 * `tokens[startIndex]`, for use after a `due`/`sch` keyword. Returns
 * `undefined` (consuming nothing) if no date expression is recognized, so
 * the caller can leave the original tokens in the title.
 *
 * Recognized forms:
 * - `today`, `tomorrow`, `YYYY-MM-DD`, or a weekday name — see
 *   `resolveDateExpression`.
 * - `next <weekday>` — *skips* the upcoming occurrence of that weekday and
 *   resolves to the one after, i.e. one week later than bare `<weekday>`
 *   would resolve to. "next" has no special meaning before anything else
 *   (e.g. `next today` / `next 2026-06-20` are not recognized as a phrase).
 * - An absolute `<month> <day>` / `<day> <month>` date, optionally followed
 *   by a 4-digit year — see `tryResolveAbsoluteDate`.
 */
function tryResolveDatePhrase(
  tokens: string[],
  startIndex: number,
  now: Date,
): { resolved: string; consumed: number } | undefined {
  const first = tokens[startIndex];
  if (first === undefined) {
    return undefined;
  }

  if (first.toLowerCase() === "next") {
    const weekday = tokens[startIndex + 1]?.toLowerCase();
    if (weekday !== undefined && weekday in WEEKDAY_TOKENS) {
      const skipped = addDays(nextWeekdayOccurrence(WEEKDAY_TOKENS[weekday], now), 7);
      return { resolved: formatDate(skipped), consumed: 2 };
    }
    return undefined;
  }

  const simple = resolveDateExpression(first, now);
  if (simple) {
    return { resolved: simple, consumed: 1 };
  }

  return tryResolveAbsoluteDate(tokens, startIndex, now);
}

const HOUR_TOKEN_PATTERN = /^(\d+)h$/i;
const MINUTE_TOKEN_PATTERN = /^(\d+)m$/i;
const BARE_NUMBER_PATTERN = /^\d+$/;

/**
 * Attempts to resolve an estimated-time duration phrase starting at
 * `tokens[startIndex]`, for use after an `est` keyword or as a bare token.
 * Returns `undefined` (consuming nothing) if `tokens[startIndex]` isn't
 * itself unit-suffixed (`<num>h` or `<num>m`) — a bare unitless number (e.g.
 * `30`) is never recognized, since it's too ambiguous with ordinary numbers
 * elsewhere in a title.
 *
 * Recognized forms (the `h` token, if present, must come first):
 * - `<num>h` — hours alone.
 * - `<num>h <num>m` — hours and minutes.
 * - `<num>h <num>` — hours, with a trailing bare number defaulting to
 *   minutes once the `h` anchor has established this is a duration.
 * - `<num>m` — minutes alone.
 */
function tryResolveDurationPhrase(
  tokens: string[],
  startIndex: number,
): { minutes: number; consumed: number } | undefined {
  const first = tokens[startIndex];
  if (first === undefined) {
    return undefined;
  }

  const hourMatch = HOUR_TOKEN_PATTERN.exec(first);
  if (hourMatch) {
    let minutes = Number(hourMatch[1]) * 60;
    let consumed = 1;

    const second = tokens[startIndex + 1];
    if (second !== undefined) {
      const minuteMatch = MINUTE_TOKEN_PATTERN.exec(second);
      if (minuteMatch) {
        minutes += Number(minuteMatch[1]);
        consumed = 2;
      } else if (BARE_NUMBER_PATTERN.test(second)) {
        minutes += Number(second);
        consumed = 2;
      }
    }

    return { minutes, consumed };
  }

  const minuteMatch = MINUTE_TOKEN_PATTERN.exec(first);
  if (minuteMatch) {
    return { minutes: Number(minuteMatch[1]), consumed: 1 };
  }

  return undefined;
}

/**
 * Parses quick-add syntax out of a free-text task title:
 * - `#tag` adds a tag
 * - `+Project` sets the project (last one wins)
 * - `!high` / `!medium` / `!low` (or `!h`/`!m`/`!l`), or a bare `high` /
 *   `medium` / `low` word, sets the priority. If `knownPriorities` is given,
 *   `!<id-or-label>` and a bare `<id-or-label>` word also match (case-
 *   insensitively) any currently-defined priority level, checked before the
 *   built-in low/medium/high words and abbreviations.
 * - `due:<expr>` or `due <phrase>` sets the due date; `sch:<expr>` or
 *   `sch <phrase>` sets the scheduled date. `<expr>` (after the colon) is
 *   `today`, `tomorrow`, `YYYY-MM-DD`, or a weekday name. `<phrase>` (after
 *   the bare keyword) additionally supports `next <weekday>` (skips the
 *   upcoming occurrence, resolving one week later than bare `<weekday>`)
 *   and absolute `<month> <day>` / `<day> <month>` dates, optionally
 *   followed by a 4-digit year (e.g. `due june 11`, `due 11th june 2027`)
 * - `due:na` or `due na` sets the due date to the `"none"` sentinel, meaning
 *   the task is never due.
 * - `@status` sets the status (last one wins), matching (case-insensitively)
 *   any currently-defined status's `id` or `label` from `knownStatuses`. An
 *   `@word` that doesn't match a known status is left in the title, since
 *   status ids are fully user-defined — there's no built-in fallback set to
 *   guess against (unlike priority's high/medium/low).
 * - `est <n>h <n>m` (or just `<n>h <n>m` — the `est` keyword is optional)
 *   sets the estimated time. The trailing `m`/its value can be dropped once
 *   an `h` anchors the phrase (`est 1h 30` works the same as `est 1h 30m`),
 *   and either unit alone is recognized on its own (`est 1h`, `est 30m`,
 *   `1h`, `30m`). A bare number with no `h`/`m` suffix at all (e.g. `30`) is
 *   never treated as a duration — too ambiguous with ordinary numbers
 *   elsewhere in a title — even when prefixed with `est`.
 *
 * Tokens that don't match a recognized pattern are left in the title
 * unchanged. The remaining words become the task title, with extra
 * whitespace collapsed.
 */
export function parseTaskInput(
  input: string,
  now: Date = new Date(),
  knownPriorities?: KnownPriority[],
  knownStatuses?: KnownStatus[],
): ParsedTaskInput {
  const tags: string[] = [];
  let project: string | undefined;
  let priority: string | undefined;
  let status: string | undefined;
  let due: string | undefined;
  let scheduled: string | undefined;
  let estimatedMinutes: number | undefined;

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

    if (token.startsWith("@") && token.length > 1) {
      const candidate = matchKnownStatus(lowerToken.slice(1), knownStatuses);
      if (candidate) {
        status = candidate;
        continue;
      }
    }

    if (token.startsWith("!") && token.length > 1) {
      const word = lowerToken.slice(1);
      const candidate = matchKnownPriority(word, knownPriorities) ?? PRIORITY_TOKENS[word];
      if (candidate) {
        priority = candidate;
        continue;
      }
    }

    const bareCandidate =
      matchKnownPriority(lowerToken, knownPriorities) ?? BARE_PRIORITY_WORDS[lowerToken];
    if (bareCandidate) {
      priority = bareCandidate;
      continue;
    }

    if (lowerToken.startsWith("due:") && token.length > 4) {
      const value = token.slice(4);
      if (value.toLowerCase() === "na") {
        due = "none";
        continue;
      }
      const resolved = resolveDateExpression(value, now);
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
      if (tokens[i + 1]?.toLowerCase() === "na") {
        due = "none";
        i += 1;
        continue;
      }
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

    if (lowerToken === "est") {
      const phrase = tryResolveDurationPhrase(tokens, i + 1);
      if (phrase) {
        estimatedMinutes = phrase.minutes;
        i += phrase.consumed;
        continue;
      }
    }

    const bareDuration = tryResolveDurationPhrase(tokens, i);
    if (bareDuration) {
      estimatedMinutes = bareDuration.minutes;
      i += bareDuration.consumed - 1;
      continue;
    }

    titleTokens.push(token);
  }

  return {
    title: titleTokens.join(" "),
    tags: [...new Set(tags)],
    project,
    priority,
    status,
    due,
    scheduled,
    estimatedMinutes,
  };
}
