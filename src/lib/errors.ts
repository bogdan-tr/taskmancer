/**
 * Extracts a human-readable message from a caught value, falling back to
 * `fallback` when none is available.
 *
 * Tauri's `invoke()` rejects with whatever the backend `Result::Err`
 * serializes to — for the `Result<T, String>` every command in this app
 * returns, that's a *plain string*, not a JS `Error` (see
 * `tauri::scripts::core.js`'s `reject(e)`, where `e` is the deserialized
 * error payload as-is). `error instanceof Error` is therefore always
 * `false` for a backend validation failure, so checking for a string first
 * is what actually surfaces messages like "a project named 'X' already
 * exists" instead of a generic fallback.
 */
export function getErrorMessage(error: unknown, fallback: string): string {
  if (typeof error === "string" && error.trim() !== "") return error;
  if (error instanceof Error && error.message.trim() !== "") return error.message;
  return fallback;
}
