# NL `+Project` path disambiguation — Design Spec

**Date:** 2026-06-21
**Status:** Approved — implementation in progress
**Scope:** Extends the existing `+Project` quick-add token to support targeting a specific nested subproject unambiguously by its ancestor path, with matching autocomplete support. This is the follow-up work the Subprojects plan's Task 17 explicitly deferred ("a real, separate piece of work, not a one-line fix"). Ships on its own, before Subtasks.

## Context

Subprojects allow two different projects to share a name as long as they have different parents (e.g. "Homework" under both "CS101" and "Math"). Today, `+Homework` resolves to whichever same-named project appears first in the loaded list — a documented, accepted limitation for raw typing, with the expectation that real disambiguation would come from autocomplete. That follow-up was never built. The user now wants it, plus the ability to type a path directly.

**Current implementation, confirmed by reading the code (not assumed):**
- `naturalLanguage.ts`'s `parseTaskInput` tokenizes the whole input by whitespace (`input.trim().split(/\s+/)`) in one pass, then for a token starting with `+`, does `project = token.slice(1)` — a `+Project` token is exactly one whitespace-delimited word, full stop.
- Other multi-word phrases (`due next friday`, `every monday, wednesday`) already use a lookahead pattern: a resolver function takes `(tokens, startIndex)`, returns `{ ..., consumed: number }`, and the main loop does `i += result.consumed`. This is the established extension point for "this token starts something that might span more tokens."
- `AddTaskModal.svelte`'s `projectNames` list **filters out every project whose name contains whitespace** before it ever reaches autocomplete — multi-word project names aren't even suggested today, let alone typeable.
- `autocomplete.ts`'s `preferredSuggestionText` already falls back to a project's raw `id` (not its name) when the name contains whitespace, specifically because "a multi-word value can't round-trip through a single bare token." This is the existing, narrower version of the exact problem this spec solves.
- `filterSuggestions` dedupes by exact string match, so two same-named projects collapse into one ambiguous suggestion entry today.

## Decisions made (from user Q&A)

- Path syntax uses `/` as the segment separator, root-first (e.g. `Work/Client A`).
- A path segment containing whitespace must be quoted: `+Work/"Client A"`. A single-word segment needs no quotes: `+Work/ClientA`.
- Autocomplete suggestion labels use the same `/`-joined path format — no separate "under X" phrasing.
- Both literal typing and autocomplete selection are supported (not just one).
- Scoped to the `+Project` token only — the `sub <parent task name>` keyword (Subtasks) keeps its existing first-match-by-name behavior, unchanged by this work.
- Ships as its own cycle, before Subtasks.

## Parsing design

Add `tryResolveProjectPath(tokens: string[], index: number): { segments: string[]; consumed: number } | undefined` to `naturalLanguage.ts`, following the exact shape of the existing `tryResolveDatePhrase`/`tryResolveDurationPhrase` lookahead helpers — called with `index` pointing at the `+`-prefixed token itself (not `index + 1`, since the path starts immediately after the `+`, in the same token).

Algorithm: scan `tokens[index].slice(1)` (everything after `+`) plus however many subsequent tokens are needed, splitting on `/` into segments, where each segment is either:
- a bare run with no whitespace and no embedded `"`, or
- a `"`-delimited run that may itself span multiple original whitespace tokens (re-joined with single spaces) until a closing `"` is found.

Returns `undefined` (falls through to today's single-word behavior unchanged) if there's no `/` in the token at all, or if a quote is opened but never closed before the input ends.

In `parseTaskInput`'s main loop, the `+` branch becomes: try the path resolver first; if it returns segments, join them back with `/` (without quotes) into the existing `project` field (e.g. `project = "Work/Client A"`) and advance `i` by `consumed`; otherwise keep today's `project = token.slice(1)` exactly as-is. **No new field on `ParsedTaskInput`** — a `/` in `project` is the signal that downstream resolution should treat it as a path rather than a bare name, keeping the interface change-surface to zero.

## Resolution design

Add `findProjectByPath(projects: Project[], segments: string[]): Project | undefined` to `projectTree.ts`: walks the tree top-down, at each level finding a child (or, for the first segment, a top-level project) whose name matches the current segment case-insensitively — first match wins at each level, mirroring `find_project`'s existing case-insensitivity. Returns `undefined` immediately if any level has no match.

`AddTaskModal.svelte`'s `matchedProject` (already rewritten earlier this session to resolve `+Project`/`projectFilter`/default-project) gains one more branch: if `parsed.project` contains `/`, split it and resolve via `findProjectByPath`; otherwise, resolve by bare name exactly as it does today.

**Resolved:** if a typed path fails to resolve (e.g. a typo'd middle segment), it falls through to the default project — exactly like any unmatched bare name already behaves today. No fallback match on just the last segment; a partially-wrong path should not silently resolve to some unrelated same-named project elsewhere, which would be more confusing than the existing, already-familiar "falls through to default" behavior.

## Autocomplete design

`AddTaskModal.svelte`'s `projectNames` stops filtering out multi-word names (that filter only existed because multi-word names couldn't round-trip through the old bare-token mechanism — quoting now solves that).

Suggestion behavior when the active token's text (after `+`) is being typed:
- **No `/` typed yet:** suggest matching projects by leaf name, exactly as today (flat search across all projects regardless of depth) — *not* requiring the user to drill down level-by-level just to find a project they already know the name of.
- **A `/` has been typed:** split the token text on the *last* `/`; resolve everything before it as a path (via `findProjectByPath`); if it resolves, suggest that project's direct children matching whatever's typed after the last `/`. If the prefix path doesn't resolve (yet), show no suggestions — there's nothing to suggest children of.

Suggestion **insertion** behavior (what text actually gets written into the title field on selection):
- If the selected project's leaf name is unique across the whole project list, insert the bare name — unchanged from today.
- If another project anywhere shares that leaf name, insert the full path instead, quoting any multi-word segment — so the inserted text is unambiguous on its own, not dependent on the dropdown having been used.

This means most everyday use (unique project names) sees zero change in typed text; path-qualification only kicks in where it's actually needed to disambiguate.

## Testing approach

Mirrors this codebase's established convention — pure logic gets direct unit tests:
- `tryResolveProjectPath`: bare name (no `/`, returns `undefined`), single-level path, multi-level path, quoted segment spanning multiple original tokens, unterminated quote (returns `undefined`), segment containing only whitespace inside quotes.
- `findProjectByPath`: single-segment top-level match, multi-level match, no match at an intermediate level, case-insensitive matching, empty `segments` array.
- `filterSuggestions`/insertion-text logic: unique name keeps bare insertion, colliding name produces quoted path insertion, drill-down suggestion scoping to a resolved parent's children.

**Manual/visual (disclosed limitation, consistent with prior entries for this codebase):** the actual typing/autocomplete *feel* (does drill-down feel discoverable, does quoting feel natural) needs a user testing pass after implementation.

## Out of scope

- The `sub <parent task name>` keyword (Subtasks) — explicitly scoped out per this session's Q&A.
- Any change to how `+Project` resolution works inside `KanbanBoard`'s `projectFilter` (id-based, unaffected — this spec is about the *typed quick-add token* only).

## Implementation

Built exactly as designed, with one small extension discovered while implementing the autocomplete piece:

- `naturalLanguage.ts`: added `parsePathSegments` (exported, for reuse) and `tryResolveProjectPath`, wired into `parseTaskInput`'s `+` branch exactly as specced. `ParsedTaskInput.project` is unchanged in shape — just sometimes a `/`-joined path string now.
- `projectTree.ts`: added `findProjectByPath` (level-by-level, case-insensitive, first-match-wins, `undefined` on any miss) and `formatProjectPathToken` (the inverse — quotes any segment containing whitespace, for producing autocomplete insertion text).
- `AddTaskModal.svelte`: `matchedProject` gained a `parsed.project.includes("/")` branch resolving via `findProjectByPath`; falls through to `projectFilter`/default-project on a miss, per the resolved open question. The old `projectNames` (which filtered out every multi-word name) was deleted entirely — superseded by the new autocomplete pipeline below.
- `autocomplete.ts`: added `projectPathSuggestions(projects, typedText)`, handling both flat leaf-name search (no `/` yet) and parent-resolved drill-down (`/` typed) in one function, used directly in `AddTaskModal`'s `updateSuggestions` for the `+` prefix instead of going through the generic `filterSuggestions` pipeline (which can't carry the extra "is this a collision" information needed to decide bare-vs-path insertion text).

**One extension beyond the original design, found necessary during implementation, not before:** a unique (non-colliding) project name that contains whitespace now gets suggested with quotes around it (e.g. `"My Project"`) instead of the old fallback of inserting the project's raw `id` (`preferredSuggestionText`'s pre-existing workaround for `!priority`/`@status`/`#tag`, which doesn't support quoting and still uses the id fallback for those). This wasn't explicitly discussed, but follows directly from "quoting now solves the round-trip problem" — there was no reason to keep showing a raw id in the input once quoting was available as a strictly better alternative.

**Verified:**
- Unit tests: 10 new cases for `tryResolveProjectPath`/`parseTaskInput` (bare token unaffected, single/multi-level paths, quoted segments spanning multiple whitespace tokens, a quoted segment first in the path, unterminated quote falls back to the bare token unchanged, empty segment falls back to the bare token unchanged), 8 new cases for `findProjectByPath` (including an explicit same-name-different-parent disambiguation case), 9 new cases for `projectPathSuggestions` (unique bare, unique-but-quoted, colliding-disambiguated, drill-down, drill-down through a quoted parent segment, unresolved parent in both flat-miss and malformed-quote forms). Full suite: 772 Vitest tests passing (up from 746), `svelte-check` 0 errors, `cargo test` unaffected (375, untouched by this work), both builds succeed.
- Manual/visual, via the established headless-Playwright-against-mocked-IPC harness, with a fixture containing two same-named "Homework" subprojects under different parents: confirmed the flat-collision dropdown shows both disambiguated as `+Personal/Homework` / `+Work/Homework`; confirmed typing `+Work/Cl` drills down to suggest only `+Work/"Client A"`; confirmed selecting a suggestion inserts the full path text and the preview resolves to the matching project; confirmed literally typing a quoted path (`+Work/"Client A"`) resolves correctly in the preview without ever touching the dropdown.
