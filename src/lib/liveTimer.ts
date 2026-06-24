/**
 * Formats a total-seconds value as a live, seconds-precision ticker:
 * `"H:MM:SS"` once an hour or more has elapsed, else `"M:SS"`. Unlike
 * `estimatedTime.ts`'s `formatMinutes` (whole-minute precision for a
 * resting/static display), this is meant to be recomputed every second
 * while a session is actively running. A negative/non-finite input is
 * clamped to zero, mirroring `estimatedTime.ts`'s clamping style.
 */
export function formatHms(totalSeconds: number): string {
  const safeTotal = Number.isFinite(totalSeconds) ? Math.max(0, Math.floor(totalSeconds)) : 0;

  const hours = Math.floor(safeTotal / 3600);
  const minutes = Math.floor((safeTotal % 3600) / 60);
  const seconds = safeTotal % 60;

  const pad = (value: number): string => value.toString().padStart(2, "0");

  if (hours > 0) {
    return `${hours}:${pad(minutes)}:${pad(seconds)}`;
  }
  return `${minutes}:${pad(seconds)}`;
}
