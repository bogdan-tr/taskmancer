import type { DueRule, RecurrenceFrequency } from "./recurrence";

export interface ParsedTaskInput {
  title: string;
  tags: string[];
  /**
   * The display name of the project this task is filed under, resolved
   * from typed/autocompleted text — see `projectId`. A `/`-separated
   * ancestor path (e.g. "Work/Client A") when the `+Project` token named
   * a specific nested subproject rather than a bare leaf name — see
   * `tryResolveProjectPath`.
   */
  project?: string;
  /**
   * The id of the project this task should actually be saved under, once
   * resolved (the parser itself never sets this — only `AddTaskModal`
   * does, after resolving `project` against the loaded project list — see
   * its `handleSubmit`). Mirrors how `dueRule` is also populated after
   * parsing, not by the parser itself.
   */
  projectId?: string;
  /**
   * The name of an existing task this new task should become a subtask
   * of, from a `sub <phrase>` quick-add token (quoted if multi-word — see
   * `trySubtaskParentName`). The parser only ever extracts the typed
   * name; resolving it against the loaded task list (and ensuring that
   * task's subtask container exists) happens in `AddTaskModal`, the same
   * division of responsibility as `project`/`projectId` above. Takes
   * precedence over `project`/`projectId` when both are present, per the
   * Subtasks design spec.
   */
  subtaskParentName?: string;
  /** The id of a `PriorityLevel` (see `Settings.priorities`). */
  priority?: string;
  /** The id of a `StatusDefinition` (see `Settings.statuses`), from an `@status` quick-add token. */
  status?: string;
  /**
   * An absolute `YYYY-MM-DD` date from a `due:`/`due` quick-add token, or
   * the `"none"` sentinel from `due:na`/`due na` meaning "never due".
   * `undefined` if no due-date token was present, *or* if `dueRule` is set
   * instead (see its own doc comment) — the two are mutually exclusive,
   * never both set.
   */
  due?: string;
  /**
   * A structured due rule from a due phrase that can't resolve to an
   * absolute date during parsing alone — `due in <n> days`, `due
   * <weekday>s`/`due every <weekday>`, `due other <weekday>s`/`due every
   * other <weekday>`, or the `{ kind: "Never" }` form of `due na`/`due:na`
   * (set here *in addition to* `due: "none"` for that one case, since
   * "never due" already has a usable absolute-date-free representation).
   * Resolving `AfterScheduled`/`Weekday` to an actual date requires the
   * task's own resolved scheduled date, which isn't necessarily known yet
   * at the point `due` is parsed (`sch ...` might appear later in the same
   * title) — so resolution is deferred to `resolveTaskPreview`, the same
   * place every other default/fallback resolution already happens.
   */
  dueRule?: DueRule;
  scheduled?: string;
  /** Minutes from an `est <n>h <n>m` quick-add token. `undefined` if no estimate token was present. */
  estimatedMinutes?: number;
  /** From an `every ...` quick-add token. `undefined` if no recurrence token was present. */
  recurrence?: { frequency: RecurrenceFrequency; endDate?: string };
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

/**
 * Whether `tokens` contains an "every ..." token, not immediately following
 * "due", that actually resolves to a real recurrence phrase via
 * `tryResolveRecurrencePhrase` — i.e. one that introduces the task's own
 * recurrence, not a due phrase's own "every" (`due every <weekday>`) and
 * not an ordinary English use of the word ("every time", "every week or
 * so", "every now and then" — none of which form a valid recurrence
 * phrase). Used to gate the weekday form of `tryResolveRecurringDuePhrase`
 * ("recurring tasks only" per design — see its own doc comment), computed
 * once up front since the due token can appear before the recurrence token
 * in the same title, e.g. "gym due mondays every saturday". Actually
 * attempting resolution (rather than just checking for the bare word
 * "every") matters: "Call every day due mondays" is a real recurrence, but
 * "Review every single PR due fridays" is not, and a bare-token check can't
 * tell the two apart — a false positive there would let the weekday due
 * rule consume `due fridays` on what's actually a non-recurring task, and
 * that due information would then be silently dropped, since `createTask`
 * (the non-recurring path) never reads `dueRule`.
 */
function hasStandaloneRecurrenceToken(tokens: string[], now: Date): boolean {
  return tokens.some((token, index) => {
    if (token.toLowerCase() !== "every") return false;
    const precededByDue = index > 0 && tokens[index - 1].toLowerCase() === "due";
    if (precededByDue) return false;
    return tryResolveRecurrencePhrase(tokens, index + 1, now) !== undefined;
  });
}

/**
 * Attempts to resolve a recurring-due-rule phrase starting at
 * `tokens[startIndex]`, for use after a `due` keyword. Unlike
 * `tryResolveDatePhrase`, these forms never resolve to an absolute date
 * here — `in <n> days` is relative to the task's own scheduled date (which
 * might not be parsed yet, e.g. if `sch ...` appears later in the same
 * title), and the weekday forms are recomputed per-occurrence once a
 * series exists. Resolution to an actual date happens later, in
 * `resolveTaskPreview`, once the scheduled date is fully known. Returns
 * `undefined` (consuming nothing) if no recognized form matches.
 *
 * Recognized forms:
 * - `in <n> day(s)` — `{ kind: "AfterScheduled", days: n }`. General
 *   grammar, recognized regardless of `isRecurring`.
 * - `<weekday>s` (plural) or `every <weekday>` — `{ kind: "Weekday",
 *   interval_weeks: 1 }`. A bare singular weekday with neither plural nor
 *   "every" is deliberately *not* matched here — that's
 *   `tryResolveDatePhrase`'s existing "next literal occurrence from today"
 *   phrase, left unchanged so it doesn't silently change meaning. Only
 *   recognized when `isRecurring` — unlike `in <n> days`, a per-occurrence
 *   weekday rule isn't a meaningful concept for a one-off task.
 * - `other <weekday>s` or `every other <weekday>` — same as above with
 *   `interval_weeks: 2`, mirroring "every other <weekday>" on the
 *   scheduling side. Also gated on `isRecurring`.
 */
function tryResolveRecurringDuePhrase(
  tokens: string[],
  startIndex: number,
  isRecurring: boolean,
): { dueRule: DueRule; consumed: number } | undefined {
  let i = startIndex;
  const hasEvery = tokens[i]?.toLowerCase() === "every";
  if (hasEvery) {
    i += 1;
  }

  if (!hasEvery && tokens[i]?.toLowerCase() === "in") {
    const numberToken = tokens[i + 1];
    const unitToken = tokens[i + 2]?.toLowerCase();
    if (
      numberToken !== undefined &&
      BARE_NUMBER_PATTERN.test(numberToken) &&
      (unitToken === "day" || unitToken === "days")
    ) {
      return {
        dueRule: { kind: "AfterScheduled", days: Number(numberToken) },
        consumed: i + 3 - startIndex,
      };
    }
    return undefined;
  }

  if (!isRecurring) {
    return undefined;
  }

  let intervalWeeks = 1;
  if (tokens[i]?.toLowerCase() === "other") {
    intervalWeeks = 2;
    i += 1;
  }

  const weekdayToken = tokens[i];
  if (weekdayToken === undefined) {
    return undefined;
  }
  const lower = stripTrailingComma(weekdayToken.toLowerCase());
  const isPlural = lower.endsWith("s") && lower.slice(0, -1) in WEEKDAY_TOKENS;
  const isSingular = lower in WEEKDAY_TOKENS;

  // A plain singular weekday with neither "every" nor "other" isn't this
  // mechanism at all — leave it for `tryResolveDatePhrase`'s existing
  // absolute-date phrase instead.
  if (!hasEvery && intervalWeeks === 1 && !isPlural) {
    return undefined;
  }
  if (!isPlural && !isSingular) {
    return undefined;
  }

  const weekday = isPlural ? WEEKDAY_TOKENS[lower.slice(0, -1)] : WEEKDAY_TOKENS[lower];
  return {
    dueRule: { kind: "Weekday", weekday, interval_weeks: intervalWeeks },
    consumed: i + 1 - startIndex,
  };
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

/** A day-of-month token that *requires* an ordinal suffix (`4th`, `31st`) — unlike `DAY_TOKEN_PATTERN`, a bare number alone doesn't match, so "every 5 days" (an interval) is never mistaken for "every 5th" (a day-of-month). */
const ORDINAL_DAY_PATTERN = /^([1-9]|[12]\d|3[01])(?:st|nd|rd|th)$/i;

/** Strips a trailing comma, for matching a weekday in a comma-separated list like "monday, wednesday, friday" where the comma sticks to the preceding word during whitespace-only tokenizing. */
function stripTrailingComma(token: string): string {
  return token.endsWith(",") ? token.slice(0, -1) : token;
}

/**
 * Attempts to resolve a recurrence phrase starting at `tokens[startIndex]`,
 * for use after an `every` keyword. Returns `undefined` (consuming nothing)
 * if no recognized form is found, so the caller can leave `every` in the
 * title as an ordinary word.
 *
 * Recognized forms:
 * - `day` — daily.
 * - `other day` — every 2 days.
 * - `<n> days` — every `n` days (`n` must be a whole number greater than 0).
 * - `weekend` — every Saturday and Sunday.
 * - `weekday` — every Monday through Friday.
 * - `<ordinal>` (e.g. `4th`, `31st`) — that day of every month. The ordinal
 *   suffix is required, distinguishing this from `<n> days` above.
 * - `<weekday>` (e.g. `monday`) — every week on that day.
 * - `other <weekday>` — every other week on that day.
 * - `<weekday>[,] <weekday>[,] ...` — every week, on all of those days
 *   (e.g. `monday, wednesday, friday` or `monday wednesday friday`).
 *
 * Any of the above may be followed by `until <date-phrase>` (the same
 * phrase grammar `due`/`sch` accept — see `tryResolveDatePhrase`) to set an
 * end date.
 */
function tryResolveRecurrencePhrase(
  tokens: string[],
  startIndex: number,
  now: Date,
): { frequency: RecurrenceFrequency; endDate?: string; consumed: number } | undefined {
  const first = tokens[startIndex]?.toLowerCase();
  if (first === undefined) {
    return undefined;
  }

  let frequency: RecurrenceFrequency | undefined;
  let consumed = 0;

  if (first === "day") {
    frequency = { kind: "EveryNDays", interval: 1 };
    consumed = 1;
  } else if (first === "weekend") {
    frequency = { kind: "Weekly", weekdays: [0, 6], interval_weeks: 1 };
    consumed = 1;
  } else if (first === "weekday") {
    frequency = { kind: "Weekly", weekdays: [1, 2, 3, 4, 5], interval_weeks: 1 };
    consumed = 1;
  } else if (first === "other") {
    const second = tokens[startIndex + 1]?.toLowerCase();
    const secondWeekday = second !== undefined ? stripTrailingComma(second) : undefined;
    if (second === "day") {
      frequency = { kind: "EveryNDays", interval: 2 };
      consumed = 2;
    } else if (secondWeekday !== undefined && secondWeekday in WEEKDAY_TOKENS) {
      frequency = { kind: "Weekly", weekdays: [WEEKDAY_TOKENS[secondWeekday]], interval_weeks: 2 };
      consumed = 2;
    }
  } else {
    const ordinalMatch = ORDINAL_DAY_PATTERN.exec(first);
    if (ordinalMatch) {
      frequency = { kind: "MonthlyByDay", day: Number(ordinalMatch[1]) };
      consumed = 1;
    } else if (BARE_NUMBER_PATTERN.test(first) && tokens[startIndex + 1]?.toLowerCase() === "days") {
      const interval = Number(first);
      if (interval > 0) {
        frequency = { kind: "EveryNDays", interval };
        consumed = 2;
      }
    } else if (stripTrailingComma(first) in WEEKDAY_TOKENS) {
      const weekdays = [WEEKDAY_TOKENS[stripTrailingComma(first)]];
      let count = 1;
      for (;;) {
        const next = tokens[startIndex + count];
        if (next === undefined) {
          break;
        }
        const candidate = stripTrailingComma(next.toLowerCase());
        if (!(candidate in WEEKDAY_TOKENS)) {
          break;
        }
        const day = WEEKDAY_TOKENS[candidate];
        if (!weekdays.includes(day)) {
          weekdays.push(day);
        }
        count += 1;
      }
      frequency = { kind: "Weekly", weekdays, interval_weeks: 1 };
      consumed = count;
    }
  }

  if (frequency === undefined) {
    return undefined;
  }

  let endDate: string | undefined;
  if (tokens[startIndex + consumed]?.toLowerCase() === "until") {
    const phrase = tryResolveDatePhrase(tokens, startIndex + consumed + 1, now);
    if (phrase) {
      endDate = phrase.resolved;
      consumed += 1 + phrase.consumed;
    }
  }

  return { frequency, endDate, consumed };
}

/**
 * Splits `raw` into path segments on `/`, respecting `"`-quoted segments (a
 * quoted segment may contain `/` and spaces; an unquoted segment may
 * contain neither). Returns `undefined` for anything malformed — an
 * unterminated quote, a stray `"` outside a segment's very start, or an
 * empty segment (e.g. a leading/trailing/doubled `/`) — so the caller can
 * fall back to treating the whole thing as an ordinary bare name instead.
 */
export function parsePathSegments(raw: string): string[] | undefined {
  const segments: string[] = [];
  let current = "";
  let inQuotes = false;
  let closedQuoteThisSegment = false;

  for (const ch of raw) {
    if (ch === '"') {
      if (!inQuotes && current === "" && !closedQuoteThisSegment) {
        inQuotes = true;
      } else if (inQuotes) {
        inQuotes = false;
        closedQuoteThisSegment = true;
      } else {
        return undefined;
      }
      continue;
    }

    if (ch === "/" && !inQuotes) {
      segments.push(current);
      current = "";
      closedQuoteThisSegment = false;
      continue;
    }

    if (!inQuotes && closedQuoteThisSegment) {
      return undefined;
    }
    current += ch;
  }

  if (inQuotes) return undefined;
  segments.push(current);

  return segments.some((segment) => segment === "") ? undefined : segments;
}

/**
 * Starting from `initialText` (already extracted from `tokens[index]` —
 * e.g. with a leading `+` stripped, or the bare token itself), pulls in as
 * many of the caller's subsequent whitespace-split tokens as needed
 * (rejoining them with a single space) until every opened `"` is closed.
 * Shared by `tryResolveProjectPath` (a `/`-path may have a quoted segment
 * spanning multiple tokens) and `trySubtaskParentName` (a quoted task name
 * may itself span multiple tokens) — both need the identical "keep
 * consuming tokens while a quote is open" loop, just starting from
 * differently-shaped initial text.
 *
 * Returns the reassembled text plus how many *extra* tokens (beyond
 * `tokens[index]` itself) were consumed — `0` if `initialText` already had
 * balanced quotes (including none at all). Returns `undefined` if a quote
 * is opened but the input ends before it's closed.
 */
function consumeQuotedPhrase(
  tokens: string[],
  index: number,
  initialText: string,
): { text: string; consumed: number } | undefined {
  let text = initialText;
  let consumed = 0;
  while ((text.match(/"/g)?.length ?? 0) % 2 !== 0) {
    const next = tokens[index + consumed + 1];
    if (next === undefined) return undefined;
    text += ` ${next}`;
    consumed += 1;
  }
  return { text, consumed };
}

/**
 * Attempts to resolve a `+Project` token starting at `tokens[index]` as a
 * `/`-separated ancestor path (e.g. `+Work/ClientA` or, for a segment
 * containing whitespace, `+Work/"Client A"`), for disambiguating same-named
 * subprojects under different parents. A quoted segment may span multiple
 * of the caller's original whitespace-split tokens — handled by
 * `consumeQuotedPhrase` — then the reassembled text is split on unquoted
 * `/` characters via `parsePathSegments`.
 *
 * Returns `undefined` — leaving `parseTaskInput`'s existing single-word
 * `project = token.slice(1)` assignment completely unchanged — whenever
 * there's no real path here at all: a bare `+Project` with no `/`, a
 * malformed quote, or a quote left unterminated through the end of the
 * input.
 */
function tryResolveProjectPath(
  tokens: string[],
  index: number,
): { segments: string[]; consumed: number } | undefined {
  const first = tokens[index];
  if (first === undefined || !first.startsWith("+") || first.length <= 1) return undefined;

  const phrase = consumeQuotedPhrase(tokens, index, first.slice(1));
  if (!phrase) return undefined;

  const segments = parsePathSegments(phrase.text);
  if (!segments || segments.length < 2) return undefined;

  return { segments, consumed: phrase.consumed };
}

/**
 * Strips a single pair of surrounding `"` quotes from `text` if present
 * (`"Fix the bug"` -> `Fix the bug`); returns `text` unchanged if it isn't
 * quoted at all (a single-word name needs no quotes). Returns `undefined`
 * for anything malformed — a quote appearing anywhere other than wrapping
 * the whole string (e.g. a stray `"` in the middle), or an empty result
 * after stripping (`sub ""`).
 */
function stripSurroundingQuotes(text: string): string | undefined {
  const quoteCount = text.match(/"/g)?.length ?? 0;
  if (quoteCount === 0) return text === "" ? undefined : text;
  if (quoteCount === 2 && text.startsWith('"') && text.endsWith('"') && text.length >= 2) {
    const inner = text.slice(1, -1);
    return inner === "" ? undefined : inner;
  }
  return undefined;
}

/**
 * Attempts to resolve a `sub <phrase>` parent-task name starting at
 * `tokens[startIndex]` (the token immediately after the `sub` keyword,
 * mirroring `due`/`sch`/`est`'s own `tokens, i + 1` lookahead convention).
 * A multi-word name must be quoted (`sub "Fix the bug"`) — same convention
 * and reason as `+Project`'s path syntax (see `tryResolveProjectPath`):
 * there's no other way to bound where a free-text task title ends and the
 * rest of the new subtask's own title begins. A single-word name needs no
 * quotes (`sub Refactor`).
 *
 * Returns `undefined` — leaving `sub` as an ordinary word in the title,
 * unchanged — if there's no token here at all, a quote is opened but
 * never closed, or the quoted/unquoted text is otherwise malformed (see
 * `stripSurroundingQuotes`).
 */
function trySubtaskParentName(
  tokens: string[],
  startIndex: number,
): { name: string; consumed: number } | undefined {
  const first = tokens[startIndex];
  if (first === undefined) return undefined;

  const phrase = consumeQuotedPhrase(tokens, startIndex, first);
  if (!phrase) return undefined;

  const name = stripSurroundingQuotes(phrase.text);
  if (name === undefined) return undefined;

  return { name, consumed: phrase.consumed + 1 };
}

/**
 * Parses quick-add syntax out of a free-text task title:
 * - `#tag` adds a tag
 * - `+Project` sets the project (last one wins) — `+Work/"Client A"` (a
 *   `/`-separated path, quoting any segment containing whitespace)
 *   targets a specific nested subproject unambiguously instead of matching
 *   the first same-named project anywhere — see `tryResolveProjectPath`.
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
 * - `every <phrase>` sets a recurrence rule — see
 *   `tryResolveRecurrencePhrase` for the full grammar (`every day`,
 *   `every other day`, `every <n> days`, `every weekend`, `every weekday`,
 *   `every <ordinal>` for a day of the month, `every <weekday>`,
 *   `every other <weekday>`, and `every <weekday>, <weekday>, ...` for
 *   multiple weekdays), optionally followed by `until <date-phrase>` for an
 *   end date.
 * - `sub <phrase>` marks this task as a subtask of an existing task named
 *   `<phrase>` — `sub "Fix the bug"` (quoted) for a multi-word name, or a
 *   bare `sub Refactor` for a single word — see `trySubtaskParentName`.
 *   Resolving the name against the loaded task list happens in
 *   `AddTaskModal`, not here; this only extracts the typed text. Disabled
 *   entirely when `options.disableSubtaskKeyword` is set — see its own
 *   doc comment — in which case `sub` is left as an ordinary word, the
 *   same as any other unrecognized token.
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
  options?: {
    /**
     * Disables `sub <phrase>` recognition, leaving `sub` as a literal word
     * in the title instead of extracting a `subtaskParentName` — set by
     * `AddTaskModal` while the dialog is already scoped to a subtask
     * container's own board (creating a task there already makes it a
     * subtask of that container's owner; a second, different `sub` target
     * would just be confusing, and nesting a subtask under a subtask is
     * the one-level-deep rule's job to prevent, not the parser's).
     */
    disableSubtaskKeyword?: boolean;
  },
): ParsedTaskInput {
  const tags: string[] = [];
  let project: string | undefined;
  let subtaskParentName: string | undefined;
  let priority: string | undefined;
  let status: string | undefined;
  let due: string | undefined;
  let dueRule: DueRule | undefined;
  let scheduled: string | undefined;
  let estimatedMinutes: number | undefined;
  let recurrence: { frequency: RecurrenceFrequency; endDate?: string } | undefined;

  const titleTokens: string[] = [];
  const tokens = input.trim().split(/\s+/).filter((token) => token !== "");
  const isRecurring = hasStandaloneRecurrenceToken(tokens, now);

  for (let i = 0; i < tokens.length; i++) {
    const token = tokens[i];
    const lowerToken = token.toLowerCase();

    if (token.startsWith("#") && token.length > 1) {
      tags.push(token.slice(1));
      continue;
    }

    if (token.startsWith("+") && token.length > 1) {
      const pathMatch = tryResolveProjectPath(tokens, i);
      if (pathMatch) {
        project = pathMatch.segments.join("/");
        i += pathMatch.consumed;
        continue;
      }
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
        dueRule = { kind: "Never" };
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
        dueRule = { kind: "Never" };
        i += 1;
        continue;
      }
      const ruleMatch = tryResolveRecurringDuePhrase(tokens, i + 1, isRecurring);
      if (ruleMatch) {
        dueRule = ruleMatch.dueRule;
        i += ruleMatch.consumed;
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

    if (lowerToken === "every") {
      const phrase = tryResolveRecurrencePhrase(tokens, i + 1, now);
      if (phrase) {
        recurrence = { frequency: phrase.frequency, endDate: phrase.endDate };
        i += phrase.consumed;
        continue;
      }
    }

    if (lowerToken === "sub" && !options?.disableSubtaskKeyword) {
      const match = trySubtaskParentName(tokens, i + 1);
      if (match) {
        subtaskParentName = match.name;
        i += match.consumed;
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
    subtaskParentName,
    priority,
    status,
    due,
    dueRule,
    scheduled,
    estimatedMinutes,
    recurrence,
  };
}
