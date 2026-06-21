# Subprojects — Design Spec

**Date:** 2026-06-20
**Status:** Approved by user, ready for implementation planning
**Scope:** Subprojects only. Subtasks (auto-generated subprojects per task) is a separate, follow-on feature that depends on this one and gets its own design/plan/implementation cycle once Subprojects has shipped and been tested.

## Context

`next_instructions.md` asks for subproject capability: a subproject is its own independent project that inherits settings/defaults from its parent, with parent-derived color suggestions, nested sidebar display with persisted expand/collapse state, and ties into a future Subtasks feature. This spec covers Subprojects in isolation.

## Decisions made during brainstorming

- **Nesting depth:** arbitrary (a subproject can itself have subprojects).
- **Task→project link:** switch `Task.project` from a name string to `Task.project_id` (an id). Requires a one-time migration; the user has real data, so the migration must be careful (exact-match by name, log warnings for unmatched tasks rather than silently dropping the link).
- **Color lineage:** shade suggestions derive from the *immediate* parent's color, not the root ancestor's.
- **Color freedom:** parent-derived shades are presented as defaults only; the full existing preset palette and custom hex input remain available.
- **Settings/defaults inheritance:** an unset field on a subproject resolves through the full ancestor chain (parent → grandparent → ... → global default), not straight to global.
- **Task rollup:** viewing a parent project's board/week/calendar shows its own tasks plus all descendant subprojects' tasks combined.
- **Rollup visual cue:** rolled-up tasks are marked with their originating subproject's color/label so origin is visible at a glance.
- **Delete cascade:** deleting a project with descendants cascades — deletes all descendant subprojects AND their tasks, after a confirmation showing exact counts.
- **NL `+Project` disambiguation:** resolved primarily through autocomplete (which disambiguates same-named subprojects under different parents by showing ancestor context); raw typed text without using autocomplete keeps today's first-match-by-name behavior as a known, documented limitation.
- **Create-subproject entry points:** both a per-project hover "+" action in the sidebar, and a button on the project's own settings page.
- **Re-parenting:** supported, via both sidebar drag-and-drop and a "move to parent" picker on the project settings page. Cycle prevention required (a project cannot become its own descendant's child).
- **Storage:** flat `Vec<Project>` with a new `parent_id: Option<String>` field; tree relationships derived at runtime via helper functions. (Chosen over a nested-JSON tree or a separate adjacency-index file — see "Approaches considered" below.)

## Approaches considered (storage model)

1. **Flat array + `parent_id` (chosen).** Minimal diff from today's `Vec<Project>` in `projects.json`. Tree derived at runtime by grouping on `parent_id`, mirroring the existing flat-list-plus-derived-grouping pattern already used for Kanban status/priority buckets. O(1) id lookups preserved.
2. **Nested JSON (`children: Vec<Project>` embedded).** Rejected — every mutation (create/delete/re-parent/reorder) requires walking and rewriting a nested structure instead of a simple `Vec` operation, and id-based lookup (used pervasively for task linking, settings, color resolution) becomes a tree search instead of a map lookup.
3. **Separate adjacency-index file.** Rejected — introduces a second source of truth that can drift out of sync with `projects.json` (e.g. an orphaned index entry after a project delete that didn't also update the index), which conflicts with this codebase's existing single-file-per-concern persistence philosophy.

## Data model

### `Project` (`src-tauri/src/project.rs`)

```rust
pub struct Project {
    pub id: String,
    pub name: String,
    pub color: String,
    pub parent_id: Option<String>,   // NEW — None means top-level
    pub order: i64,                  // now scoped: sibling order within parent_id (or among top-level if None)
    pub created: String,
    pub board: ProjectBoard,
    pub defaults: TaskDefaults,
}
```

Persistence is unchanged structurally: still one flat JSON array in `<app_data_dir>/projects.json`, via the existing `project_storage.rs` read/write functions.

### New `project_tree.rs` module

Pure helper functions operating on `&[Project]`:

- `children_of(projects, parent_id: Option<&str>) -> Vec<&Project>` — direct children of a given parent (or top-level projects if `None`).
- `ancestors_of(projects, id: &str) -> Vec<&Project>` — walks `parent_id` upward from a project to the root, nearest-first. Used for settings-inheritance resolution, breadcrumbs, and cycle checks.
- `descendants_of(projects, id: &str) -> Vec<&Project>` — all transitive children, used for rollup view filtering and cascade-delete counting.
- `would_create_cycle(projects, moving_id: &str, new_parent_id: &str) -> bool` — true if `new_parent_id` is `moving_id` or any of `moving_id`'s current descendants. Re-parenting is rejected when this returns true.

### `Task` (`src-tauri/src/task.rs`)

`project: Option<String>` (name) → `project_id: Option<String>` (id).

### Migration

This is a `Task`-side migration only — `Project` records need no migration, since `parent_id` defaults to `None` for all existing projects. Runs once at startup, before the app finishes loading:

1. Load `projects.json` and all task markdown files as normal.
2. For every loaded `Task` whose legacy `project` frontmatter key is present and non-empty, and whose `project_id` is not yet set: look up the `Project` whose `name` matches case-insensitively. If found, set `project_id` to that project's `id`. If not found, log a startup warning naming the task and the unmatched project string, and leave `project_id: None`.
3. Every task touched in step 2 is immediately rewritten to disk in the new `project_id`-only frontmatter format (not left as an in-memory-only change waiting for some future unrelated save) — so migration is a one-time, verifiable, eager batch operation, and a crash or restart partway through can't leave some files migrated and others not silently inconsistent.
4. The migration is idempotent: re-running it after a task is already migrated (`project_id` set, legacy `project` key absent) is a no-op for that task.
5. New task writes use `project_id` only; the legacy `project` key is never written by anything after this point.

## Settings & defaults inheritance

`ProjectBoard` and `TaskDefaults` fields remain `Option<T>`. Resolution changes from "own value or global" to a chain walk:

```
own value
  → nearest ancestor with Some value (walking ancestors_of(), nearest-first)
  → global Settings default
```

One new resolver function, e.g. `resolve_board_setting<T>(projects, settings, project_id, field_selector) -> T`, replaces the current direct `Option::or(global)` calls everywhere `ProjectBoard`/`TaskDefaults` fields are read.

`ProjectBoardSettings.svelte` must show, for any field whose effective value comes from an ancestor rather than the project's own setting, which ancestor it's inherited from (e.g. "inherited from CS101") so the page isn't confusing when a freshly created subproject already shows non-default values it never explicitly set.

## Color system

New function in `colorPresets.ts`:

```ts
function shadesOf(parentHex: string, count: number): string[]
```

Built on the existing `hexToOklch`/`neonCardColor` primitives — generates `count` shades at the parent's hue/chroma, varying lightness across a range that avoids extremes likely to break card/bar text contrast.

`ColorPicker.svelte` gains an optional `parentColor` prop. When set (i.e. when creating/editing a subproject), the color grid's first row shows `shadesOf(parentColor, 5)` ahead of the existing 8 fixed presets, visually labeled to distinguish "Shades of [parent name]" from "Presets". The custom hex input remains available unconditionally.

Color is a one-time suggestion at creation time only — re-parenting a project never recomputes or changes its already-set color.

## Sidebar UI

`Sidebar.svelte`'s flat project list becomes a recursive tree:

- New `ProjectTreeNode.svelte` — renders one project row, recursing into its own children when expanded. Indents by depth.
- Expand/collapse chevron shown only on nodes that have at least one child.
- Per-row hover actions: a "+" button (opens the new-subproject dialog, pre-scoped to that project as parent) alongside any existing per-project actions.

### Expand-state persistence

New `projectTree.svelte.ts`, following the existing `sidebar.svelte.ts` / `displaySettings.svelte.ts` shape:

- `projectTreeState = $state<{ expanded: Record<string, boolean> }>(...)`.
- Persisted as one JSON blob under a single `localStorage` key (e.g. `taskmancer:project-tree-expanded`).
- `initProjectTree()` restores on boot.
- Default: a project not yet present in the map is treated as collapsed, **except** that a parent is auto-expanded the first time it gains its first subproject (so a newly created subproject is immediately visible without a manual toggle). Once a project has any entry in the map (from an explicit user toggle), that explicit value is always respected.

### Re-parenting

- **Drag-and-drop:** using the existing `svelte-dnd-action` dependency. Dropping a project row onto another row re-parents it as a child. A "Top Level" drop zone at the root of the list promotes a project out of any parent. Drops that would create a cycle (per `would_create_cycle`) are rejected, shown via a disabled drop-cursor state.
- **"Move to parent" picker:** on the project's settings page, a searchable dropdown listing all projects except the project itself and its own descendants (cycle prevention applied identically here).

## Project page, rollup, breadcrumb, deletion

- **URL:** unchanged — `/projects/{id}` regardless of nesting depth.
- **Breadcrumb:** a new trail above the project title, built from `ancestors_of()`, root-first, each segment linking to that ancestor's page.
- **Rollup filtering:** `KanbanBoard.svelte`'s `projectFilter` (and the equivalent Week/Calendar view filtering) changes from "task.project_id === this project's id" to "task.project_id is this project's id OR any id in `descendants_of(this id)`".
- **Rollup visual cue:** a task card whose `project_id` is a descendant (not the currently-viewed project itself) shows a small colored dot/label naming its originating subproject — reusing the `.bar-status-box`-style swatch pattern from `WeekBarItem.svelte`, repurposed here for "origin project" rather than "status".
- **Cascade delete:** the existing project-delete confirmation gains a line stating the exact count of descendant subprojects and tasks that will also be deleted, computed via `descendants_of()` plus a count of tasks whose `project_id` falls in that set. Confirming performs a full cascade: every descendant `Project` is deleted, and every task belonging to the deleted project or any of its descendants is deleted.

## NL parser & autocomplete

- `naturalLanguage.ts`'s `+ProjectName` token matching changes from "exact name string" to "case-insensitive name match against the currently loaded project list, first match wins on ambiguity" (unchanged behavior for the no-autocomplete path — documented as a known limitation, not solved by this feature).
- `AddTaskModal`'s autocomplete option list, when multiple projects share a name under different parents, renders each option with ancestor context to disambiguate visually (e.g. "Homework — CS101" vs "Homework — Math").
- Selecting an autocomplete option sets a concrete `project_id` directly in the modal's own state — bypassing the parser's name-based resolution for that specific selection, so the ambiguity is resolved at pick time rather than at parse time.

## Testing approach

Following the codebase's established convention — pure logic gets direct unit tests, `#[tauri::command]`-wrapped functions are not tested directly.

**Rust:**
- `project_tree.rs`: `children_of`, `ancestors_of`, `descendants_of`, `would_create_cycle` — empty tree, single level, multi-level, attempted cycles, a `parent_id` pointing at a deleted/missing project.
- Settings-chain resolver: own value wins; falls through one level; falls through multiple levels; falls through to global when no ancestor has a value.
- Migration: exact match, case-insensitive match, no match (warning path, `project_id` stays `None`), already-migrated task (idempotent re-run), empty/missing legacy `project` field.

**Frontend (Vitest):**
- `shadesOf()` — correct count, hue/chroma preserved from parent, lightness bounds avoid contrast-breaking extremes.
- Rollup project-filter logic — descendant-inclusive matching against a multi-level tree.
- Expand-state `localStorage` round-trip, including the "auto-expand on first child" default.
- Autocomplete disambiguation — collision detection and ancestor-context label formatting.

**Manual/visual (disclosed limitation, consistent with prior log entries for this codebase):** sidebar tree rendering at varying depths, drag-and-drop re-parenting feel, cascade-delete confirmation copy, breadcrumb rendering. These need a user testing pass after implementation.

## Out of scope for this spec

- Subtasks (auto-generated per-task subprojects) — explicitly deferred to its own design cycle once this ships.
- Per-view (board/week/calendar) × per-scope "show subtasks" settings matrix — not applicable until Subtasks exists.
- Any change to the NL parser's `sub <parent task name>` syntax — belongs to the Subtasks spec.
