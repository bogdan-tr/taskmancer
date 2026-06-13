/** Splits a comma-separated tag string into a normalized list of tags. */
export function parseTags(value: string): string[] {
  return value
    .split(",")
    .map((tag) => tag.trim())
    .filter((tag) => tag.length > 0);
}

/** Joins a list of tags into a comma-separated string for editing. */
export function formatTags(tags: string[]): string {
  return tags.join(", ");
}

/** Converts an empty/whitespace-only string to `undefined` for optional fields. */
export function emptyToUndefined(value: string): string | undefined {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : undefined;
}

const ISO_DATE_PATTERN = /^\d{4}-\d{2}-\d{2}$/;

/** Returns true if `value` is empty/whitespace-only or matches `YYYY-MM-DD`. */
export function isValidOptionalDate(value: string): boolean {
  return value.trim().length === 0 || ISO_DATE_PATTERN.test(value.trim());
}
