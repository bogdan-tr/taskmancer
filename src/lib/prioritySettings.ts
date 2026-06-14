import type { PriorityLevel } from "./types";

/** Returns `true` if `a` and `b` contain the same levels in the same order. */
export function levelsEqual(a: PriorityLevel[], b: PriorityLevel[]): boolean {
  if (a.length !== b.length) return false;

  return a.every((level, index) => {
    const other = b[index];
    return (
      level.id === other.id &&
      level.label === other.label &&
      level.color === other.color &&
      level.rank === other.rank
    );
  });
}

/** Returns a copy of `levels` with `rank` set to each level's 1-based position. */
export function renumber(levels: PriorityLevel[]): PriorityLevel[] {
  return levels.map((level, index) => ({ ...level, rank: index + 1 }));
}

/**
 * Returns `base` if it isn't already used by `existingIds`, otherwise
 * `${base}-2`, `${base}-3`, etc. until an unused id is found.
 */
export function uniqueId(existingIds: string[], base: string): string {
  if (!existingIds.includes(base)) return base;

  let suffix = 2;
  while (existingIds.includes(`${base}-${suffix}`)) suffix++;
  return `${base}-${suffix}`;
}

/**
 * Returns a human-readable reason `level` can't be deleted, or `undefined`
 * if deletion is allowed.
 */
export function deleteBlockReason(
  level: PriorityLevel,
  levelCount: number,
  defaultPriorityId: string | undefined,
  taskCounts: Record<string, number>,
): string | undefined {
  if (levelCount <= 1) {
    return "At least one priority level is required";
  }

  if (level.id === defaultPriorityId) {
    return "This is the default priority and can't be deleted";
  }

  const count = taskCounts[level.id] ?? 0;
  if (count > 0) {
    return `${count} task${count === 1 ? "" : "s"} use this priority — reassign them first`;
  }

  return undefined;
}
