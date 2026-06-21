# Subprojects Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add nested (arbitrary-depth) subprojects to taskmancer: a `parent_id` on `Project`, ancestor-chain settings inheritance, parent-derived color suggestions, a sidebar tree UI with drag-and-drop re-parenting, task rollup across descendants, cascade delete, and id-based (not name-based) linkage between tasks and projects.

**Architecture:** Storage stays a flat `Vec<Project>` in `projects.json` with a new `parent_id: Option<String>` field; tree relationships (children/ancestors/descendants/cycle-check) are derived at runtime by pure helper functions, mirrored in Rust (`project_tree.rs`) and TypeScript (`projectTree.ts`). `Task.project` (a name string) and `Settings.default_project` (also a name string) both switch to id-based fields (`project_id`, `default_project_id`) for unambiguous linkage once projects can share names at different tree positions. A one-time eager startup migration rewrites existing task files and `settings.json` from the old name-based fields to the new id-based ones.

**Tech Stack:** Rust (Tauri backend, `src-tauri/src/`), Svelte 5 + TypeScript (frontend, `src/`), `uuid` crate for ids, `svelte-dnd-action` for drag-and-drop, Vitest + `cargo test`.

**Spec:** `docs/superpowers/specs/2026-06-20-subprojects-design.md`

## Global Constraints

- Rust requires the whole crate to compile to run any test — tasks that change a struct's shape (`Project`, `Task`, `Settings`) must update every call site in the same task, not defer it to a later task.
- `svelte-check` type-checks the whole frontend project — the same whole-project constraint applies to `types.ts` changes on the frontend side.
- IDs are generated via `uuid::Uuid::new_v4().to_string()` (already used by `Project::new`/`Task::new`) — no new id-generation crate.
- Color is hex-only (`#RRGGBB`), validated by `validate_hex_color` (`commands.rs`) — no native `<input type="color">` anywhere (hangs the GTK webview on Linux).
- Rust unit tests live inline in `#[cfg(test)] mod tests { use super::*; ... }` at the bottom of the same file — no separate `tests/` directory.
- Vitest test files are colocated next to their source file with a `.test.ts` suffix (`.svelte.test.ts` for `.svelte.ts` modules) — no separate `tests/`/`__tests__/` directory.
- `cargo fmt -- --check` and `cargo clippy --lib -- -D warnings` must stay clean; `npx svelte-check` must report 0 errors; `npm run build` must succeed. Run all of these as part of finishing each task that touches their respective layer.
- Never test `#[tauri::command]`-wrapped functions directly — only their extracted pure-logic helpers.
- Real user data exists in `projects.json`/task markdown files/`settings.json` — migrations must be eager (rewrite to disk immediately, not lazily on next unrelated save) and idempotent, with unmatched references logged as warnings, never silently dropped.

---

## Task 1: `project_tree.rs` — pure tree helpers

**Files:**
- Create: `src-tauri/src/project_tree.rs`
- Modify: `src-tauri/src/lib.rs:1-10` (add `mod project_tree;`)

**Interfaces:**
- Produces: `pub fn children_of<'a>(projects: &'a [Project], parent_id: Option<&str>) -> Vec<&'a Project>`, `pub fn ancestors_of<'a>(projects: &'a [Project], id: &str) -> Vec<&'a Project>`, `pub fn self_and_ancestors<'a>(projects: &'a [Project], id: &str) -> Vec<&'a Project>`, `pub fn descendants_of<'a>(projects: &'a [Project], id: &str) -> Vec<&'a Project>`, `pub fn would_create_cycle(projects: &[Project], moving_id: &str, new_parent_id: &str) -> bool`. All operate on `crate::project::Project`, which does not yet have a `parent_id` field (added in Task 2) — write this task's tests against a local test-only struct shape by adding `parent_id` usage now; since Task 2 adds the field to the real `Project` struct immediately after, this task's code will not compile-check against the real struct until Task 2 lands. To keep this task independently compilable and testable today, this task ALSO adds the `parent_id: Option<String>` field to `Project` (pulled forward from Task 2) — see Step 1 below. Task 2 then only needs the command-layer wiring (create/update), not the struct field itself.

- [ ] **Step 1: Add `parent_id` to `Project` (pulled forward so this task's helpers compile)**

Edit `src-tauri/src/project.rs`. Find the `Project` struct (currently):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub color: String,
    #[serde(default)]
    pub order: i64,
    pub created: String,
    #[serde(default)]
    pub board: ProjectBoard,
    #[serde(default)]
    pub defaults: TaskDefaults,
}
```

Replace with:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub color: String,
    /// The id of this project's parent, or `None` for a top-level project.
    /// Nesting is arbitrary depth — a project named here may itself have a
    /// non-`None` `parent_id`. See `crate::project_tree` for helpers that
    /// walk this relationship.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub order: i64,
    pub created: String,
    #[serde(default)]
    pub board: ProjectBoard,
    #[serde(default)]
    pub defaults: TaskDefaults,
}
```

Find `Project::new`:

```rust
impl Project {
    /// Creates a new project with a freshly generated id and the current
    /// time as `created`.
    pub fn new(name: String, color: String, order: i64) -> Self {
        Project {
            id: Uuid::new_v4().to_string(),
            name,
            color,
            order,
            created: Utc::now().to_rfc3339(),
            board: ProjectBoard::default(),
            defaults: TaskDefaults::default(),
        }
    }
}
```

Replace with (adds `parent_id: None` — callers that need a subproject set `project.parent_id = Some(...)` on the returned value, keeping every existing 3-argument call site unchanged):

```rust
impl Project {
    /// Creates a new top-level project with a freshly generated id and the
    /// current time as `created`. Callers creating a subproject set
    /// `parent_id` on the returned value directly (it's a public field).
    pub fn new(name: String, color: String, order: i64) -> Self {
        Project {
            id: Uuid::new_v4().to_string(),
            name,
            color,
            parent_id: None,
            order,
            created: Utc::now().to_rfc3339(),
            board: ProjectBoard::default(),
            defaults: TaskDefaults::default(),
        }
    }
}
```

- [ ] **Step 2: Add a test confirming the field defaults correctly**

Append inside the existing `#[cfg(test)] mod tests { ... }` block at the bottom of `project.rs` (after the existing `board_ink_mode_is_none_when_absent_from_json` test):

```rust
    #[test]
    fn new_project_has_no_parent() {
        let project = Project::new("Homework".to_string(), "#ff0000".to_string(), 1);

        assert_eq!(project.parent_id, None);
    }

    #[test]
    fn parent_id_is_none_when_absent_from_json() {
        let json = r##"{"id":"abc","name":"Inbox","color":"#000000","created":"2026-06-11T10:00:00+00:00"}"##;

        let project: Project = serde_json::from_str(json).expect("parsing should succeed");

        assert_eq!(project.parent_id, None);
    }

    #[test]
    fn parent_id_round_trips_when_set() {
        let mut project = Project::new("Homework".to_string(), "#ff0000".to_string(), 1);
        project.parent_id = Some("parent-123".to_string());

        let json = serde_json::to_string(&project).expect("serialization should succeed");
        let parsed: Project = serde_json::from_str(&json).expect("parsing should succeed");

        assert_eq!(parsed.parent_id, Some("parent-123".to_string()));
    }
```

- [ ] **Step 3: Run the project.rs tests to confirm they pass**

Run: `cd src-tauri && cargo test --lib project::tests`
Expected: all tests pass, including the 3 new ones.

- [ ] **Step 4: Write the failing tests for `project_tree.rs`**

Create `src-tauri/src/project_tree.rs`:

```rust
use crate::project::Project;

/// Returns the direct children of `parent_id` (or every top-level project,
/// if `parent_id` is `None`), in the order they appear in `projects`.
pub fn children_of<'a>(projects: &'a [Project], parent_id: Option<&str>) -> Vec<&'a Project> {
    projects
        .iter()
        .filter(|p| p.parent_id.as_deref() == parent_id)
        .collect()
}

/// Returns the ancestors of the project identified by `id`, nearest-first,
/// ending at the root. Empty if `id` doesn't exist in `projects` or names a
/// top-level project.
pub fn ancestors_of<'a>(projects: &'a [Project], id: &str) -> Vec<&'a Project> {
    let mut result = Vec::new();
    let mut current_id = projects
        .iter()
        .find(|p| p.id == id)
        .and_then(|p| p.parent_id.clone());

    while let Some(ancestor_id) = current_id {
        let Some(ancestor) = projects.iter().find(|p| p.id == ancestor_id) else {
            break;
        };
        result.push(ancestor);
        current_id = ancestor.parent_id.clone();
    }

    result
}

/// Returns the project identified by `id` (if found) followed by its
/// ancestors, nearest-first — the full settings-resolution chain for that
/// project, ending at the root. Empty if `id` doesn't exist in `projects`.
pub fn self_and_ancestors<'a>(projects: &'a [Project], id: &str) -> Vec<&'a Project> {
    let mut result: Vec<&Project> = projects.iter().find(|p| p.id == id).into_iter().collect();
    result.extend(ancestors_of(projects, id));
    result
}

/// Returns every transitive descendant of the project identified by `id`
/// (children, grandchildren, ...). Order is not guaranteed to be any
/// particular traversal order — callers needing a specific order should sort
/// the result themselves.
pub fn descendants_of<'a>(projects: &'a [Project], id: &str) -> Vec<&'a Project> {
    let mut result: Vec<&Project> = Vec::new();
    let mut frontier: Vec<String> = vec![id.to_string()];

    while let Some(current_id) = frontier.pop() {
        for child in children_of(projects, Some(current_id.as_str())) {
            result.push(child);
            frontier.push(child.id.clone());
        }
    }

    result
}

/// Returns `true` if making `new_parent_id` the parent of `moving_id` would
/// create a cycle — i.e. `new_parent_id` is `moving_id` itself, or is one of
/// `moving_id`'s current descendants.
pub fn would_create_cycle(projects: &[Project], moving_id: &str, new_parent_id: &str) -> bool {
    if moving_id == new_parent_id {
        return true;
    }
    descendants_of(projects, moving_id)
        .iter()
        .any(|p| p.id == new_parent_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a small fixture tree:
    /// ```text
    /// root_a (top-level)
    /// ├── child_a1
    /// │   └── grandchild_a1a
    /// └── child_a2
    /// root_b (top-level, no children)
    /// ```
    fn fixture_tree() -> Vec<Project> {
        let mut root_a = Project::new("Root A".to_string(), "#111111".to_string(), 1);
        root_a.id = "root_a".to_string();

        let mut root_b = Project::new("Root B".to_string(), "#222222".to_string(), 2);
        root_b.id = "root_b".to_string();

        let mut child_a1 = Project::new("Child A1".to_string(), "#333333".to_string(), 1);
        child_a1.id = "child_a1".to_string();
        child_a1.parent_id = Some("root_a".to_string());

        let mut child_a2 = Project::new("Child A2".to_string(), "#444444".to_string(), 2);
        child_a2.id = "child_a2".to_string();
        child_a2.parent_id = Some("root_a".to_string());

        let mut grandchild_a1a = Project::new("Grandchild A1a".to_string(), "#555555".to_string(), 1);
        grandchild_a1a.id = "grandchild_a1a".to_string();
        grandchild_a1a.parent_id = Some("child_a1".to_string());

        vec![root_a, root_b, child_a1, child_a2, grandchild_a1a]
    }

    #[test]
    fn children_of_none_returns_top_level_projects() {
        let projects = fixture_tree();

        let children = children_of(&projects, None);

        let ids: Vec<&str> = children.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["root_a", "root_b"]);
    }

    #[test]
    fn children_of_returns_direct_children_only() {
        let projects = fixture_tree();

        let children = children_of(&projects, Some("root_a"));

        let ids: Vec<&str> = children.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["child_a1", "child_a2"]);
    }

    #[test]
    fn children_of_returns_empty_for_a_leaf() {
        let projects = fixture_tree();

        let children = children_of(&projects, Some("child_a2"));

        assert!(children.is_empty());
    }

    #[test]
    fn ancestors_of_top_level_project_is_empty() {
        let projects = fixture_tree();

        let ancestors = ancestors_of(&projects, "root_a");

        assert!(ancestors.is_empty());
    }

    #[test]
    fn ancestors_of_returns_nearest_first() {
        let projects = fixture_tree();

        let ancestors = ancestors_of(&projects, "grandchild_a1a");

        let ids: Vec<&str> = ancestors.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["child_a1", "root_a"]);
    }

    #[test]
    fn ancestors_of_missing_id_is_empty() {
        let projects = fixture_tree();

        let ancestors = ancestors_of(&projects, "does_not_exist");

        assert!(ancestors.is_empty());
    }

    #[test]
    fn ancestors_of_stops_at_a_dangling_parent_id() {
        let mut projects = fixture_tree();
        // Simulate a parent that was deleted without cleaning up this
        // reference: child_a1's parent_id points at a project that no
        // longer exists.
        projects[2].parent_id = Some("deleted_project".to_string());

        let ancestors = ancestors_of(&projects, "grandchild_a1a");

        let ids: Vec<&str> = ancestors.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["child_a1"]);
    }

    #[test]
    fn self_and_ancestors_includes_self_first() {
        let projects = fixture_tree();

        let chain = self_and_ancestors(&projects, "grandchild_a1a");

        let ids: Vec<&str> = chain.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["grandchild_a1a", "child_a1", "root_a"]);
    }

    #[test]
    fn self_and_ancestors_for_missing_id_is_empty() {
        let projects = fixture_tree();

        let chain = self_and_ancestors(&projects, "does_not_exist");

        assert!(chain.is_empty());
    }

    #[test]
    fn descendants_of_returns_all_levels() {
        let projects = fixture_tree();

        let descendants = descendants_of(&projects, "root_a");

        let mut ids: Vec<&str> = descendants.iter().map(|p| p.id.as_str()).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec!["child_a1", "child_a2", "grandchild_a1a"]);
    }

    #[test]
    fn descendants_of_a_leaf_is_empty() {
        let projects = fixture_tree();

        let descendants = descendants_of(&projects, "grandchild_a1a");

        assert!(descendants.is_empty());
    }

    #[test]
    fn descendants_of_an_unrelated_root_is_empty() {
        let projects = fixture_tree();

        let descendants = descendants_of(&projects, "root_b");

        assert!(descendants.is_empty());
    }

    #[test]
    fn would_create_cycle_when_moving_under_self() {
        let projects = fixture_tree();

        assert!(would_create_cycle(&projects, "root_a", "root_a"));
    }

    #[test]
    fn would_create_cycle_when_moving_under_own_descendant() {
        let projects = fixture_tree();

        assert!(would_create_cycle(&projects, "root_a", "grandchild_a1a"));
    }

    #[test]
    fn would_not_create_cycle_when_moving_to_an_unrelated_project() {
        let projects = fixture_tree();

        assert!(!would_create_cycle(&projects, "child_a1", "root_b"));
    }

    #[test]
    fn would_not_create_cycle_when_moving_to_top_level() {
        let projects = fixture_tree();

        // Promoting child_a1 to top-level is modeled as re-parenting under
        // a synthetic id that isn't in the tree at all — would_create_cycle
        // only guards against id-based cycles, so this is just confirming
        // an arbitrary unrelated id is never flagged.
        assert!(!would_create_cycle(&projects, "child_a1", "unrelated_id"));
    }
}
```

- [ ] **Step 5: Run the new tests to confirm they pass**

Run: `cd src-tauri && cargo test --lib project_tree::tests`
Expected: all 16 tests pass.

- [ ] **Step 6: Register the module**

Edit `src-tauri/src/lib.rs`. Find:

```rust
mod commands;
mod project;
mod project_storage;
mod recurrence;
mod series;
mod series_storage;
mod settings;
mod settings_storage;
mod storage;
mod task;
```

Replace with:

```rust
mod commands;
mod project;
mod project_storage;
mod project_tree;
mod recurrence;
mod series;
mod series_storage;
mod settings;
mod settings_storage;
mod storage;
mod task;
```

- [ ] **Step 7: Run the full backend test suite and lints**

Run: `cd src-tauri && cargo test --lib && cargo fmt -- --check && cargo clippy --lib -- -D warnings`
Expected: all tests pass (no regressions), `cargo fmt` reports no diffs, `cargo clippy` reports no warnings. (`project_tree` isn't `use`d from anywhere yet, so clippy may report `unused` on the new `mod` line — if so, add `#[allow(dead_code)]` to the top of `project_tree.rs` temporarily; Task 3 starts consuming it and the `allow` should be removed then.)

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/project.rs src-tauri/src/project_tree.rs src-tauri/src/lib.rs
git commit -m "feat: add Project.parent_id and project_tree tree helpers"
```

---

## Task 2: `create_project`/`update_project` accept and validate `parent_id`

**Files:**
- Modify: `src-tauri/src/commands.rs:984-1009` (`create_project`)
- Modify: `src-tauri/src/commands.rs:1031-1093` (`apply_project_update`/`update_project`)

**Interfaces:**
- Consumes: `project_tree::would_create_cycle(projects: &[Project], moving_id: &str, new_parent_id: &str) -> bool` (Task 1).
- Produces: `create_project(state, name: String, color: Option<String>, parent_id: Option<String>) -> Result<Project, String>` (new 3rd parameter); `apply_project_update`/`update_project` now persist `update.parent_id` (validated) instead of dropping it.

- [ ] **Step 1: Write failing tests for parent_id validation in `create_project`**

`create_project`'s logic (uniqueness check, color resolution) is presently only exercised indirectly — there's no existing `#[cfg(test)]` block in `commands.rs` calling `create_project` directly since it's a `#[tauri::command]`. Per the global constraint, don't test the command function directly. Instead, extract a tiny pure validator this task introduces, and test that. Add to the bottom of `commands.rs`'s existing `#[cfg(test)] mod tests { ... }` block (if none exists yet, check first — search for `mod tests` in `commands.rs`; if absent, add one at the very end of the file):

```bash
grep -n "mod tests" src-tauri/src/commands.rs
```

If that prints nothing, append this to the end of `commands.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_parent_id_accepts_none() {
        let projects = vec![Project::new("Inbox".to_string(), "#111111".to_string(), 1)];

        assert!(validate_parent_id(&projects, None, None).is_ok());
    }

    #[test]
    fn validate_parent_id_accepts_an_existing_project() {
        let parent = Project::new("Inbox".to_string(), "#111111".to_string(), 1);
        let parent_id = parent.id.clone();
        let projects = vec![parent];

        assert!(validate_parent_id(&projects, Some(&parent_id), None).is_ok());
    }

    #[test]
    fn validate_parent_id_rejects_a_missing_project() {
        let projects = vec![Project::new("Inbox".to_string(), "#111111".to_string(), 1)];

        let result = validate_parent_id(&projects, Some("does-not-exist"), None);

        assert!(result.is_err());
    }

    #[test]
    fn validate_parent_id_rejects_a_self_cycle_on_update() {
        let project = Project::new("Inbox".to_string(), "#111111".to_string(), 1);
        let id = project.id.clone();
        let projects = vec![project];

        let result = validate_parent_id(&projects, Some(&id), Some(&id));

        assert!(result.is_err());
    }

    #[test]
    fn validate_parent_id_rejects_moving_under_own_descendant() {
        let mut parent = Project::new("Parent".to_string(), "#111111".to_string(), 1);
        parent.id = "parent".to_string();
        let mut child = Project::new("Child".to_string(), "#222222".to_string(), 2);
        child.id = "child".to_string();
        child.parent_id = Some("parent".to_string());
        let projects = vec![parent, child];

        let result = validate_parent_id(&projects, Some("child"), Some("parent"));

        assert!(result.is_err());
    }

    #[test]
    fn validate_parent_id_allows_an_unrelated_new_parent_on_update() {
        let mut a = Project::new("A".to_string(), "#111111".to_string(), 1);
        a.id = "a".to_string();
        let mut b = Project::new("B".to_string(), "#222222".to_string(), 2);
        b.id = "b".to_string();
        let projects = vec![a, b];

        assert!(validate_parent_id(&projects, Some("b"), Some("a")).is_ok());
    }
}
```

If a `mod tests` block already exists by the time this task runs (Task 4/5/6 may have added one first, depending on execution order — check with the `grep` above), append these 6 `#[test]` functions inside the existing block instead of creating a new one.

- [ ] **Step 2: Run the tests to confirm they fail (function doesn't exist yet)**

Run: `cd src-tauri && cargo test --lib commands::tests::validate_parent_id`
Expected: FAIL with `cannot find function 'validate_parent_id' in this scope`.

- [ ] **Step 3: Implement `validate_parent_id` and wire it into `create_project`**

Edit `src-tauri/src/commands.rs`. Add the import at the top — find:

```rust
use crate::project::{Project, ProjectBoard, DEFAULT_PROJECT_COLOR};
use crate::project_storage;
```

Replace with:

```rust
use crate::project::{Project, ProjectBoard, DEFAULT_PROJECT_COLOR};
use crate::project_storage;
use crate::project_tree::would_create_cycle;
```

Find `create_project`:

```rust
/// Creates a new project with a trimmed, non-empty, case-insensitively
/// unique name. `order` is set to one past the current maximum so new
/// projects sort after existing ones. Falls back to
/// [`DEFAULT_PROJECT_COLOR`] when `color` is `None`; otherwise `color` must
/// be a 6-digit hex color (see [`validate_hex_color`]).
#[tauri::command]
pub fn create_project(
    state: State<AppState>,
    name: String,
    color: Option<String>,
) -> Result<Project, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("project name must not be empty".to_string());
    }

    let color = resolve_project_color(color)?;

    let _guard = state.projects_lock.lock().map_err(|e| e.to_string())?;
    let mut projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
    if projects.iter().any(|p| p.name.eq_ignore_ascii_case(&name)) {
        return Err(format!("a project named '{name}' already exists"));
    }

    let order = next_order(&projects);
    let project = Project::new(name, color, order);
    projects.push(project.clone());
    project_storage::save_projects(&state.projects_file, &projects).map_err(|e| e.to_string())?;

    Ok(project)
}
```

Replace with (adds the `parent_id` parameter, validates it via the new `validate_parent_id`, and sets it on the created project):

```rust
/// Returns an error if `parent_id` (when set) doesn't reference an existing
/// project in `projects`, or — when `moving_id` is also set (an existing
/// project being re-parented, as opposed to a brand-new project that can
/// never have descendants yet) — would create a cycle (see
/// [`would_create_cycle`]).
fn validate_parent_id(
    projects: &[Project],
    parent_id: Option<&str>,
    moving_id: Option<&str>,
) -> Result<(), String> {
    let Some(parent_id) = parent_id else {
        return Ok(());
    };

    if !projects.iter().any(|p| p.id == parent_id) {
        return Err(format!("parent project '{parent_id}' not found"));
    }

    if let Some(moving_id) = moving_id {
        if would_create_cycle(projects, moving_id, parent_id) {
            return Err("cannot move a project under itself or one of its own subprojects".to_string());
        }
    }

    Ok(())
}

/// Creates a new project with a trimmed, non-empty, case-insensitively
/// unique name. `order` is set to one past the current maximum so new
/// projects sort after existing ones. Falls back to
/// [`DEFAULT_PROJECT_COLOR`] when `color` is `None`; otherwise `color` must
/// be a 6-digit hex color (see [`validate_hex_color`]). `parent_id`, when
/// set, must reference an existing project (see [`validate_parent_id`]) —
/// nesting depth is otherwise unrestricted.
#[tauri::command]
pub fn create_project(
    state: State<AppState>,
    name: String,
    color: Option<String>,
    parent_id: Option<String>,
) -> Result<Project, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("project name must not be empty".to_string());
    }

    let color = resolve_project_color(color)?;

    let _guard = state.projects_lock.lock().map_err(|e| e.to_string())?;
    let mut projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
    if projects.iter().any(|p| p.name.eq_ignore_ascii_case(&name)) {
        return Err(format!("a project named '{name}' already exists"));
    }
    validate_parent_id(&projects, parent_id.as_deref(), None)?;

    let order = next_order(&projects);
    let mut project = Project::new(name, color, order);
    project.parent_id = parent_id;
    projects.push(project.clone());
    project_storage::save_projects(&state.projects_file, &projects).map_err(|e| e.to_string())?;

    Ok(project)
}
```

- [ ] **Step 4: Wire `parent_id` into `apply_project_update`/`update_project`**

Find `apply_project_update`'s final struct literal (inside the function, near the end):

```rust
    let updated = Project {
        id: existing.id.clone(),
        name,
        color: update.color,
        order: update.order,
        created: existing.created.clone(),
        board: update.board,
        defaults: update.defaults,
    };
    projects[index] = updated.clone();
    Ok(updated)
}
```

Replace with (validates the new `parent_id` before building the literal, using the project's own id as `moving_id` since this is an existing project that may have descendants):

```rust
    let existing_id = existing.id.clone();
    validate_parent_id(projects, update.parent_id.as_deref(), Some(&existing_id))?;

    let updated = Project {
        id: existing_id,
        name,
        color: update.color,
        parent_id: update.parent_id,
        order: update.order,
        created: existing.created.clone(),
        board: update.board,
        defaults: update.defaults,
    };
    projects[index] = updated.clone();
    Ok(updated)
}
```

Note `existing` (the `&projects[index]` reference taken earlier in the function for `existing.color`) is borrowed before this point — `validate_parent_id(projects, ...)` takes `&[Project]` (the whole slice, not just `existing`), which conflicts with `existing: &Project` (a borrow of `projects[index]`) still being live across the call. Find the line just above where `existing` is declared:

```rust
    let existing = &projects[index];
    if update.color != existing.color {
        validate_hex_color(&update.color)?;
    }
```

Replace with (clones what's needed out of `existing` before the borrow would otherwise conflict, then drops the reference):

```rust
    let existing_color = projects[index].color.clone();
    let existing_created = projects[index].created.clone();
    if update.color != existing_color {
        validate_hex_color(&update.color)?;
    }
```

And update the struct literal from Step 4 above to use `existing_created` instead of `existing.created.clone()` and `existing_id` instead of `existing.id.clone()` (since `existing` no longer exists as a binding) — find:

```rust
    let existing_id = existing.id.clone();
    validate_parent_id(projects, update.parent_id.as_deref(), Some(&existing_id))?;

    let updated = Project {
        id: existing_id,
        name,
        color: update.color,
        parent_id: update.parent_id,
        order: update.order,
        created: existing.created.clone(),
        board: update.board,
        defaults: update.defaults,
    };
```

Replace with:

```rust
    let existing_id = projects[index].id.clone();
    validate_parent_id(projects, update.parent_id.as_deref(), Some(&existing_id))?;

    let updated = Project {
        id: existing_id,
        name,
        color: update.color,
        parent_id: update.parent_id,
        order: update.order,
        created: existing_created,
        board: update.board,
        defaults: update.defaults,
    };
```

- [ ] **Step 5: Run the tests to confirm they pass**

Run: `cd src-tauri && cargo test --lib commands::tests::validate_parent_id`
Expected: all 6 tests pass.

- [ ] **Step 6: Run the full backend suite and lints**

Run: `cd src-tauri && cargo test --lib && cargo fmt -- --check && cargo clippy --lib -- -D warnings`
Expected: all tests pass, no fmt diffs, no clippy warnings. (`create_project`'s 3-argument call sites elsewhere in `commands.rs` — e.g. `backfill_projects`, which calls `Project::new` directly, not `create_project` — are unaffected, since `create_project`'s new `parent_id` parameter doesn't change `Project::new`'s signature.)

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: create_project/update_project accept and validate parent_id"
```

---

## Task 3: Ancestor-chain settings resolution

**Files:**
- Modify: `src-tauri/src/commands.rs` (`resolve_default_status`, `effective_default_tags`, `effective_default_estimated_minutes`, `effective_default_code` [removed], `resolve_creation_defaults`, `build_series_occurrence`, `generate_series_occurrences`, `create_task`, `create_recurring_task`, `ensure_occurrences_until`, plus their existing unit tests)

**Interfaces:**
- Consumes: `project_tree::self_and_ancestors(projects: &[Project], id: &str) -> Vec<&Project>` (Task 1).
- Produces: `resolve_default_status(settings: &Settings, project_chain: &[&Project]) -> String`, `effective_default_tags(global: &TaskDefaults, project_chain: &[&Project]) -> Vec<String>`, `effective_default_estimated_minutes(global: &TaskDefaults, project_chain: &[&Project]) -> Option<u32>`, `resolve_creation_defaults(settings: &Settings, project_chain: &[&Project], today: NaiveDate, tags, due, scheduled) -> (Vec<String>, Option<String>, String)` — all now take a `project_chain: &[&Project]` (nearest-first, from `self_and_ancestors`) instead of a single `Option<&ProjectBoard>`/`Option<&TaskDefaults>`. `effective_default_code` is removed (folded inline into `resolve_creation_defaults`).

This task changes only the *settings-resolution* logic — it does not change how a task's project is identified (still by name, via `find_project`, until Task 5/6). It only changes what happens once a matching `Project` is found: instead of reading that one project's `board`/`defaults` directly, it walks the full ancestor chain.

- [ ] **Step 1: Update the existing unit tests for the four functions to the new chain-based signature**

These functions already have tests (verify with `grep -n "fn resolve_default_status_\|fn effective_default_tags_\|fn effective_default_estimated_minutes_\|fn effective_default_code_" src-tauri/src/commands.rs` — there will be several `#[test]` functions for each, somewhere in the large `#[cfg(test)] mod tests` block). For each existing test calling one of these four functions with `Option<&ProjectBoard>`/`Option<&TaskDefaults>`, change the call to build a `Vec<&Project>` chain instead. Concretely, search-and-replace this pattern throughout the test module:

Find (repeated, with varying project values, e.g.):
```rust
        let board = ProjectBoard {
            default_status: Some("doing".to_string()),
            ..Default::default()
        };

        let result = resolve_default_status(&settings, Some(&board));
```

Replace with (wrap the board in a throwaway `Project` and pass a one-element chain):
```rust
        let mut project = Project::new("Test".to_string(), "#111111".to_string(), 1);
        project.board = ProjectBoard {
            default_status: Some("doing".to_string()),
            ..Default::default()
        };
        let chain = vec![&project];

        let result = resolve_default_status(&settings, &chain);
```

Apply the equivalent transformation to every test calling `resolve_default_status`, `effective_default_tags`, `effective_default_estimated_minutes` with a `ProjectBoard`/`TaskDefaults` argument: construct a `Project`, set its `.board`/`.defaults`, build a one-element (or, for chain-specific new tests below, multi-element) `Vec<&Project>`, and pass that. For tests currently passing `None` (no project), pass `&[]` (an empty chain) instead. For `effective_default_code` specifically, since the function is removed, delete its tests entirely — the equivalent behavior is now covered by `resolve_creation_defaults`'s own tests (which already exist and don't call `effective_default_code` directly, so they need no signature change beyond the `project_chain` parameter rename covered in Step 3).

- [ ] **Step 2: Run the tests to confirm they fail (signatures don't match yet)**

Run: `cd src-tauri && cargo test --lib commands::tests::resolve_default_status`
Expected: FAIL with a type mismatch (`Option<&ProjectBoard>` vs the test now passing `&Vec<&Project>`).

- [ ] **Step 3: Rewrite the four functions**

Edit `src-tauri/src/commands.rs`. Find:

```rust
fn resolve_default_status(settings: &Settings, project_board: Option<&ProjectBoard>) -> String {
    if let Some(default_id) = project_board.and_then(|board| board.default_status.as_ref()) {
        if validate_status_id(settings, default_id).is_ok() {
            return default_id.clone();
        }
    }

    if let Some(default_id) = &settings.defaults.status {
        if validate_status_id(settings, default_id).is_ok() {
            return default_id.clone();
        }
    }

    settings
        .statuses
        .iter()
        .min_by_key(|status| status.order)
        .map(|status| status.id.clone())
        .unwrap_or_else(|| "backlog".to_string())
}
```

Replace with:

```rust
/// Resolves the status a new task should get when none was explicitly
/// requested. Checked in order: each project in `project_chain` (the task's
/// own project, then its ancestors nearest-first — see
/// [`crate::project_tree::self_and_ancestors`])'s `board.default_status` (if
/// it names a currently-defined status), `settings.defaults.status` (if it
/// names a currently-defined status), the status with the lowest `order`
/// (order 1 sorts first), otherwise `"backlog"` if no statuses are defined
/// at all.
fn resolve_default_status(settings: &Settings, project_chain: &[&Project]) -> String {
    for project in project_chain {
        if let Some(default_id) = &project.board.default_status {
            if validate_status_id(settings, default_id).is_ok() {
                return default_id.clone();
            }
        }
    }

    if let Some(default_id) = &settings.defaults.status {
        if validate_status_id(settings, default_id).is_ok() {
            return default_id.clone();
        }
    }

    settings
        .statuses
        .iter()
        .min_by_key(|status| status.order)
        .map(|status| status.id.clone())
        .unwrap_or_else(|| "backlog".to_string())
}
```

Find:

```rust
/// Returns the default tags that should be merged into a newly-created
/// task's explicit tags: the project's default tags if it has any, otherwise
/// the global default tags.
fn effective_default_tags(global: &TaskDefaults, project: Option<&TaskDefaults>) -> Vec<String> {
    match project {
        Some(p) if !p.tags.is_empty() => p.tags.clone(),
        _ => global.tags.clone(),
    }
}

/// Resolves the effective `estimated_minutes` default for a new task that
/// doesn't specify its own estimate: the project-level override if set,
/// otherwise the global default, otherwise `None` (no estimate).
fn effective_default_estimated_minutes(
    global: &TaskDefaults,
    project: Option<&TaskDefaults>,
) -> Option<u32> {
    project
        .and_then(|p| p.estimated_minutes)
        .or(global.estimated_minutes)
}

/// Resolves the effective relative-date code for a `due`/`scheduled`
/// default: the project-level override if set, otherwise the global default.
fn effective_default_code<'a>(
    global: &'a Option<String>,
    project: Option<&'a Option<String>>,
) -> Option<&'a String> {
    project.and_then(|p| p.as_ref()).or(global.as_ref())
}
```

Replace with:

```rust
/// Returns the default tags that should be merged into a newly-created
/// task's explicit tags: the nearest project in `project_chain` (the task's
/// own project, then its ancestors nearest-first) with a non-empty
/// `defaults.tags`, otherwise the global default tags.
fn effective_default_tags(global: &TaskDefaults, project_chain: &[&Project]) -> Vec<String> {
    project_chain
        .iter()
        .map(|p| &p.defaults.tags)
        .find(|tags| !tags.is_empty())
        .cloned()
        .unwrap_or_else(|| global.tags.clone())
}

/// Resolves the effective `estimated_minutes` default for a new task that
/// doesn't specify its own estimate: the nearest project in `project_chain`
/// (the task's own project, then its ancestors nearest-first) with an
/// override, otherwise the global default, otherwise `None` (no estimate).
fn effective_default_estimated_minutes(global: &TaskDefaults, project_chain: &[&Project]) -> Option<u32> {
    project_chain
        .iter()
        .find_map(|p| p.defaults.estimated_minutes)
        .or(global.estimated_minutes)
}
```

Find:

```rust
fn resolve_creation_defaults(
    settings: &Settings,
    project_defaults: Option<&TaskDefaults>,
    today: chrono::NaiveDate,
    tags: Option<Vec<String>>,
    due: Option<String>,
    scheduled: Option<String>,
) -> (Vec<String>, Option<String>, String) {
    let due_code = effective_default_code(&settings.defaults.due, project_defaults.map(|d| &d.due));
    let scheduled_code = effective_default_code(
        &settings.defaults.scheduled,
        project_defaults.map(|d| &d.scheduled),
    );

    let resolved_scheduled = scheduled
        .or_else(|| resolve_default_scheduled_date(scheduled_code, today))
        .unwrap_or_else(|| today.format("%Y-%m-%d").to_string());

    let resolved_due = match due.as_deref() {
        Some("none") => None,
        Some(_) => due,
        None => {
            let scheduled_date =
                chrono::NaiveDate::parse_from_str(&resolved_scheduled, "%Y-%m-%d").ok();
            scheduled_date.and_then(|date| resolve_default_due_date(due_code, date))
        }
    };

    let default_tags = effective_default_tags(&settings.defaults, project_defaults);
    let final_tags = merge_tags(tags.unwrap_or_default(), default_tags);

    (final_tags, resolved_due, resolved_scheduled)
}
```

Replace with:

```rust
fn resolve_creation_defaults(
    settings: &Settings,
    project_chain: &[&Project],
    today: chrono::NaiveDate,
    tags: Option<Vec<String>>,
    due: Option<String>,
    scheduled: Option<String>,
) -> (Vec<String>, Option<String>, String) {
    let due_code = project_chain
        .iter()
        .find_map(|p| p.defaults.due.as_ref())
        .or(settings.defaults.due.as_ref());
    let scheduled_code = project_chain
        .iter()
        .find_map(|p| p.defaults.scheduled.as_ref())
        .or(settings.defaults.scheduled.as_ref());

    let resolved_scheduled = scheduled
        .or_else(|| resolve_default_scheduled_date(scheduled_code, today))
        .unwrap_or_else(|| today.format("%Y-%m-%d").to_string());

    let resolved_due = match due.as_deref() {
        Some("none") => None,
        Some(_) => due,
        None => {
            let scheduled_date =
                chrono::NaiveDate::parse_from_str(&resolved_scheduled, "%Y-%m-%d").ok();
            scheduled_date.and_then(|date| resolve_default_due_date(due_code, date))
        }
    };

    let default_tags = effective_default_tags(&settings.defaults, project_chain);
    let final_tags = merge_tags(tags.unwrap_or_default(), default_tags);

    (final_tags, resolved_due, resolved_scheduled)
}
```

- [ ] **Step 4: Update `build_series_occurrence` and `generate_series_occurrences` to take a chain**

Find:

```rust
fn build_series_occurrence(
    series: &Series,
    settings: &Settings,
    project_board: Option<&ProjectBoard>,
    date: chrono::NaiveDate,
) -> Task {
    let mut task = Task::new(series.title.clone());
    task.status = resolve_default_status(settings, project_board);
```

Replace with:

```rust
fn build_series_occurrence(
    series: &Series,
    settings: &Settings,
    project_chain: &[&Project],
    date: chrono::NaiveDate,
) -> Task {
    let mut task = Task::new(series.title.clone());
    task.status = resolve_default_status(settings, project_chain);
```

(The rest of `build_series_occurrence`'s body is unchanged — only the parameter and the call to `resolve_default_status` change.)

Find:

```rust
fn generate_series_occurrences(
    tasks_dir: &Path,
    settings: &Settings,
    project_board: Option<&ProjectBoard>,
    series: &mut Series,
    through: chrono::NaiveDate,
) -> Result<Vec<Task>, String> {
    let generated_until = chrono::NaiveDate::parse_from_str(&series.generated_until, "%Y-%m-%d")
        .map_err(|_| "series has an invalid generated_until date".to_string())?;

    let dates = occurrence_dates_in_range(series, generated_until, through);
    let mut created = Vec::with_capacity(dates.len());
    for date in &dates {
        let task = build_series_occurrence(series, settings, project_board, *date);
        storage::save_task(tasks_dir, &task).map_err(|e| e.to_string())?;
        created.push(task);
    }
```

Replace with:

```rust
fn generate_series_occurrences(
    tasks_dir: &Path,
    settings: &Settings,
    project_chain: &[&Project],
    series: &mut Series,
    through: chrono::NaiveDate,
) -> Result<Vec<Task>, String> {
    let generated_until = chrono::NaiveDate::parse_from_str(&series.generated_until, "%Y-%m-%d")
        .map_err(|_| "series has an invalid generated_until date".to_string())?;

    let dates = occurrence_dates_in_range(series, generated_until, through);
    let mut created = Vec::with_capacity(dates.len());
    for date in &dates {
        let task = build_series_occurrence(series, settings, project_chain, *date);
        storage::save_task(tasks_dir, &task).map_err(|e| e.to_string())?;
        created.push(task);
    }
```

(The rest of the body, including the `generated_until` watermark logic below, is unchanged.) Update any existing tests for `build_series_occurrence`/`generate_series_occurrences` the same way as Step 1 (wrap a `ProjectBoard` in a `Project`, build a `Vec<&Project>` chain — use `&[]` where the test currently passes `None`).

- [ ] **Step 5: Update `create_task`'s call site**

Find:

```rust
    let project_name = resolve_project_name(project, &settings);

    let projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
    let matched_project = find_project(&projects, &project_name);
    let project_board = matched_project.map(|p| &p.board);
    let project_defaults = matched_project.map(|p| &p.defaults);
    let status = match status {
        Some(id) => {
            validate_status_id(&settings, &id)?;
            id
        }
        None => resolve_default_status(&settings, project_board),
    };

    // Use the user's local date so relative-date defaults (e.g. "today",
    // "tomorrow") match the day they see on their device's calendar,
    // regardless of the machine's UTC offset.
    let today = chrono::Local::now().date_naive();
    let (final_tags, resolved_due, resolved_scheduled) =
        resolve_creation_defaults(&settings, project_defaults, today, tags, due, scheduled);
    let resolved_estimated_minutes = estimated_minutes
        .or_else(|| effective_default_estimated_minutes(&settings.defaults, project_defaults));
```

Replace with:

```rust
    let project_name = resolve_project_name(project, &settings);

    let projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
    let matched_project = find_project(&projects, &project_name);
    let project_chain: Vec<&Project> = matched_project
        .map(|p| project_tree::self_and_ancestors(&projects, &p.id))
        .unwrap_or_default();
    let status = match status {
        Some(id) => {
            validate_status_id(&settings, &id)?;
            id
        }
        None => resolve_default_status(&settings, &project_chain),
    };

    // Use the user's local date so relative-date defaults (e.g. "today",
    // "tomorrow") match the day they see on their device's calendar,
    // regardless of the machine's UTC offset.
    let today = chrono::Local::now().date_naive();
    let (final_tags, resolved_due, resolved_scheduled) =
        resolve_creation_defaults(&settings, &project_chain, today, tags, due, scheduled);
    let resolved_estimated_minutes = estimated_minutes
        .or_else(|| effective_default_estimated_minutes(&settings.defaults, &project_chain));
```

- [ ] **Step 6: Update `create_recurring_task`'s call site**

Find:

```rust
    let project_name = resolve_project_name(project, &settings);

    let projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
    let matched_project = find_project(&projects, &project_name);
    let project_board = matched_project.map(|p| &p.board);
    let project_defaults = matched_project.map(|p| &p.defaults);
    let status = match status {
        Some(id) => {
            validate_status_id(&settings, &id)?;
            id
        }
        None => resolve_default_status(&settings, project_board),
    };

    let today = chrono::Local::now().date_naive();
    // `resolve_creation_defaults`'s own due resolution is discarded here —
    // it only knows about the literal `due` string, not `due_rule`, and
    // would otherwise resolve the first occurrence's due from the
    // configured default code regardless of what `due_rule` says, while
    // every later occurrence correctly uses `series.due_rule` (see
    // `build_series_occurrence`) — the exact inconsistency this whole
    // feature exists to fix, just one occurrence later than before.
    let (final_tags, _due_resolution_unused_for_recurring_tasks, resolved_scheduled) =
        resolve_creation_defaults(&settings, project_defaults, today, tags, due, scheduled);
    let resolved_estimated_minutes = estimated_minutes
        .or_else(|| effective_default_estimated_minutes(&settings.defaults, project_defaults));
    let series_due_rule = due_rule.unwrap_or_else(|| {
        effective_default_code(&settings.defaults.due, project_defaults.map(|d| &d.due))
            .map(|code| DueRule::DefaultCode { code: code.clone() })
            .unwrap_or(DueRule::Never)
    });
```

Replace with:

```rust
    let project_name = resolve_project_name(project, &settings);

    let projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
    let matched_project = find_project(&projects, &project_name);
    let project_chain: Vec<&Project> = matched_project
        .map(|p| project_tree::self_and_ancestors(&projects, &p.id))
        .unwrap_or_default();
    let status = match status {
        Some(id) => {
            validate_status_id(&settings, &id)?;
            id
        }
        None => resolve_default_status(&settings, &project_chain),
    };

    let today = chrono::Local::now().date_naive();
    // `resolve_creation_defaults`'s own due resolution is discarded here —
    // it only knows about the literal `due` string, not `due_rule`, and
    // would otherwise resolve the first occurrence's due from the
    // configured default code regardless of what `due_rule` says, while
    // every later occurrence correctly uses `series.due_rule` (see
    // `build_series_occurrence`) — the exact inconsistency this whole
    // feature exists to fix, just one occurrence later than before.
    let (final_tags, _due_resolution_unused_for_recurring_tasks, resolved_scheduled) =
        resolve_creation_defaults(&settings, &project_chain, today, tags, due, scheduled);
    let resolved_estimated_minutes = estimated_minutes
        .or_else(|| effective_default_estimated_minutes(&settings.defaults, &project_chain));
    let series_due_rule = due_rule.unwrap_or_else(|| {
        project_chain
            .iter()
            .find_map(|p| p.defaults.due.as_ref())
            .or(settings.defaults.due.as_ref())
            .map(|code| DueRule::DefaultCode { code: code.clone() })
            .unwrap_or(DueRule::Never)
    });
```

A few lines further down in the same function, find:

```rust
    let horizon = anchor + chrono::Duration::days(RECURRENCE_BASELINE_LOOKAHEAD_DAYS);
    let mut created = generate_series_occurrences(
        &state.tasks_dir,
        &settings,
        project_board,
        &mut series,
        horizon,
    )?;
```

Replace with:

```rust
    let horizon = anchor + chrono::Duration::days(RECURRENCE_BASELINE_LOOKAHEAD_DAYS);
    let mut created = generate_series_occurrences(
        &state.tasks_dir,
        &settings,
        &project_chain,
        &mut series,
        horizon,
    )?;
```

- [ ] **Step 7: Update `ensure_occurrences_until`'s call site**

Find:

```rust
    let project_board = series
        .project
        .as_deref()
        .and_then(|name| find_project(&projects, name))
        .map(|p| &p.board);

    let created = generate_series_occurrences(
        &state.tasks_dir,
        &settings,
        project_board,
        series,
        through_date,
    )?;
```

Replace with:

```rust
    let project_chain: Vec<&Project> = series
        .project
        .as_deref()
        .and_then(|name| find_project(&projects, name))
        .map(|p| project_tree::self_and_ancestors(&projects, &p.id))
        .unwrap_or_default();

    let created = generate_series_occurrences(
        &state.tasks_dir,
        &settings,
        &project_chain,
        series,
        through_date,
    )?;
```

- [ ] **Step 8: Add new tests proving the chain actually walks multiple levels**

Add to the `#[cfg(test)] mod tests` block in `commands.rs`:

```rust
    #[test]
    fn resolve_default_status_falls_through_to_grandparent_when_parent_has_no_override() {
        let settings = Settings::default();
        let mut grandparent = Project::new("Grandparent".to_string(), "#111111".to_string(), 1);
        grandparent.board.default_status = Some("doing".to_string());
        let parent = Project::new("Parent".to_string(), "#222222".to_string(), 2);
        let child = Project::new("Child".to_string(), "#333333".to_string(), 3);
        let chain = vec![&child, &parent, &grandparent];

        let result = resolve_default_status(&settings, &chain);

        assert_eq!(result, "doing");
    }

    #[test]
    fn resolve_default_status_prefers_nearest_override_over_a_further_one() {
        let settings = Settings::default();
        let mut grandparent = Project::new("Grandparent".to_string(), "#111111".to_string(), 1);
        grandparent.board.default_status = Some("doing".to_string());
        let mut parent = Project::new("Parent".to_string(), "#222222".to_string(), 2);
        parent.board.default_status = Some("blocked".to_string());
        let child = Project::new("Child".to_string(), "#333333".to_string(), 3);
        let chain = vec![&child, &parent, &grandparent];

        let result = resolve_default_status(&settings, &chain);

        assert_eq!(result, "blocked");
    }

    #[test]
    fn effective_default_tags_falls_through_to_grandparent() {
        let global = TaskDefaults::default();
        let mut grandparent = Project::new("Grandparent".to_string(), "#111111".to_string(), 1);
        grandparent.defaults.tags = vec!["inherited".to_string()];
        let parent = Project::new("Parent".to_string(), "#222222".to_string(), 2);
        let child = Project::new("Child".to_string(), "#333333".to_string(), 3);
        let chain = vec![&child, &parent, &grandparent];

        let result = effective_default_tags(&global, &chain);

        assert_eq!(result, vec!["inherited".to_string()]);
    }

    #[test]
    fn effective_default_estimated_minutes_falls_through_to_grandparent() {
        let global = TaskDefaults::default();
        let mut grandparent = Project::new("Grandparent".to_string(), "#111111".to_string(), 1);
        grandparent.defaults.estimated_minutes = Some(90);
        let parent = Project::new("Parent".to_string(), "#222222".to_string(), 2);
        let child = Project::new("Child".to_string(), "#333333".to_string(), 3);
        let chain = vec![&child, &parent, &grandparent];

        let result = effective_default_estimated_minutes(&global, &chain);

        assert_eq!(result, Some(90));
    }

    #[test]
    fn resolve_creation_defaults_due_code_falls_through_to_grandparent() {
        let settings = Settings::default();
        let mut grandparent = Project::new("Grandparent".to_string(), "#111111".to_string(), 1);
        grandparent.defaults.due = Some("next_day".to_string());
        let parent = Project::new("Parent".to_string(), "#222222".to_string(), 2);
        let child = Project::new("Child".to_string(), "#333333".to_string(), 3);
        let chain = vec![&child, &parent, &grandparent];
        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 20).unwrap();

        let (_, resolved_due, resolved_scheduled) =
            resolve_creation_defaults(&settings, &chain, today, None, None, None);

        assert_eq!(resolved_scheduled, "2026-06-20");
        assert_eq!(resolved_due, Some("2026-06-21".to_string()));
    }
```

- [ ] **Step 9: Run the full test suite to confirm everything passes**

Run: `cd src-tauri && cargo test --lib`
Expected: all tests pass, including the 4 new chain-specific tests and every updated existing test from Step 1.

- [ ] **Step 10: Lint**

Run: `cd src-tauri && cargo fmt -- --check && cargo clippy --lib -- -D warnings`
Expected: no diffs, no warnings.

- [ ] **Step 11: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: project settings/defaults resolve through the full ancestor chain"
```

---

## Task 4: Cascade delete

**Files:**
- Modify: `src-tauri/src/commands.rs` (`tasks_for_project` → `tasks_for_projects`, `apply_task_strategy`, `DeleteProjectResult`, `delete_project`, plus a new `projects_to_delete` helper)

**Interfaces:**
- Consumes: `project_tree::descendants_of` (Task 1).
- Produces: `fn projects_to_delete(projects: &[Project], project_id: &str) -> Vec<Project>` (new, directly testable — the target project plus every descendant, owned; empty if `project_id` doesn't exist). `delete_project` now deletes the whole subtree, not just one project, and `DeleteProjectResult` gains `deleted_subprojects: usize`.

Per this codebase's established convention, `#[tauri::command]`-wrapped functions (`delete_project` itself) are not tested directly — this task extracts the "what's in the doomed subtree" logic into the new pure `projects_to_delete` helper specifically so it has direct test coverage.

- [ ] **Step 1: Write the failing tests for `projects_to_delete`**

Add to the `#[cfg(test)] mod tests` block in `commands.rs`:

```rust
    #[test]
    fn projects_to_delete_with_no_descendants_returns_just_the_target() {
        let project = Project::new("Solo".to_string(), "#111111".to_string(), 1);
        let id = project.id.clone();
        let projects = vec![project];

        let doomed = projects_to_delete(&projects, &id);

        assert_eq!(doomed.len(), 1);
        assert_eq!(doomed[0].id, id);
    }

    #[test]
    fn projects_to_delete_includes_every_descendant() {
        let mut parent = Project::new("Parent".to_string(), "#111111".to_string(), 1);
        parent.id = "parent".to_string();
        let mut child = Project::new("Child".to_string(), "#222222".to_string(), 2);
        child.id = "child".to_string();
        child.parent_id = Some("parent".to_string());
        let mut grandchild = Project::new("Grandchild".to_string(), "#333333".to_string(), 3);
        grandchild.id = "grandchild".to_string();
        grandchild.parent_id = Some("child".to_string());
        let mut unrelated = Project::new("Unrelated".to_string(), "#444444".to_string(), 4);
        unrelated.id = "unrelated".to_string();
        let projects = vec![parent, child, grandchild, unrelated];

        let doomed = projects_to_delete(&projects, "parent");

        let mut ids: Vec<&str> = doomed.iter().map(|p| p.id.as_str()).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec!["child", "grandchild", "parent"]);
    }

    #[test]
    fn projects_to_delete_for_a_missing_id_is_empty() {
        let projects = vec![Project::new("Solo".to_string(), "#111111".to_string(), 1)];

        let doomed = projects_to_delete(&projects, "does-not-exist");

        assert!(doomed.is_empty());
    }
```

- [ ] **Step 2: Run the tests to confirm they fail**

Run: `cd src-tauri && cargo test --lib commands::tests::projects_to_delete`
Expected: FAIL with `cannot find function 'projects_to_delete'`.

- [ ] **Step 3: Implement `projects_to_delete`**

Add this function to `commands.rs`, directly above `delete_project`:

```rust
/// Returns the project identified by `project_id` together with all of its
/// descendants (see [`crate::project_tree::descendants_of`]) — the full set
/// of projects [`delete_project`] removes for a cascading delete. Empty if
/// `project_id` doesn't exist in `projects`.
fn projects_to_delete(projects: &[Project], project_id: &str) -> Vec<Project> {
    let Some(target) = projects.iter().find(|p| p.id == project_id) else {
        return Vec::new();
    };
    std::iter::once(target.clone())
        .chain(
            project_tree::descendants_of(projects, project_id)
                .into_iter()
                .cloned(),
        )
        .collect()
}
```

- [ ] **Step 4: Run the tests to confirm they pass**

Run: `cd src-tauri && cargo test --lib commands::tests::projects_to_delete`
Expected: all 3 tests pass.

- [ ] **Step 5: Rename `tasks_for_project` to `tasks_for_projects` (plural, multi-name)**

Find:

```rust
/// Returns the tasks currently filed under `project_name` (matched
/// case-insensitively, mirroring [`find_project`]) - i.e. those that need to
/// be reassigned, archived, or deleted before `project_name` can itself be
/// deleted.
fn tasks_for_project<'a>(tasks: &'a [Task], project_name: &str) -> Vec<&'a Task> {
    tasks
        .iter()
        .filter(|t| {
            t.project
                .as_deref()
                .is_some_and(|p| p.eq_ignore_ascii_case(project_name))
        })
        .collect()
}
```

Replace with:

```rust
/// Returns the tasks currently filed under any name in `project_names`
/// (matched case-insensitively, mirroring [`find_project`]) - i.e. those
/// that need to be reassigned, archived, or deleted before the
/// corresponding projects can themselves be deleted.
fn tasks_for_projects<'a>(tasks: &'a [Task], project_names: &[String]) -> Vec<&'a Task> {
    tasks
        .iter()
        .filter(|t| {
            t.project
                .as_deref()
                .is_some_and(|p| project_names.iter().any(|name| p.eq_ignore_ascii_case(name)))
        })
        .collect()
}
```

Find the existing test(s) for `tasks_for_project` (search `grep -n "fn tasks_for_project" src-tauri/src/commands.rs` for the test names, likely around line 2934-2950 based on the surrounding `homework`/`homework_upper`/`other` fixtures already in the file) and update each call from `tasks_for_project(&tasks, "Homework")` to `tasks_for_projects(&tasks, &["Homework".to_string()])` (wrap the single name in a one-element `Vec<String>`). Add one new test confirming multi-name matching:

```rust
    #[test]
    fn tasks_for_projects_matches_any_of_several_names() {
        let mut homework = Task::new("Read chapter 1".to_string());
        homework.project = Some("Homework".to_string());
        let mut chores = Task::new("Clean room".to_string());
        chores.project = Some("Chores".to_string());
        let mut other = Task::new("Plan trip".to_string());
        other.project = Some("Vacation".to_string());
        let tasks = vec![homework, chores, other];

        let matching = tasks_for_projects(&tasks, &["Homework".to_string(), "Chores".to_string()]);

        assert_eq!(matching.len(), 2);
    }
```

- [ ] **Step 6: Update `apply_task_strategy`'s reassign-target check to reject any project in the doomed set**

Find:

```rust
fn apply_task_strategy(
    tasks_dir: &Path,
    archive_dir: &Path,
    projects: &[Project],
    deleted_project_id: &str,
    tasks: &[&Task],
    strategy: &ProjectTaskStrategy,
) -> Result<usize, String> {
    match strategy {
        ProjectTaskStrategy::Reassign { target_project_id } => {
            if target_project_id == deleted_project_id {
                return Err("cannot reassign tasks to the project being deleted".to_string());
            }
```

Replace with:

```rust
fn apply_task_strategy(
    tasks_dir: &Path,
    archive_dir: &Path,
    projects: &[Project],
    deleted_project_ids: &[String],
    tasks: &[&Task],
    strategy: &ProjectTaskStrategy,
) -> Result<usize, String> {
    match strategy {
        ProjectTaskStrategy::Reassign { target_project_id } => {
            if deleted_project_ids.contains(target_project_id) {
                return Err("cannot reassign tasks to a project being deleted".to_string());
            }
```

The rest of `apply_task_strategy`'s body is unchanged. Find its existing test(s) (search `grep -n "fn apply_task_strategy" src-tauri/src/commands.rs`) and update each call site's `deleted_project_id` argument (a single `&str`) to `deleted_project_ids` (a `&[String]`, e.g. `&["deleted-id".to_string()]`). Add one new test:

```rust
    #[test]
    fn apply_task_strategy_rejects_reassigning_to_any_doomed_descendant() {
        let dir = tempfile::tempdir().unwrap();
        let archive_dir = dir.path().join("archive");
        let target = Project::new("Descendant".to_string(), "#111111".to_string(), 1);
        let target_id = target.id.clone();
        let projects = vec![target];
        let task = Task::new("Demo".to_string());
        let tasks: Vec<&Task> = vec![&task];

        let result = apply_task_strategy(
            dir.path(),
            &archive_dir,
            &projects,
            &["parent-id".to_string(), target_id.clone()],
            &tasks,
            &ProjectTaskStrategy::Reassign {
                target_project_id: target_id,
            },
        );

        assert!(result.is_err());
    }
```

- [ ] **Step 7: Rewrite `DeleteProjectResult` and `delete_project`**

Find:

```rust
#[derive(Debug, Serialize)]
pub struct DeleteProjectResult {
    pub affected_tasks: usize,
}

/// Deletes the project identified by `project_id`. The configured default
/// project can never be deleted (see [`ensure_not_default_project`]). If the
/// project still has tasks (matched by name, case-insensitively - see
/// [`tasks_for_project`]), `task_strategy` is required and is applied to all
/// of them (see [`apply_task_strategy`]) before the project itself is removed
/// from the projects file. Tasks already moved to the archive don't count
/// toward this check and never block deletion.
#[tauri::command]
pub fn delete_project(
    state: State<AppState>,
    project_id: String,
    task_strategy: Option<ProjectTaskStrategy>,
) -> Result<DeleteProjectResult, String> {
    let settings =
        settings_storage::load_settings(&state.settings_file).map_err(|e| e.to_string())?;

    let _guard = state.projects_lock.lock().map_err(|e| e.to_string())?;
    let mut projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;

    let index = projects
        .iter()
        .position(|p| p.id == project_id)
        .ok_or_else(|| format!("project '{project_id}' not found"))?;
    ensure_not_default_project(&projects[index], &settings)?;

    let tasks = storage::list_tasks(&state.tasks_dir).map_err(|e| e.to_string())?;
    let matching = tasks_for_project(&tasks, &projects[index].name);

    let affected_tasks = if matching.is_empty() {
        0
    } else {
        let strategy = task_strategy.ok_or_else(|| {
            "this project still has tasks; choose how to handle them before deleting it".to_string()
        })?;
        apply_task_strategy(
            &state.tasks_dir,
            &state.archive_dir,
            &projects,
            &project_id,
            &matching,
            &strategy,
        )?
    };

    projects.remove(index);
    project_storage::save_projects(&state.projects_file, &projects).map_err(|e| e.to_string())?;

    Ok(DeleteProjectResult { affected_tasks })
}
```

Replace with:

```rust
#[derive(Debug, Serialize)]
pub struct DeleteProjectResult {
    pub affected_tasks: usize,
    /// How many descendant subprojects were deleted along with the
    /// requested project (0 if it had none).
    pub deleted_subprojects: usize,
}

/// Deletes the project identified by `project_id` together with every
/// descendant subproject (see [`projects_to_delete`]) — a cascading delete.
/// The configured default project can never be deleted, nor can any project
/// whose subtree contains it (see [`ensure_not_default_project`]). If the
/// project or any descendant still has tasks (matched by name,
/// case-insensitively - see [`tasks_for_projects`]), `task_strategy` is
/// required and is applied to all of them together (see
/// [`apply_task_strategy`]) before every project in the subtree is removed
/// from the projects file. Tasks already moved to the archive don't count
/// toward this check and never block deletion.
#[tauri::command]
pub fn delete_project(
    state: State<AppState>,
    project_id: String,
    task_strategy: Option<ProjectTaskStrategy>,
) -> Result<DeleteProjectResult, String> {
    let settings =
        settings_storage::load_settings(&state.settings_file).map_err(|e| e.to_string())?;

    let _guard = state.projects_lock.lock().map_err(|e| e.to_string())?;
    let mut projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;

    let doomed = projects_to_delete(&projects, &project_id);
    if doomed.is_empty() {
        return Err(format!("project '{project_id}' not found"));
    }
    for project in &doomed {
        ensure_not_default_project(project, &settings)?;
    }

    let doomed_ids: Vec<String> = doomed.iter().map(|p| p.id.clone()).collect();
    let doomed_names: Vec<String> = doomed.iter().map(|p| p.name.clone()).collect();
    let deleted_subprojects = doomed.len() - 1;

    let tasks = storage::list_tasks(&state.tasks_dir).map_err(|e| e.to_string())?;
    let matching = tasks_for_projects(&tasks, &doomed_names);

    let affected_tasks = if matching.is_empty() {
        0
    } else {
        let strategy = task_strategy.ok_or_else(|| {
            "this project still has tasks; choose how to handle them before deleting it".to_string()
        })?;
        apply_task_strategy(
            &state.tasks_dir,
            &state.archive_dir,
            &projects,
            &doomed_ids,
            &matching,
            &strategy,
        )?
    };

    projects.retain(|p| !doomed_ids.contains(&p.id));
    project_storage::save_projects(&state.projects_file, &projects).map_err(|e| e.to_string())?;

    Ok(DeleteProjectResult {
        affected_tasks,
        deleted_subprojects,
    })
}
```

- [ ] **Step 8: Run the full test suite**

Run: `cd src-tauri && cargo test --lib`
Expected: all tests pass, including every renamed/updated test from Steps 1-6.

- [ ] **Step 9: Lint**

Run: `cd src-tauri && cargo fmt -- --check && cargo clippy --lib -- -D warnings`
Expected: no diffs, no warnings.

- [ ] **Step 10: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: deleting a project cascades to its full subproject/task subtree"
```

---

## Task 5: Switch `Task`/`Series`/`Settings` from name-based to id-based project linkage

**Files:**
- Modify: `src-tauri/src/task.rs`, `src-tauri/src/series.rs`, `src-tauri/src/settings.rs`, `src-tauri/src/commands.rs`

**Interfaces:**
- Produces: `Task.project_id: Option<String>` (was `project: Option<String>`, a name), `Series.project_id: Option<String>` (was `project`), `Settings.default_project_id: String` (was `default_project`, a name). `resolve_project_name`/`find_project`-by-name/`backfill_projects` are removed; replaced by `resolve_project_id` and an id-based `find_project`.

This is the largest task in this plan and must land as one unit — Rust requires the whole crate to compile, and every one of these renames touches call sites across all four files simultaneously. Work through the steps in order; the crate will not compile between steps within this task (that's expected — only run the build/test commands at the designated checkpoints, not after every single edit).

**Why `backfill_projects` is removed, not adapted:** today, if a task's `project` name doesn't match any existing `Project`, `list_projects` auto-creates one with that name the next time it's called — a safety net against name drift (e.g. a project rename leaves old tasks holding the stale name, and a phantom project gets silently recreated under that stale name). Once linkage is by id, a task's `project_id` keeps pointing at the same project through a rename (the id never changes), so that drift can no longer happen, and "auto-create a project for an unrecognized id" isn't a sensible operation in the first place (ids aren't human-chosen, so there's nothing meaningful to name the phantom project). The safety net's reason to exist goes away along with the bug class it was protecting against.

**Why `Settings.default_project_id` seeds as an empty string:** `Settings::default()` runs before any `Project` necessarily exists (e.g. the very first launch, before `projects.json` exists), so it cannot seed a real id the way the old name-based field could seed a placeholder string like `"General"`. The empty string is a transient sentinel — Task 6's startup migration always runs before the app becomes interactive and is responsible for ensuring a real default project exists and `default_project_id` is set to its id, the same way `storage::migrate_scheduled_dates` already runs unconditionally in `lib.rs`'s `.setup()` before anything else. `validate_settings` (called only on the `save_settings` command path, never on load) still rejects an empty value, since by the time a user can reach Settings to save, migration has already run.

- [ ] **Step 1: Rename `Task.project` to `Task.project_id`**

Edit `src-tauri/src/task.rs`. Find:

```rust
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
```

Replace with:

```rust
    /// The id of the `Project` this task belongs to. `None` only
    /// transiently — every task gets a concrete project id at creation
    /// time (see `commands::resolve_project_id`), falling back to
    /// `Settings.default_project_id` when none is given explicitly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
```

Find (in `Task::new`):

```rust
            status: default_status(),
            project: None,
```

Replace with:

```rust
            status: default_status(),
            project_id: None,
```

Find (in the `#[cfg(test)] mod tests` block, `new_task_has_sensible_defaults`):

```rust
        assert!(task.project.is_none());
```

Replace with:

```rust
        assert!(task.project_id.is_none());
```

Find (in `to_markdown_then_from_markdown_round_trips`):

```rust
        task.project = Some("CS101/Homework".to_string());
```

Replace with:

```rust
        task.project_id = Some("project-id-cs101-homework".to_string());
```

Find (in `from_markdown_applies_defaults_for_missing_optional_fields`):

```rust
        assert!(task.project.is_none());
```

Replace with:

```rust
        assert!(task.project_id.is_none());
```

- [ ] **Step 2: Rename `Series.project` to `Series.project_id`**

Edit `src-tauri/src/series.rs`. Find:

```rust
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    pub priority: String,
```

Replace with:

```rust
    pub title: String,
    /// The id of the `Project` this series' occurrences belong to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub priority: String,
```

Find (in `Series::new`'s parameter list):

```rust
        title: String,
        project: Option<String>,
        priority: String,
```

Replace with:

```rust
        title: String,
        project_id: Option<String>,
        priority: String,
```

Find (in `Series::new`'s body, the struct literal):

```rust
            title,
            project,
            priority,
```

Replace with:

```rust
            title,
            project_id,
            priority,
```

Search `series.rs` for any test referencing `project:` (e.g. inside a `new_series`-style test helper or a direct `Series { ... }`/`Series::new(...)` call) via `grep -n "project" src-tauri/src/series.rs` and rename each to `project_id:`/positionally-unchanged (since `Series::new` is called positionally, no test call site needs an edit beyond what the parameter rename already covers — only direct struct-literal construction, if any, needs the field name updated).

- [ ] **Step 3: Run task.rs and series.rs tests to confirm they pass**

Run: `cd src-tauri && cargo test --lib task::tests series::tests`
Expected: all tests pass.

- [ ] **Step 4: Rename `Settings.default_project` to `Settings.default_project_id`**

Edit `src-tauri/src/settings.rs`. Find:

```rust
/// Fallback project name for newly-created tasks that don't specify a
/// project, used as the seeded value of `Settings.default_project` and to
/// fill in `settings.json` files written before that field existed.
fn default_project_name() -> String {
    "General".to_string()
}
```

Delete this function entirely (no replacement — see Step 6 below for what replaces its one call site).

Find:

```rust
/// `default_project` names the project a new task is filed under when no
```

Replace with (update the doc comment on the `Settings` struct itself — search for the surrounding doc block and update its prose, then the field declaration below it):

```rust
/// `default_project_id` is the id of the project a new task is filed under
/// when no
```

Find:

```rust
    #[serde(default = "default_project_name")]
    pub default_project: String,
```

Replace with:

```rust
    /// The id of the project a new task is filed under when no project is
    /// specified. Empty only transiently, before the startup migration (see
    /// `commands::migration` — Task 6) ensures a real default project
    /// exists and sets this to its id; `validate_settings` rejects an
    /// empty value on every save after that point.
    #[serde(default)]
    pub default_project_id: String,
```

Find (in `Settings::default()`):

```rust
            default_project: default_project_name(),
```

Replace with:

```rust
            default_project_id: String::new(),
```

(There are two occurrences of this line in `settings.rs` per the earlier grep — one in `impl Default for Settings` around line 240, one in a test fixture around line 1379. Apply the same replacement to both.)

- [ ] **Step 5: Update `validate_settings`'s default-project check**

Find:

```rust
    if settings.default_project.trim().is_empty() {
        return Err("a default project name must be defined".to_string());
    }
```

Replace with:

```rust
    if settings.default_project_id.is_empty() {
        return Err("a default project must be defined".to_string());
    }
```

- [ ] **Step 6: Update `validate_settings`'s existing tests**

Find (in `#[cfg(test)] mod tests`):

```rust
    fn validate_settings_rejects_an_empty_default_project() {
        let settings = Settings {
            default_project: String::new(),
```

Replace with:

```rust
    fn validate_settings_rejects_an_empty_default_project() {
        let settings = Settings {
            default_project_id: String::new(),
```

Find:

```rust
    fn validate_settings_rejects_a_whitespace_only_default_project() {
        let settings = Settings {
            default_project: "   ".to_string(),
```

Delete this entire test — a whitespace-only id is no longer a meaningfully distinct case from any other non-empty-but-invalid id (ids aren't trimmed, since they're never typed by a user). Its companion "rejects empty" test (kept above) covers the only validation this field still needs at this layer.

Find:

```rust
    fn validate_settings_accepts_a_valid_default_project() {
        let settings = Settings {
            default_project: "Inbox".to_string(),
```

Replace with:

```rust
    fn validate_settings_accepts_a_valid_default_project() {
        let settings = Settings {
            default_project_id: "some-project-id".to_string(),
```

Find (in `default_settings_seed_has_a_default_project_name_of_general` and its neighbor, around line 557-567):

```rust
    fn default_settings_seed_has_a_default_project_name_of_general() {
```

Rename the test and update its body — find the full test:

```rust
    fn default_settings_seed_has_a_default_project_name_of_general() {
        let settings = Settings::default();

        assert_eq!(settings.default_project, "General");
    }
```

Replace with:

```rust
    fn default_settings_seed_has_an_empty_default_project_id() {
        let settings = Settings::default();

        assert_eq!(settings.default_project_id, "");
    }
```

Find the second test referencing `settings.default_project, "General"` nearby (per the grep, there's a second hit at line 567 — likely a "round-trips through JSON with old field absent" style test) and apply the same `default_project` → `default_project_id`, `"General"` → `""` transformation, adjusting its name/doc comment if it specifically mentions backfilling from an absent field.

- [ ] **Step 7: Add a cross-validation check in `validate_settings_against_projects`**

Find:

```rust
fn validate_settings_against_projects(
    settings: &Settings,
    projects: &[Project],
) -> Result<(), String> {
    for project in projects {
```

Replace with:

```rust
fn validate_settings_against_projects(
    settings: &Settings,
    projects: &[Project],
) -> Result<(), String> {
    if !projects.iter().any(|p| p.id == settings.default_project_id) {
        return Err(format!(
            "default project '{}' does not exist",
            settings.default_project_id
        ));
    }

    for project in projects {
```

This function lives in `commands.rs`, not `settings.rs` — make this edit there, not in `settings.rs`. Find its existing tests (`grep -n "fn .*validate_settings_against_projects" src-tauri/src/commands.rs`) and add `settings.default_project_id = <a project id present in the test's fixture projects>;` to each existing test's `settings` setup so they don't start failing on the new check. Add one new test:

```rust
    #[test]
    fn validate_settings_against_projects_rejects_a_default_project_that_does_not_exist() {
        let mut settings = Settings::default();
        settings.default_project_id = "does-not-exist".to_string();
        let projects = vec![Project::new("Inbox".to_string(), "#111111".to_string(), 1)];

        let result = validate_settings_against_projects(&settings, &projects);

        assert!(result.is_err());
    }
```

- [ ] **Step 8: Run settings.rs and the updated commands.rs validation tests**

Run: `cd src-tauri && cargo test --lib settings::tests commands::tests::validate_settings`
Expected: all tests pass (the crate as a whole will not yet compile — `commands.rs`'s task-creation functions still reference the now-renamed `project`/`default_project` fields; that's fixed in the remaining steps. Running a scoped `cargo test --lib <module>::tests` instead of the full suite is expected to still fail at this checkpoint due to those other compile errors elsewhere in the same crate — Rust compiles the whole crate regardless of which test filter you pass. **Do not attempt to run any test command until Step 14** — Steps 8-13 below are a single non-compiling intermediate state; come back and run tests only once Step 13 is done.)

- [ ] **Step 9: Replace `resolve_project_name`/`find_project` with id-based equivalents**

Edit `src-tauri/src/commands.rs`. Find:

```rust
/// Looks up `project_name` case-insensitively among `projects`, or `None` if
/// no project matches (e.g. the named project doesn't exist yet and will be
/// backfilled by `list_projects`).
fn find_project<'a>(projects: &'a [Project], project_name: &str) -> Option<&'a Project> {
    projects
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(project_name))
}

/// Resolves the project name a task should be saved with: `project`, trimmed,
/// if it's non-empty, otherwise `settings.default_project`. Ensures a task can
/// never be created or updated with an empty/missing project.
fn resolve_project_name(project: Option<String>, settings: &Settings) -> String {
    match project.as_deref().map(str::trim) {
        Some(trimmed) if !trimmed.is_empty() => trimmed.to_string(),
        _ => settings.default_project.trim().to_string(),
    }
}
```

Replace with:

```rust
/// Looks up `project_id` among `projects`, or `None` if no project with
/// that id exists (e.g. it was deleted after the frontend last refreshed
/// its project list).
fn find_project<'a>(projects: &'a [Project], project_id: &str) -> Option<&'a Project> {
    projects.iter().find(|p| p.id == project_id)
}

/// Resolves the project id a task should be saved with: `project_id` if
/// it's `Some`, otherwise `settings.default_project_id`. Ensures a task can
/// never be created or updated without a project id.
fn resolve_project_id(project_id: Option<String>, settings: &Settings) -> String {
    project_id.unwrap_or_else(|| settings.default_project_id.clone())
}
```

- [ ] **Step 10: Update `apply_create_overrides`, `create_task`, `create_recurring_task`**

Find (`apply_create_overrides`'s signature and the relevant lines of its body):

```rust
fn apply_create_overrides(
    task: &mut Task,
    project: Option<String>,
    tags: Option<Vec<String>>,
    priority: Option<String>,
    due: Option<String>,
    scheduled: String,
    estimated_minutes: Option<u32>,
) {
    if let Some(project) = project {
        task.project = Some(project);
    }
```

Replace with:

```rust
fn apply_create_overrides(
    task: &mut Task,
    project_id: Option<String>,
    tags: Option<Vec<String>>,
    priority: Option<String>,
    due: Option<String>,
    scheduled: String,
    estimated_minutes: Option<u32>,
) {
    if let Some(project_id) = project_id {
        task.project_id = Some(project_id);
    }
```

Find `create_task`'s signature:

```rust
pub fn create_task(
    state: State<AppState>,
    title: String,
    project: Option<String>,
    tags: Option<Vec<String>>,
```

Replace with:

```rust
pub fn create_task(
    state: State<AppState>,
    title: String,
    project_id: Option<String>,
    tags: Option<Vec<String>>,
```

Find (in `create_task`'s body):

```rust
    let project_name = resolve_project_name(project, &settings);

    let projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
    let matched_project = find_project(&projects, &project_name);
    let project_chain: Vec<&Project> = matched_project
        .map(|p| project_tree::self_and_ancestors(&projects, &p.id))
        .unwrap_or_default();
```

Replace with:

```rust
    let resolved_project_id = resolve_project_id(project_id, &settings);

    let projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
    let matched_project = find_project(&projects, &resolved_project_id);
    let project_chain: Vec<&Project> = matched_project
        .map(|p| project_tree::self_and_ancestors(&projects, &p.id))
        .unwrap_or_default();
```

A little further down in `create_task`, find:

```rust
    apply_create_overrides(
        &mut task,
        Some(project_name),
        Some(final_tags),
```

Replace with:

```rust
    apply_create_overrides(
        &mut task,
        Some(resolved_project_id),
        Some(final_tags),
```

Find `create_recurring_task`'s signature:

```rust
pub fn create_recurring_task(
    state: State<AppState>,
    title: String,
    project: Option<String>,
    tags: Option<Vec<String>>,
```

Replace with:

```rust
pub fn create_recurring_task(
    state: State<AppState>,
    title: String,
    project_id: Option<String>,
    tags: Option<Vec<String>>,
```

Find (in `create_recurring_task`'s body, after Task 3's edits already changed the lines around it):

```rust
    let project_name = resolve_project_name(project, &settings);

    let projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
    let matched_project = find_project(&projects, &project_name);
    let project_chain: Vec<&Project> = matched_project
        .map(|p| project_tree::self_and_ancestors(&projects, &p.id))
        .unwrap_or_default();
```

Replace with:

```rust
    let resolved_project_id = resolve_project_id(project_id, &settings);

    let projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;
    let matched_project = find_project(&projects, &resolved_project_id);
    let project_chain: Vec<&Project> = matched_project
        .map(|p| project_tree::self_and_ancestors(&projects, &p.id))
        .unwrap_or_default();
```

Find (further down in `create_recurring_task`, the `Series::new` call):

```rust
    let mut series = Series::new(
        frequency,
        resolved_scheduled.clone(),
        end_date,
        series_due_rule,
        title.clone(),
        Some(project_name.clone()),
        priority.clone(),
        final_tags.clone(),
        resolved_estimated_minutes,
        String::new(),
    );
```

Replace with:

```rust
    let mut series = Series::new(
        frequency,
        resolved_scheduled.clone(),
        end_date,
        series_due_rule,
        title.clone(),
        Some(resolved_project_id.clone()),
        priority.clone(),
        final_tags.clone(),
        resolved_estimated_minutes,
        String::new(),
    );
```

Find (the `first_task`'s `apply_create_overrides` call, a little further down):

```rust
        apply_create_overrides(
            &mut first_task,
            Some(project_name),
            Some(final_tags),
```

Replace with:

```rust
        apply_create_overrides(
            &mut first_task,
            Some(resolved_project_id),
            Some(final_tags),
```

- [ ] **Step 11: Update `apply_task_update`, `update_task`, `apply_series_template_update`, `apply_template_to_occurrence`**

Find:

```rust
fn apply_task_update(existing: &mut Task, title: String, project_name: String, task: Task) {
    existing.title = title;
    existing.project = Some(project_name);
```

Replace with:

```rust
fn apply_task_update(existing: &mut Task, title: String, project_id: String, task: Task) {
    existing.title = title;
    existing.project_id = Some(project_id);
```

Find (in `update_task`):

```rust
    let project_name = resolve_project_name(task.project.clone(), &settings);
    let mut existing = storage::load_task(&state.tasks_dir, &task.id).map_err(|e| e.to_string())?;
    apply_task_update(&mut existing, title, project_name, task);
```

Replace with:

```rust
    let resolved_project_id = resolve_project_id(task.project_id.clone(), &settings);
    let mut existing = storage::load_task(&state.tasks_dir, &task.id).map_err(|e| e.to_string())?;
    apply_task_update(&mut existing, title, resolved_project_id, task);
```

There are two call sites for `resolve_project_name(task.project.clone(), ...)` per the earlier grep (`update_task` and `update_series_occurrence`) — apply the identical replacement (`resolve_project_id(task.project_id.clone(), &settings)`) at the second one too; find it with `grep -n "resolve_project_name(task.project.clone" src-tauri/src/commands.rs` to confirm both locations before editing.

Find:

```rust
fn apply_series_template_update(series: &mut Series, task: &Task) {
    series.title = task.title.clone();
    series.project = task.project.clone();
```

Replace with:

```rust
fn apply_series_template_update(series: &mut Series, task: &Task) {
    series.title = task.title.clone();
    series.project_id = task.project_id.clone();
```

Find:

```rust
fn apply_template_to_occurrence(occurrence: &mut Task, task: &Task) {
    occurrence.title = task.title.clone();
    occurrence.project = task.project.clone();
```

Replace with:

```rust
fn apply_template_to_occurrence(occurrence: &mut Task, task: &Task) {
    occurrence.title = task.title.clone();
    occurrence.project_id = task.project_id.clone();
```

- [ ] **Step 12: Update `ensure_not_default_project`, `tasks_for_projects`, `apply_task_strategy`'s reassign branch**

Find:

```rust
fn ensure_not_default_project(project: &Project, settings: &Settings) -> Result<(), String> {
    if project
        .name
        .eq_ignore_ascii_case(settings.default_project.trim())
    {
        Err(format!(
            "'{}' is the default project and cannot be deleted",
            project.name
        ))
    } else {
        Ok(())
    }
}
```

Replace with:

```rust
fn ensure_not_default_project(project: &Project, settings: &Settings) -> Result<(), String> {
    if project.id == settings.default_project_id {
        Err(format!(
            "'{}' is the default project and cannot be deleted",
            project.name
        ))
    } else {
        Ok(())
    }
}
```

Find (the version of `tasks_for_projects` from Task 4):

```rust
fn tasks_for_projects<'a>(tasks: &'a [Task], project_names: &[String]) -> Vec<&'a Task> {
    tasks
        .iter()
        .filter(|t| {
            t.project
                .as_deref()
                .is_some_and(|p| project_names.iter().any(|name| p.eq_ignore_ascii_case(name)))
        })
        .collect()
}
```

Replace with:

```rust
fn tasks_for_projects<'a>(tasks: &'a [Task], project_ids: &[String]) -> Vec<&'a Task> {
    tasks
        .iter()
        .filter(|t| {
            t.project_id
                .as_deref()
                .is_some_and(|id| project_ids.iter().any(|target| target == id))
        })
        .collect()
}
```

Update its tests from Task 4 (`tasks_for_projects_matches_any_of_several_names` and any renamed-from-`tasks_for_project` tests) to set `task.project_id = Some("some-id".to_string())` and pass plain id strings instead of names — the assertions themselves (counts) don't need to change, just the fixture values.

Find (in `apply_task_strategy`'s `Reassign` branch):

```rust
            let target = projects
                .iter()
                .find(|p| &p.id == target_project_id)
                .ok_or_else(|| format!("target project '{target_project_id}' not found"))?;
            for task in tasks {
                let mut updated = (*task).clone();
                updated.project = Some(target.name.clone());
                storage::update_task(tasks_dir, &updated).map_err(|e| e.to_string())?;
            }
```

Replace with:

```rust
            if !projects.iter().any(|p| &p.id == target_project_id) {
                return Err(format!("target project '{target_project_id}' not found"));
            }
            for task in tasks {
                let mut updated = (*task).clone();
                updated.project_id = Some(target_project_id.clone());
                storage::update_task(tasks_dir, &updated).map_err(|e| e.to_string())?;
            }
```

- [ ] **Step 13: Remove `backfill_projects` and simplify `list_projects`**

Find:

```rust
/// Ensures every distinct, non-empty `task.project` name has a corresponding
/// `Project` entry (matched case-insensitively against existing names).
/// Returns the (possibly extended) project list and whether any entries were
/// added, so callers can avoid rewriting the projects file when nothing
/// changed. New entries get fresh ids, [`DEFAULT_PROJECT_COLOR`], and
/// ascending `order` values starting just past the current maximum.
fn backfill_projects(existing: Vec<Project>, tasks: &[Task]) -> (Vec<Project>, bool) {
    let mut projects = existing;
    let mut next = next_order(&projects);
    let mut changed = false;
    let mut added_names: Vec<String> = Vec::new();

    for task in tasks {
        let Some(name) = task
            .project
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
        else {
            continue;
        };

        let already_known = projects.iter().any(|p| p.name.eq_ignore_ascii_case(name))
            || added_names
                .iter()
                .any(|added| added.eq_ignore_ascii_case(name));
        if already_known {
            continue;
        }

        added_names.push(name.to_string());
        projects.push(Project::new(
            name.to_string(),
            DEFAULT_PROJECT_COLOR.to_string(),
            next,
        ));
        next += 1;
        changed = true;
    }

    (projects, changed)
}
```

Delete this function entirely. Find its tests (`grep -n "fn backfill_projects" src-tauri/src/commands.rs` to locate every `#[test]` named `backfill_projects_*`) and delete each one — there were 4-5 per the earlier research (`backfill_projects_adds_missing_project_names_from_tasks`, `backfill_projects_dedupes_multiple_tasks_with_the_same_new_project_name`, and others around the same block).

Find `list_projects`:

```rust
#[tauri::command]
pub fn list_projects(state: State<AppState>) -> Result<Vec<Project>, String> {
    let tasks = storage::list_tasks(&state.tasks_dir).map_err(|e| e.to_string())?;

    let _guard = state.projects_lock.lock().map_err(|e| e.to_string())?;
    let existing =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;

    let (mut projects, changed) = backfill_projects(existing, &tasks);
    if changed {
        project_storage::save_projects(&state.projects_file, &projects)
            .map_err(|e| e.to_string())?;
    }

    projects.sort_by_key(|p| p.order);
    Ok(projects)
}
```

Replace with:

```rust
#[tauri::command]
pub fn list_projects(state: State<AppState>) -> Result<Vec<Project>, String> {
    let _guard = state.projects_lock.lock().map_err(|e| e.to_string())?;
    let mut projects =
        project_storage::list_projects(&state.projects_file).map_err(|e| e.to_string())?;

    projects.sort_by_key(|p| p.order);
    Ok(projects)
}
```

- [ ] **Step 14: Update every remaining test in `commands.rs`/`storage.rs` that sets `.project`/`default_project` by name**

Run `grep -n "\.project = Some(\|\.project,\|default_project:\|default_project," src-tauri/src/commands.rs src-tauri/src/storage.rs` to enumerate every remaining hit (the earlier full-codebase grep found roughly 25 such lines across `commands.rs` test fixtures and 6 in `storage.rs`). For each:
- `task.project = Some("SomeName".to_string())` → `task.project_id = Some("some-id".to_string())` (any placeholder string is fine — these are opaque round-trip/equality fixtures, not name-matching tests anymore).
- `assert_eq!(task.project, Some("X".to_string()))` → `assert_eq!(task.project_id, Some("x-id".to_string()))` (matching whatever placeholder the same test's setup now uses).
- `assert_eq!(series.project, ...)` / `assert_eq!(occurrence.project, ...)` → `.project_id`.
- Any test fixture that previously relied on name-based matching across two tasks with deliberately-different-cased names (e.g. `"Homework"` vs `"HOMEWORK"`, used to test `eq_ignore_ascii_case` behavior in the old `tasks_for_project`/`find_project`) should be simplified to plain distinct id strings — case-insensitivity is no longer a relevant concept for id comparison. Where such a test's entire purpose was demonstrating case-insensitive name matching (e.g. a test literally named `*_matches_case_insensitively`), delete it — there is no longer a case-insensitivity property to test for an id.

This step has no separate code listing because its edits are mechanical, repeated applications of the substitution rules above across many small, otherwise-unrelated test fixtures — apply them directly in `commands.rs`/`storage.rs` rather than treating each as a separate plan step.

- [ ] **Step 15: Run the full backend test suite**

Run: `cd src-tauri && cargo test --lib`
Expected: the crate compiles and every test passes. If any test still references the old `.project`/`.default_project`/`resolve_project_name`/`tasks_for_project`/`backfill_projects` names, the compiler error will name the exact file and line — fix it using the same substitution rules from Step 14 and re-run.

- [ ] **Step 16: Lint**

Run: `cd src-tauri && cargo fmt -- --check && cargo clippy --lib -- -D warnings`
Expected: no diffs, no warnings.

- [ ] **Step 17: Commit**

```bash
git add src-tauri/src/task.rs src-tauri/src/series.rs src-tauri/src/settings.rs src-tauri/src/commands.rs
git commit -m "feat: switch task/series/settings project linkage from name to id"
```

---

## Task 6: Startup migration

**Files:**
- Modify: `src-tauri/src/storage.rs` (new `migrate_task_project_names_to_ids` + `read_legacy_project_name`)
- Modify: `src-tauri/src/commands.rs` (new `ensure_default_project`)
- Modify: `src-tauri/src/lib.rs` (`.setup()` wiring)

**Interfaces:**
- Produces: `storage::migrate_task_project_names_to_ids(tasks_dir: &Path, projects: &[Project]) -> Result<(), StorageError>`, `commands::ensure_default_project(projects: Vec<Project>, settings: Settings) -> Option<(Vec<Project>, Settings)>` (returns `None` if nothing needed to change).

**Why a raw frontmatter read is required:** by the end of Task 5, `Task` no longer has a `project` field at all — only `project_id`. `Task::from_markdown` deserializes via `serde`, which silently *ignores* an unrecognized `project` key in old frontmatter rather than erroring or preserving it. By the time a legacy file has been parsed into a `Task`, the old project name is already gone. This migration must read the raw YAML frontmatter text itself, before any `Task` deserialization happens, to recover that name.

- [ ] **Step 1: Write the failing tests for `read_legacy_project_name`**

Add to the `#[cfg(test)] mod tests` block in `storage.rs` (the one that already contains `migrate_scheduled_dates`'s tests):

```rust
    #[test]
    fn read_legacy_project_name_extracts_the_project_key() {
        let content = "---\nid: abc\ntitle: Demo\nproject: Homework\ncreated: 2026-06-11T10:00:00+00:00\n---\n\nNotes.";

        let name = read_legacy_project_name(content);

        assert_eq!(name, Some("Homework".to_string()));
    }

    #[test]
    fn read_legacy_project_name_returns_none_when_absent() {
        let content = "---\nid: abc\ntitle: Demo\ncreated: 2026-06-11T10:00:00+00:00\n---\n\nNotes.";

        let name = read_legacy_project_name(content);

        assert_eq!(name, None);
    }

    #[test]
    fn read_legacy_project_name_returns_none_for_already_migrated_content() {
        let content = "---\nid: abc\ntitle: Demo\nproject_id: some-id\ncreated: 2026-06-11T10:00:00+00:00\n---\n\nNotes.";

        let name = read_legacy_project_name(content);

        assert_eq!(name, None);
    }

    #[test]
    fn read_legacy_project_name_returns_none_for_content_with_no_frontmatter() {
        let name = read_legacy_project_name("Just plain markdown, no frontmatter.");

        assert_eq!(name, None);
    }
```

- [ ] **Step 2: Run the tests to confirm they fail**

Run: `cd src-tauri && cargo test --lib storage::tests::read_legacy_project_name`
Expected: FAIL with `cannot find function 'read_legacy_project_name'`.

- [ ] **Step 3: Implement `read_legacy_project_name` and `migrate_task_project_names_to_ids`**

Edit `src-tauri/src/storage.rs`. Add near `migrate_scheduled_dates` (the two migrations are independent but conceptually siblings — place this one directly after it):

```rust
/// Extracts the legacy `project` frontmatter key (a plain project name
/// string, from before `Task.project_id` replaced it) directly from
/// `content`'s raw YAML frontmatter, without going through
/// `Task::from_markdown` — which can no longer see this key at all, since
/// `Task` itself has no `project` field anymore (only `project_id`).
/// Returns `None` if there's no frontmatter, no `project` key, or the key
/// isn't a plain string.
fn read_legacy_project_name(content: &str) -> Option<String> {
    let after_open = content.strip_prefix("---\n")?;
    let end = after_open.find("\n---")?;
    let frontmatter_yaml = &after_open[..end];
    let value: serde_yaml::Value = serde_yaml::from_str(frontmatter_yaml).ok()?;
    value.get("project")?.as_str().map(str::to_string)
}

/// One-time migration: rewrites every task markdown file in `tasks_dir`
/// that still has the legacy `project` frontmatter key to use `project_id`
/// instead, resolved by case-insensitive name lookup against `projects`
/// (see [`read_legacy_project_name`]). Idempotent — a task with no legacy
/// `project` key (already migrated, or created fresh after this field
/// existed) is left completely untouched, never rewritten. A task whose
/// legacy name doesn't match any project keeps `project_id` unset and gets
/// a logged warning naming the task and the unmatched name, rather than
/// silently losing the link — real user data is at stake here, so an
/// unresolvable reference must be visible, not swallowed.
pub fn migrate_task_project_names_to_ids(
    tasks_dir: &Path,
    projects: &[Project],
) -> Result<(), StorageError> {
    if !tasks_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(tasks_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }

        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) => {
                eprintln!(
                    "skipping unreadable task file {} during project-id migration: {err}",
                    path.display()
                );
                continue;
            }
        };

        let Some(legacy_name) = read_legacy_project_name(&content) else {
            continue;
        };

        let mut task = match Task::from_markdown(&content) {
            Ok(task) => task,
            Err(err) => {
                eprintln!(
                    "skipping unreadable task file {} during project-id migration: {err}",
                    path.display()
                );
                continue;
            }
        };

        match projects
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(&legacy_name))
        {
            Some(matched) => task.project_id = Some(matched.id.clone()),
            None => eprintln!(
                "task '{}' ({}) referenced unknown project '{legacy_name}' during project-id migration — leaving its project unset",
                task.title, task.id
            ),
        }

        save_task(tasks_dir, &task)?;
    }

    Ok(())
}
```

Add `use crate::project::Project;` to `storage.rs`'s imports if not already present (check with `grep -n "^use" src-tauri/src/storage.rs` — `Project` is likely not yet imported there since this is the first function in this file to need it).

- [ ] **Step 4: Run the tests to confirm they pass**

Run: `cd src-tauri && cargo test --lib storage::tests::read_legacy_project_name`
Expected: all 4 tests pass.

- [ ] **Step 5: Write the failing tests for `migrate_task_project_names_to_ids`**

Add to the same test block:

```rust
    #[test]
    fn migrate_task_project_names_to_ids_resolves_a_matching_project() {
        let dir = tempdir().unwrap();
        let mut task = Task::new("Read chapter 1".to_string());
        let task_id = task.id.clone();
        save_task(dir.path(), &task).unwrap();
        // Simulate a legacy file by writing the raw markdown directly,
        // bypassing `Task::to_markdown` (which would write `project_id`,
        // not the legacy `project` key).
        let legacy_content = format!(
            "---\nid: {task_id}\ntitle: Read chapter 1\nproject: Homework\ncreated: {}\n---\n\n",
            task.created
        );
        fs::write(dir.path().join(format!("{task_id}.md")), legacy_content).unwrap();
        task.project_id = None;

        let homework = Project::new("Homework".to_string(), "#111111".to_string(), 1);
        let homework_id = homework.id.clone();
        let projects = vec![homework];

        migrate_task_project_names_to_ids(dir.path(), &projects).unwrap();

        let migrated = load_task(dir.path(), &task_id).unwrap();
        assert_eq!(migrated.project_id, Some(homework_id));
    }

    #[test]
    fn migrate_task_project_names_to_ids_matches_case_insensitively() {
        let dir = tempdir().unwrap();
        let task_id = "task-1".to_string();
        let legacy_content = format!(
            "---\nid: {task_id}\ntitle: Demo\nproject: HOMEWORK\ncreated: 2026-06-11T10:00:00+00:00\n---\n\n"
        );
        fs::write(dir.path().join(format!("{task_id}.md")), legacy_content).unwrap();

        let homework = Project::new("Homework".to_string(), "#111111".to_string(), 1);
        let homework_id = homework.id.clone();
        let projects = vec![homework];

        migrate_task_project_names_to_ids(dir.path(), &projects).unwrap();

        let migrated = load_task(dir.path(), &task_id).unwrap();
        assert_eq!(migrated.project_id, Some(homework_id));
    }

    #[test]
    fn migrate_task_project_names_to_ids_leaves_unmatched_tasks_unset_without_failing() {
        let dir = tempdir().unwrap();
        let task_id = "task-1".to_string();
        let legacy_content = format!(
            "---\nid: {task_id}\ntitle: Demo\nproject: DoesNotExist\ncreated: 2026-06-11T10:00:00+00:00\n---\n\n"
        );
        fs::write(dir.path().join(format!("{task_id}.md")), legacy_content).unwrap();
        let projects: Vec<Project> = Vec::new();

        let result = migrate_task_project_names_to_ids(dir.path(), &projects);

        assert!(result.is_ok());
        let migrated = load_task(dir.path(), &task_id).unwrap();
        assert_eq!(migrated.project_id, None);
    }

    #[test]
    fn migrate_task_project_names_to_ids_is_idempotent() {
        let dir = tempdir().unwrap();
        let mut task = Task::new("Already migrated".to_string());
        task.project_id = Some("existing-id".to_string());
        save_task(dir.path(), &task).unwrap();
        let projects = vec![Project::new(
            "Unrelated".to_string(),
            "#222222".to_string(),
            1,
        )];

        migrate_task_project_names_to_ids(dir.path(), &projects).unwrap();

        let unchanged = load_task(dir.path(), &task.id).unwrap();
        assert_eq!(unchanged.project_id, Some("existing-id".to_string()));
    }

    #[test]
    fn migrate_task_project_names_to_ids_returns_ok_for_missing_directory() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");

        let result = migrate_task_project_names_to_ids(&missing, &[]);

        assert!(result.is_ok());
    }
```

- [ ] **Step 6: Run the tests to confirm they pass**

Run: `cd src-tauri && cargo test --lib storage::tests::migrate_task_project_names_to_ids`
Expected: all 5 tests pass.

- [ ] **Step 7: Write the failing tests for `ensure_default_project`**

Add to the `#[cfg(test)] mod tests` block in `commands.rs`:

```rust
    #[test]
    fn ensure_default_project_creates_one_when_none_exists() {
        let projects: Vec<Project> = Vec::new();
        let settings = Settings::default();

        let result = ensure_default_project(projects, settings);

        let (projects, settings) = result.expect("should have created a default project");
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "General");
        assert_eq!(settings.default_project_id, projects[0].id);
    }

    #[test]
    fn ensure_default_project_does_nothing_when_default_already_exists() {
        let project = Project::new("Inbox".to_string(), "#111111".to_string(), 1);
        let mut settings = Settings::default();
        settings.default_project_id = project.id.clone();
        let projects = vec![project];

        let result = ensure_default_project(projects, settings);

        assert!(result.is_none());
    }

    #[test]
    fn ensure_default_project_creates_one_when_configured_id_is_stale() {
        let project = Project::new("Inbox".to_string(), "#111111".to_string(), 1);
        let mut settings = Settings::default();
        settings.default_project_id = "a-deleted-project-id".to_string();
        let projects = vec![project];

        let result = ensure_default_project(projects, settings);

        let (projects, settings) = result.expect("should have created a default project");
        assert_eq!(projects.len(), 2);
        assert!(projects.iter().any(|p| p.id == settings.default_project_id));
    }
```

- [ ] **Step 8: Run the tests to confirm they fail**

Run: `cd src-tauri && cargo test --lib commands::tests::ensure_default_project`
Expected: FAIL with `cannot find function 'ensure_default_project'`.

- [ ] **Step 9: Implement `ensure_default_project`**

Add to `commands.rs`, directly above `list_projects`:

```rust
/// Ensures `settings.default_project_id` references a project that actually
/// exists, creating a new top-level "General" project (using
/// [`DEFAULT_PROJECT_COLOR`] and the next available `order`, via the same
/// [`next_order`] every other project-creation path uses) if it doesn't.
/// Covers both a brand-new install (`default_project_id` seeds as an empty
/// string — see `Settings::default`) and an upgrade from before this field
/// existed, where the id is just stale or never resolved. Returns
/// `Some((updated_projects, updated_settings))` if a project was created
/// (so the caller knows both files need saving), or `None` if
/// `default_project_id` already pointed at a real project.
fn ensure_default_project(
    projects: Vec<Project>,
    settings: Settings,
) -> Option<(Vec<Project>, Settings)> {
    if projects.iter().any(|p| p.id == settings.default_project_id) {
        return None;
    }

    let mut projects = projects;
    let mut settings = settings;
    let order = next_order(&projects);
    let project = Project::new("General".to_string(), DEFAULT_PROJECT_COLOR.to_string(), order);
    settings.default_project_id = project.id.clone();
    projects.push(project);
    Some((projects, settings))
}
```

- [ ] **Step 10: Run the tests to confirm they pass**

Run: `cd src-tauri && cargo test --lib commands::tests::ensure_default_project`
Expected: all 3 tests pass.

- [ ] **Step 11: Wire both migrations into `lib.rs`'s startup**

Edit `src-tauri/src/lib.rs`. Find:

```rust
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let tasks_dir = app_data_dir.join("tasks");
            let archive_dir = app_data_dir.join("archive");
            let projects_file = app_data_dir.join("projects.json");
            let settings_file = app_data_dir.join("settings.json");
            let series_file = app_data_dir.join("series.json");
            storage::migrate_scheduled_dates(&tasks_dir)?;
            app.manage(commands::AppState {
                tasks_dir,
                archive_dir,
                projects_file,
                settings_file,
                series_file,
                projects_lock: std::sync::Mutex::new(()),
                series_lock: std::sync::Mutex::new(()),
            });
            Ok(())
        })
```

Replace with:

```rust
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let tasks_dir = app_data_dir.join("tasks");
            let archive_dir = app_data_dir.join("archive");
            let projects_file = app_data_dir.join("projects.json");
            let settings_file = app_data_dir.join("settings.json");
            let series_file = app_data_dir.join("series.json");
            storage::migrate_scheduled_dates(&tasks_dir)?;

            let projects = project_storage::list_projects(&projects_file)?;
            let settings = settings_storage::load_settings(&settings_file)?;
            let projects = match commands::ensure_default_project(projects, settings) {
                Some((projects, settings)) => {
                    project_storage::save_projects(&projects_file, &projects)?;
                    settings_storage::save_settings(&settings_file, &settings)?;
                    projects
                }
                None => projects,
            };
            storage::migrate_task_project_names_to_ids(&tasks_dir, &projects)?;

            app.manage(commands::AppState {
                tasks_dir,
                archive_dir,
                projects_file,
                settings_file,
                series_file,
                projects_lock: std::sync::Mutex::new(()),
                series_lock: std::sync::Mutex::new(()),
            });
            Ok(())
        })
```

Find the module imports at the top of `lib.rs`:

```rust
mod commands;
mod project;
mod project_storage;
mod project_tree;
mod recurrence;
mod series;
mod series_storage;
mod settings;
mod settings_storage;
mod storage;
mod task;

use tauri::Manager;
```

Replace with (adds the two new `use` lines needed by the `.setup()` block above — `commands`/`storage` are already accessible as the crate's own modules, but `project_storage`/`settings_storage` need to be referenced by their full path or imported; check whether `lib.rs` already has `use crate::project_storage;`-style imports elsewhere — if `project_storage::list_projects(...)` and `settings_storage::load_settings(...)` are written using the module path directly as shown above, no additional `use` is needed beyond the existing `mod` declarations, since `lib.rs` is the crate root and can reference any of its own declared modules by path without importing them):

```rust
mod commands;
mod project;
mod project_storage;
mod project_tree;
mod recurrence;
mod series;
mod series_storage;
mod settings;
mod settings_storage;
mod storage;
mod task;

use tauri::Manager;
```

(No change needed here — included for clarity that the module list itself is already correct from Task 1's edit; `project_storage::`/`settings_storage::` in the `.setup()` body above resolve directly against the `mod` declarations already present.)

- [ ] **Step 12: Run the full backend suite**

Run: `cd src-tauri && cargo test --lib`
Expected: every test in the crate passes.

- [ ] **Step 13: Lint and build**

Run: `cd src-tauri && cargo fmt -- --check && cargo clippy --lib -- -D warnings && cargo build`
Expected: no diffs, no warnings, build succeeds (a full `cargo build`, not just `--lib`, confirms `lib.rs`'s `.setup()` closure — only exercised by the actual Tauri binary target, not `--lib` tests — compiles correctly).

- [ ] **Step 14: Commit**

```bash
git add src-tauri/src/storage.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: migrate legacy name-based project links to ids at startup"
```

This is the last backend task — at this point the Rust side is fully id-based, settings inheritance walks the ancestor chain, and deleting a project cascades through its subtree. Tasks 7 onward are all frontend.

---

## Task 7: Frontend identity swap — `types.ts`/`api.ts` and every call site

**Files:**
- Modify: `src/lib/types.ts`, `src/lib/api.ts`, `src/lib/naturalLanguage.ts`, `src/lib/projectColor.ts`, `src/lib/projectColor.test.ts`, `src/lib/deleteProject.ts`, `src/lib/components/AddTaskModal.svelte`, `src/lib/components/KanbanBoard.svelte`, `src/lib/components/TaskCard.svelte`, `src/lib/components/WeekBarItem.svelte`, `src/lib/components/DeleteProjectSection.svelte`, `src/lib/components/DefaultsSettings.svelte`, `src/routes/projects/[id]/+page.svelte`

**Interfaces:**
- Produces: `Project.parent_id?: string` (new), `Task.project_id?: string` (was `project`), `Settings.default_project_id: string` (was `default_project`). `createProject(name, color?, parentId?)`, `createTask(input, projectId?)`, `createRecurringTask(input, frequency, endDate, dueRule, projectId?)`.

Mirrors backend Task 5: `svelte-check` type-checks the whole frontend project, so every call site referencing the renamed fields must be fixed in this same task — there is no way to land a partial rename and still pass `npx svelte-check`.

- [ ] **Step 1: Update `types.ts`**

Edit `src/lib/types.ts`. Find:

```ts
export interface Task {
  id: string;
  title: string;
  /** The id of a user-defined `StatusDefinition` (see `Settings.statuses`). */
  status: string;
  project?: string;
  tags: string[];
```

Replace with:

```ts
export interface Task {
  id: string;
  title: string;
  /** The id of a user-defined `StatusDefinition` (see `Settings.statuses`). */
  status: string;
  /** The id of the `Project` this task belongs to. */
  project_id?: string;
  tags: string[];
```

Find:

```ts
export interface Project {
  id: string;
  name: string;
  color: string;
  order: number;
  created: string;
  board: ProjectBoard;
  defaults: TaskDefaults;
}
```

Replace with:

```ts
export interface Project {
  id: string;
  name: string;
  color: string;
  /** The id of this project's parent, or `undefined` for a top-level project. Nesting is arbitrary depth. */
  parent_id?: string;
  order: number;
  created: string;
  board: ProjectBoard;
  defaults: TaskDefaults;
}
```

Find (the doc comment + field, search around line 155-165 per the earlier research):

```ts
 * `default_project` names the project a new task is filed under when no
```

Update this prose to describe an id instead of a name (read the full surrounding comment with `grep -n -B2 -A2 "default_project" src/lib/types.ts` first, then adjust the wording to match — e.g. "names" becomes "is the id of").

Find:

```ts
  default_project: string;
```

Replace with:

```ts
  default_project_id: string;
```

- [ ] **Step 2: Update `api.ts`**

Edit `src/lib/api.ts`. Find:

```ts
export async function createTask(input: ParsedTaskInput): Promise<Task> {
  return invoke<Task>("create_task", {
    title: input.title,
    project: input.project,
    tags: input.tags.length > 0 ? input.tags : undefined,
    priority: input.priority,
    status: input.status,
    due: input.due,
    scheduled: input.scheduled,
    estimatedMinutes: input.estimatedMinutes,
  });
}
```

Replace with:

```ts
export async function createTask(input: ParsedTaskInput, projectId?: string): Promise<Task> {
  return invoke<Task>("create_task", {
    title: input.title,
    projectId,
    tags: input.tags.length > 0 ? input.tags : undefined,
    priority: input.priority,
    status: input.status,
    due: input.due,
    scheduled: input.scheduled,
    estimatedMinutes: input.estimatedMinutes,
  });
}
```

Find:

```ts
export async function createRecurringTask(
  input: ParsedTaskInput,
  frequency: RecurrenceFrequency,
  endDate: string | undefined,
  dueRule: DueRule | undefined,
): Promise<Task[]> {
  return invoke<Task[]>("create_recurring_task", {
    title: input.title,
    project: input.project,
    tags: input.tags.length > 0 ? input.tags : undefined,
    priority: input.priority,
    status: input.status,
    due: input.due,
    scheduled: input.scheduled,
    estimatedMinutes: input.estimatedMinutes,
    frequency,
    endDate,
    dueRule,
  });
}
```

Replace with:

```ts
export async function createRecurringTask(
  input: ParsedTaskInput,
  frequency: RecurrenceFrequency,
  endDate: string | undefined,
  dueRule: DueRule | undefined,
  projectId?: string,
): Promise<Task[]> {
  return invoke<Task[]>("create_recurring_task", {
    title: input.title,
    projectId,
    tags: input.tags.length > 0 ? input.tags : undefined,
    priority: input.priority,
    status: input.status,
    due: input.due,
    scheduled: input.scheduled,
    estimatedMinutes: input.estimatedMinutes,
    frequency,
    endDate,
    dueRule,
  });
}
```

Find:

```ts
export async function createProject(name: string, color?: string): Promise<Project> {
  return invoke<Project>("create_project", { name, color });
}
```

Replace with:

```ts
export async function createProject(name: string, color?: string, parentId?: string): Promise<Project> {
  return invoke<Project>("create_project", { name, color, parentId });
}
```

- [ ] **Step 3: Add `projectId` to `ParsedTaskInput`**

Edit `src/lib/naturalLanguage.ts`. Find the `ParsedTaskInput` interface's `project` field:

```ts
export interface ParsedTaskInput {
  title: string;
  tags: string[];
  project?: string;
```

Replace with:

```ts
export interface ParsedTaskInput {
  title: string;
  tags: string[];
  /** The display name of the project this task is filed under, resolved from typed/autocompleted text — see `projectId`. */
  project?: string;
  /**
   * The id of the project this task should actually be saved under, once
   * resolved (the parser itself never sets this — only `AddTaskModal`
   * does, after resolving `project` against the loaded project list — see
   * its `handleSubmit`). Mirrors how `dueRule` is also populated after
   * parsing, not by the parser itself.
   */
  projectId?: string;
```

- [ ] **Step 4: Update `projectColor.ts` to resolve by id**

Edit `src/lib/projectColor.ts`. Replace the whole file:

```ts
import type { InkMode } from "./colorPresets";
import { DEFAULT_PROJECT_COLOR, type Project } from "./types";

/** The project's stored color, or `DEFAULT_PROJECT_COLOR` for tasks with no project or an unrecognized one. */
export function resolveProjectColor(projectId: string | undefined, projects: Project[]): string {
  if (!projectId) return DEFAULT_PROJECT_COLOR;
  return projects.find((project) => project.id === projectId)?.color ?? DEFAULT_PROJECT_COLOR;
}

/**
 * The project's `board.card_lightness` override, or `globalLightness` for
 * tasks with no project, an unrecognized project, or a project that hasn't
 * overridden it. `?? globalLightness` (not `||`) so an override of exactly
 * `0` is respected rather than treated as unset.
 */
export function resolveCardLightness(
  projectId: string | undefined,
  projects: Project[],
  globalLightness: number,
): number {
  if (!projectId) return globalLightness;
  return projects.find((project) => project.id === projectId)?.board.card_lightness ?? globalLightness;
}

/** Same as `resolveCardLightness`, but for `board.bar_lightness` (week/calendar-view bars). */
export function resolveBarLightness(
  projectId: string | undefined,
  projects: Project[],
  globalLightness: number,
): number {
  if (!projectId) return globalLightness;
  return projects.find((project) => project.id === projectId)?.board.bar_lightness ?? globalLightness;
}

/** Same as `resolveCardLightness`, but for `board.ink_mode` (color-coded card/bar text). */
export function resolveInkMode(
  projectId: string | undefined,
  projects: Project[],
  globalInkMode: InkMode,
): InkMode {
  if (!projectId) return globalInkMode;
  return projects.find((project) => project.id === projectId)?.board.ink_mode ?? globalInkMode;
}
```

Open `src/lib/projectColor.test.ts` and update every test fixture: each `Project` object literal used as a fixture needs a `name`/`id` such that lookups switch from matching on `project.name === "SomeName"` to `project.id === "some-id"` — concretely, change every test's call from e.g. `resolveProjectColor("Homework", projects)` to `resolveProjectColor(homeworkProject.id, projects)` (using whatever variable name that test's fixture project is bound to), since the function now expects an id, not a name, as its first argument. The assertions themselves (expected colors/lightness/ink-mode values) don't change.

- [ ] **Step 5: Update `AddTaskModal.svelte`**

Edit `src/lib/components/AddTaskModal.svelte`. Find:

```ts
  let defaultProjectName = $derived(settingsState.current?.default_project ?? "General");
```

Replace with:

```ts
  let defaultProjectName = $derived(
    projectsState.items.find((p) => p.id === settingsState.current?.default_project_id)?.name ?? "General",
  );
```

Find `handleSubmit`:

```ts
  async function handleSubmit(event: Event) {
    event.preventDefault();
    if (!parsed.title) return;
    if (effectiveParsed.recurrence) {
      await onSubmit({
        ...effectiveParsed,
        project: preview.project,
        dueRule: resolveSeriesDueRule(effectiveParsed.due, effectiveParsed.dueRule, preview.scheduledDate),
      });
    } else {
      await onSubmit({
        ...effectiveParsed,
        project: preview.project,
        due: resolveNonRecurringDue(effectiveParsed.due, effectiveParsed.dueRule, preview.scheduledDate),
        dueRule: undefined,
      });
    }
  }
```

Replace with:

```ts
  async function handleSubmit(event: Event) {
    event.preventDefault();
    if (!parsed.title) return;
    // `matchedProject` already resolves `preview.project` (a name) against
    // the loaded project list (see its own `$derived` above) for the
    // preview pane — reused here as the actual submission's id. A typed
    // name with no match resolves to `undefined`, which the backend
    // resolves to the configured default project, same as leaving the
    // project blank entirely.
    const projectId = matchedProject?.id;
    if (effectiveParsed.recurrence) {
      await onSubmit({
        ...effectiveParsed,
        project: preview.project,
        projectId,
        dueRule: resolveSeriesDueRule(effectiveParsed.due, effectiveParsed.dueRule, preview.scheduledDate),
      });
    } else {
      await onSubmit({
        ...effectiveParsed,
        project: preview.project,
        projectId,
        due: resolveNonRecurringDue(effectiveParsed.due, effectiveParsed.dueRule, preview.scheduledDate),
        dueRule: undefined,
      });
    }
  }
```

- [ ] **Step 6: Update `KanbanBoard.svelte`**

Edit `src/lib/components/KanbanBoard.svelte`. Find:

```ts
  interface Props {
    title: string;
    tagline?: string;
    accentColor?: string;
    /** When set, only tasks whose `project` matches this name are shown. */
    projectFilter?: string;
  }
```

Replace with:

```ts
  interface Props {
    title: string;
    tagline?: string;
    accentColor?: string;
    /** When set, only tasks whose `project_id` matches this id are shown. */
    projectFilter?: string;
  }
```

Find:

```ts
  /**
   * The current project, looked up by name (case-insensitively, mirroring
   * `find_project_board` in the Rust command layer) when this board is scoped
   * to a project via `projectFilter`.
   */
  let project = $derived(
    projectFilter
      ? projectsState.items.find((p) => p.name.toLowerCase() === projectFilter.toLowerCase())
      : undefined,
  );
```

Replace with:

```ts
  /**
   * The current project, looked up by id, when this board is scoped to a
   * project via `projectFilter`.
   */
  let project = $derived(
    projectFilter ? projectsState.items.find((p) => p.id === projectFilter) : undefined,
  );
```

Find (inside `refresh()`):

```ts
      const visible = projectFilter
        ? allTasks.filter((task) => task.project === projectFilter)
        : allTasks;
```

Replace with:

```ts
      const visible = projectFilter
        ? allTasks.filter((task) => task.project_id === projectFilter)
        : allTasks;
```

Find (inside `replaceTask`):

```ts
    if (projectFilter && updated.project !== projectFilter) {
      removeTask(updated.id);
      return;
    }
```

Replace with:

```ts
    if (projectFilter && updated.project_id !== projectFilter) {
      removeTask(updated.id);
      return;
    }
```

Find `handleAddTask`:

```ts
  async function handleAddTask(parsed: ParsedTaskInput) {
    try {
      if (parsed.recurrence) {
        const tasks = await createRecurringTask(
          parsed,
          parsed.recurrence.frequency,
          parsed.recurrence.endDate,
          parsed.dueRule,
        );
        for (const task of tasks) replaceTask(task);
      } else {
        const task = await createTask(parsed);
        replaceTask(task);
      }
```

Replace with:

```ts
  async function handleAddTask(parsed: ParsedTaskInput) {
    try {
      if (parsed.recurrence) {
        const tasks = await createRecurringTask(
          parsed,
          parsed.recurrence.frequency,
          parsed.recurrence.endDate,
          parsed.dueRule,
          parsed.projectId,
        );
        for (const task of tasks) replaceTask(task);
      } else {
        const task = await createTask(parsed, parsed.projectId);
        replaceTask(task);
      }
```

- [ ] **Step 7: Update `TaskCard.svelte`**

Edit `src/lib/components/TaskCard.svelte`. Find:

```ts
  const projectColor = $derived(resolveProjectColor(task.project, projectsState.items));
```

Replace with:

```ts
  const projectColor = $derived(resolveProjectColor(task.project_id, projectsState.items));
  const projectName = $derived(projectsState.items.find((p) => p.id === task.project_id)?.name);
```

Find the next two `resolveCardLightness`/`resolveInkMode` calls passing `task.project,` (lines ~62 and ~80 per the earlier research) and change each occurrence of `task.project,`/`task.project)` to `task.project_id,`/`task.project_id)` — there are 2 remaining call sites beyond the one already shown above.

Find:

```ts
    draftProject = task.project ?? "";
```

Replace with:

```ts
    draftProject = projectsState.items.find((p) => p.id === task.project_id)?.name ?? "";
```

Find:

```ts
    const updated: Task = {
      ...task,
      title: draftTitle,
      project: emptyToUndefined(draftProject),
      tags: parseTags(draftTags),
```

Replace with:

```ts
    const updated: Task = {
      ...task,
      title: draftTitle,
      project_id: projectsState.items.find((p) => p.name.toLowerCase() === draftProject.trim().toLowerCase())?.id,
      tags: parseTags(draftTags),
```

Find (the project chip in the template):

```svelte
      {#if task.project && !isColorCoded}
        <span class="chip project" style="--chip-color: {projectColor}; --chip-text-color: {projectChipTextColor}">
          {task.project}
        </span>
      {/if}
```

Replace with:

```svelte
      {#if projectName && !isColorCoded}
        <span class="chip project" style="--chip-color: {projectColor}; --chip-text-color: {projectChipTextColor}">
          {projectName}
        </span>
      {/if}
```

- [ ] **Step 8: Update `WeekBarItem.svelte`**

Edit `src/lib/components/WeekBarItem.svelte`. Find:

```ts
  let barColor = $derived(resolveProjectColor(task.project, projectsState.items));
```

Replace with:

```ts
  let barColor = $derived(resolveProjectColor(task.project_id, projectsState.items));
  let projectName = $derived(projectsState.items.find((p) => p.id === task.project_id)?.name);
```

Find the `resolveBarLightness`/`resolveInkMode` calls passing `task.project,`/`task.project)` (lines ~58 and ~68) and change each to `task.project_id,`/`task.project_id)`.

Find:

```svelte
        {#if task.project}
          <span class="chip project" style="--chip-color: {barColor}">{task.project}</span>
        {/if}
```

Replace with:

```svelte
        {#if projectName}
          <span class="chip project" style="--chip-color: {barColor}">{projectName}</span>
        {/if}
```

- [ ] **Step 9: Update `deleteProject.ts` and `DeleteProjectSection.svelte`**

Edit `src/lib/deleteProject.ts`. Find:

```ts
/**
 * Returns `true` if `projectName` is the configured default project
 * (case-insensitive, ignoring surrounding whitespace) - mirrors
 * `ensure_not_default_project` in `src-tauri/src/commands.rs`. The default
 * project can never be deleted.
 */
export function isDefaultProject(projectName: string, defaultProjectName: string): boolean {
  return projectName.trim().toLowerCase() === defaultProjectName.trim().toLowerCase();
}

/**
 * Returns the tasks currently filed under `projectName` (matched
 * case-insensitively) - mirrors `tasks_for_project` in
 * `src-tauri/src/commands.rs`.
 */
export function tasksForProject(tasks: Task[], projectName: string): Task[] {
  const target = projectName.toLowerCase();
  return tasks.filter((task) => (task.project ?? "").toLowerCase() === target);
}
```

Replace with:

```ts
/**
 * Returns `true` if `projectId` is the configured default project - mirrors
 * `ensure_not_default_project` in `src-tauri/src/commands.rs`. The default
 * project can never be deleted.
 */
export function isDefaultProject(projectId: string, defaultProjectId: string): boolean {
  return projectId === defaultProjectId;
}

/**
 * Returns the tasks currently filed under `projectId` - mirrors
 * `tasks_for_projects` in `src-tauri/src/commands.rs`.
 */
export function tasksForProject(tasks: Task[], projectId: string): Task[] {
  return tasks.filter((task) => task.project_id === projectId);
}
```

Edit `src/lib/components/DeleteProjectSection.svelte`. Find:

```ts
  let isDefault = $derived(
    isDefaultProject(project.name, settingsState.current?.default_project ?? ""),
  );
```

Replace with:

```ts
  let isDefault = $derived(
    isDefaultProject(project.id, settingsState.current?.default_project_id ?? ""),
  );
```

Find:

```ts
      pendingTaskCount = tasksForProject(tasks, project.name).length;
```

Replace with:

```ts
      pendingTaskCount = tasksForProject(tasks, project.id).length;
```

- [ ] **Step 10: Update the project page route**

Edit `src/routes/projects/[id]/+page.svelte`. Find:

```svelte
    <KanbanBoard title={project.name} accentColor={project.color} projectFilter={project.name} />
```

Replace with:

```svelte
    <KanbanBoard title={project.name} accentColor={project.color} projectFilter={project.id} />
```

- [ ] **Step 11: Update `DefaultsSettings.svelte`'s default-project field to a project picker**

A free-text input can no longer work once the field is an id, not a name. Edit `src/lib/components/DefaultsSettings.svelte`. Find the imports at the top:

```ts
  import { hoursAndMinutesFromMinutes, minutesFromHoursAndMinutes, normalizeHoursMinutes } from "$lib/estimatedTime";
  import { DUE_RELATIVE_DATE_OPTIONS, SCHEDULED_RELATIVE_DATE_OPTIONS } from "$lib/relativeDates";
  import { persistSettings, settingsState } from "$lib/settings.svelte";
  import { formatTags, parseTags } from "$lib/taskFields";
  import type { TaskDefaults } from "$lib/types";
```

Replace with:

```ts
  import { hoursAndMinutesFromMinutes, minutesFromHoursAndMinutes, normalizeHoursMinutes } from "$lib/estimatedTime";
  import { projectsState } from "$lib/projects.svelte";
  import { DUE_RELATIVE_DATE_OPTIONS, SCHEDULED_RELATIVE_DATE_OPTIONS } from "$lib/relativeDates";
  import { persistSettings, settingsState } from "$lib/settings.svelte";
  import { formatTags, parseTags } from "$lib/taskFields";
  import type { TaskDefaults } from "$lib/types";
```

Find every remaining `default_project`/`draftDefaultProject`/`baselineDefaultProject` reference in the script section (the `$state`/`$derived` declarations, the `$effect` seeding block, `discardChanges`, and `save`) and rename `default_project` → `default_project_id` wherever it's a settings-field reference (e.g. `settingsState.current?.default_project` → `settingsState.current?.default_project_id`, and the `save()` function's `persistSettings({ ...settingsState.current, default_project: defaultProject, defaults })` → `persistSettings({ ...settingsState.current, default_project_id: defaultProject, defaults })`). The local variable/state names (`draftDefaultProject`, `baselineDefaultProject`) can stay as-is — they now just hold an id string instead of a name string, and renaming every local variable isn't required for correctness, only the actual `Settings` field key matters for the type to check out.

Find the `save()` function's empty-check:

```ts
    const defaultProject = draftDefaultProject.trim();
    if (!defaultProject) {
      errorMessage = "Default project name cannot be empty";
      return;
    }
```

Replace with:

```ts
    const defaultProject = draftDefaultProject;
    if (!defaultProject) {
      errorMessage = "Default project must be selected";
      return;
    }
```

Find the template's input field:

```svelte
    <div class="field">
      <label for="default-project">Default project name</label>
      <input
        id="default-project"
        type="text"
        placeholder="e.g. General"
        bind:value={draftDefaultProject}
      />
    </div>
```

Replace with:

```svelte
    <div class="field">
      <label for="default-project">Default project</label>
      <select id="default-project" bind:value={draftDefaultProject}>
        {#each projectsState.items as candidate (candidate.id)}
          <option value={candidate.id}>{candidate.name}</option>
        {/each}
      </select>
    </div>
```

- [ ] **Step 12: Run the full frontend verification suite**

Run: `npx vitest run && npx svelte-check && npm run build`
Expected: every Vitest test passes (after the `projectColor.test.ts` fixture updates from Step 4 — fix any remaining failures the same way), `svelte-check` reports 0 errors, and the build succeeds. If `svelte-check` reports any remaining reference to `.project`/`default_project` (not `_id`), it names the exact file/line — apply the same id-based substitution pattern used throughout this task.

- [ ] **Step 13: Commit**

```bash
git add src/lib/types.ts src/lib/api.ts src/lib/naturalLanguage.ts src/lib/projectColor.ts src/lib/projectColor.test.ts src/lib/deleteProject.ts src/lib/components/AddTaskModal.svelte src/lib/components/KanbanBoard.svelte src/lib/components/TaskCard.svelte src/lib/components/WeekBarItem.svelte src/lib/components/DeleteProjectSection.svelte src/lib/components/DefaultsSettings.svelte "src/routes/projects/[id]/+page.svelte"
git commit -m "feat: switch frontend project linkage from name to id"
```

---

## Task 8: `projectTree.ts` — pure tree helpers (frontend mirror of `project_tree.rs`)

**Files:**
- Create: `src/lib/projectTree.ts`
- Create: `src/lib/projectTree.test.ts`

**Interfaces:**
- Consumes: `Project` (from `types.ts`, Task 7's `parent_id` field).
- Produces: `childrenOf(projects: Project[], parentId: string | undefined): Project[]`, `ancestorsOf(projects: Project[], id: string): Project[]`, `selfAndAncestors(projects: Project[], id: string): Project[]`, `descendantsOf(projects: Project[], id: string): Project[]`, `wouldCreateCycle(projects: Project[], movingId: string, newParentId: string): boolean`.

This is a direct TypeScript port of `project_tree.rs` (Task 1) — same behavior, same function shapes, adapted to TS idioms (`undefined` instead of `Option`, array methods instead of iterator chains).

- [ ] **Step 1: Write the failing tests**

Create `src/lib/projectTree.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { ancestorsOf, childrenOf, descendantsOf, selfAndAncestors, wouldCreateCycle } from "./projectTree";
import type { Project } from "./types";

/**
 * Builds a small fixture tree:
 * ```text
 * root_a (top-level)
 * ├── child_a1
 * │   └── grandchild_a1a
 * └── child_a2
 * root_b (top-level, no children)
 * ```
 */
function fixtureTree(): Project[] {
  const base = (id: string, name: string, parentId: string | undefined, order: number): Project => ({
    id,
    name,
    color: "#111111",
    parent_id: parentId,
    order,
    created: "2026-06-11T10:00:00+00:00",
    board: { statuses: [] },
    defaults: { tags: [] },
  });

  return [
    base("root_a", "Root A", undefined, 1),
    base("root_b", "Root B", undefined, 2),
    base("child_a1", "Child A1", "root_a", 1),
    base("child_a2", "Child A2", "root_a", 2),
    base("grandchild_a1a", "Grandchild A1a", "child_a1", 1),
  ];
}

describe("childrenOf", () => {
  it("returns top-level projects when parentId is undefined", () => {
    const projects = fixtureTree();

    const children = childrenOf(projects, undefined);

    expect(children.map((p) => p.id)).toEqual(["root_a", "root_b"]);
  });

  it("returns direct children only", () => {
    const projects = fixtureTree();

    const children = childrenOf(projects, "root_a");

    expect(children.map((p) => p.id)).toEqual(["child_a1", "child_a2"]);
  });

  it("returns empty for a leaf", () => {
    const projects = fixtureTree();

    expect(childrenOf(projects, "child_a2")).toEqual([]);
  });
});

describe("ancestorsOf", () => {
  it("is empty for a top-level project", () => {
    const projects = fixtureTree();

    expect(ancestorsOf(projects, "root_a")).toEqual([]);
  });

  it("returns ancestors nearest-first", () => {
    const projects = fixtureTree();

    const ancestors = ancestorsOf(projects, "grandchild_a1a");

    expect(ancestors.map((p) => p.id)).toEqual(["child_a1", "root_a"]);
  });

  it("is empty for a missing id", () => {
    const projects = fixtureTree();

    expect(ancestorsOf(projects, "does-not-exist")).toEqual([]);
  });

  it("stops at a dangling parent_id", () => {
    const projects = fixtureTree();
    projects[2] = { ...projects[2], parent_id: "deleted-project" };

    const ancestors = ancestorsOf(projects, "grandchild_a1a");

    expect(ancestors.map((p) => p.id)).toEqual(["child_a1"]);
  });
});

describe("selfAndAncestors", () => {
  it("includes self first", () => {
    const projects = fixtureTree();

    const chain = selfAndAncestors(projects, "grandchild_a1a");

    expect(chain.map((p) => p.id)).toEqual(["grandchild_a1a", "child_a1", "root_a"]);
  });

  it("is empty for a missing id", () => {
    const projects = fixtureTree();

    expect(selfAndAncestors(projects, "does-not-exist")).toEqual([]);
  });
});

describe("descendantsOf", () => {
  it("returns all levels", () => {
    const projects = fixtureTree();

    const descendants = descendantsOf(projects, "root_a");

    expect(descendants.map((p) => p.id).sort()).toEqual(["child_a1", "child_a2", "grandchild_a1a"]);
  });

  it("is empty for a leaf", () => {
    const projects = fixtureTree();

    expect(descendantsOf(projects, "grandchild_a1a")).toEqual([]);
  });

  it("is empty for an unrelated root", () => {
    const projects = fixtureTree();

    expect(descendantsOf(projects, "root_b")).toEqual([]);
  });
});

describe("wouldCreateCycle", () => {
  it("is true when moving under self", () => {
    const projects = fixtureTree();

    expect(wouldCreateCycle(projects, "root_a", "root_a")).toBe(true);
  });

  it("is true when moving under own descendant", () => {
    const projects = fixtureTree();

    expect(wouldCreateCycle(projects, "root_a", "grandchild_a1a")).toBe(true);
  });

  it("is false when moving to an unrelated project", () => {
    const projects = fixtureTree();

    expect(wouldCreateCycle(projects, "child_a1", "root_b")).toBe(false);
  });
});
```

- [ ] **Step 2: Run the tests to confirm they fail**

Run: `npx vitest run projectTree`
Expected: FAIL — `Cannot find module './projectTree'`.

- [ ] **Step 3: Implement `projectTree.ts`**

Create `src/lib/projectTree.ts`:

```ts
import type { Project } from "./types";

/** Returns the direct children of `parentId` (or every top-level project, if `parentId` is `undefined`), in the order they appear in `projects`. */
export function childrenOf(projects: Project[], parentId: string | undefined): Project[] {
  return projects.filter((p) => p.parent_id === parentId);
}

/** Returns the ancestors of the project identified by `id`, nearest-first, ending at the root. Empty if `id` doesn't exist in `projects` or names a top-level project. */
export function ancestorsOf(projects: Project[], id: string): Project[] {
  const result: Project[] = [];
  let currentId = projects.find((p) => p.id === id)?.parent_id;

  while (currentId !== undefined) {
    const ancestor = projects.find((p) => p.id === currentId);
    if (!ancestor) break;
    result.push(ancestor);
    currentId = ancestor.parent_id;
  }

  return result;
}

/** Returns the project identified by `id` (if found) followed by its ancestors, nearest-first — the full settings-resolution chain for that project, ending at the root. Empty if `id` doesn't exist in `projects`. */
export function selfAndAncestors(projects: Project[], id: string): Project[] {
  const self = projects.find((p) => p.id === id);
  if (!self) return [];
  return [self, ...ancestorsOf(projects, id)];
}

/** Returns every transitive descendant of the project identified by `id` (children, grandchildren, ...). */
export function descendantsOf(projects: Project[], id: string): Project[] {
  const result: Project[] = [];
  const frontier: string[] = [id];

  while (frontier.length > 0) {
    const currentId = frontier.pop();
    if (currentId === undefined) break;
    for (const child of childrenOf(projects, currentId)) {
      result.push(child);
      frontier.push(child.id);
    }
  }

  return result;
}

/** Returns `true` if making `newParentId` the parent of `movingId` would create a cycle — i.e. `newParentId` is `movingId` itself, or is one of `movingId`'s current descendants. */
export function wouldCreateCycle(projects: Project[], movingId: string, newParentId: string): boolean {
  if (movingId === newParentId) return true;
  return descendantsOf(projects, movingId).some((p) => p.id === newParentId);
}
```

- [ ] **Step 4: Run the tests to confirm they pass**

Run: `npx vitest run projectTree`
Expected: all 16 tests pass.

- [ ] **Step 5: Run the full frontend verification suite**

Run: `npx vitest run && npx svelte-check`
Expected: no regressions, 0 type errors.

- [ ] **Step 6: Commit**

```bash
git add src/lib/projectTree.ts src/lib/projectTree.test.ts
git commit -m "feat: add projectTree pure tree helpers"
```

---

## Task 9: `shadesOf()` — parent-derived color shade suggestions

**Files:**
- Modify: `src/lib/colorPresets.ts`
- Modify: `src/lib/colorPresets.test.ts`

**Interfaces:**
- Consumes: `hexToOklch`, `neonCardColor`, `cssColorToHex`, `isHexColor` (all already in `colorPresets.ts`).
- Produces: `shadesOf(parentHex: string, count: number): string[]`.

- [ ] **Step 1: Write the failing tests**

Find `src/lib/colorPresets.test.ts` (check it exists — `grep -n "describe" src/lib/colorPresets.test.ts` to see its existing structure, then add a new `describe("shadesOf", ...)` block following the same style) and add:

```ts
describe("shadesOf", () => {
  it("returns the requested count", () => {
    const shades = shadesOf("#3b82f6", 5);

    expect(shades).toHaveLength(5);
  });

  it("returns valid hex colors", () => {
    const shades = shadesOf("#3b82f6", 5);

    for (const shade of shades) {
      expect(isHexColor(shade)).toBe(true);
    }
  });

  it("preserves the parent's hue across every shade", () => {
    const parentHue = hexToOklch("#3b82f6").h;
    const shades = shadesOf("#3b82f6", 5);

    for (const shade of shades) {
      expect(hexToOklch(shade).h).toBeCloseTo(parentHue, 0);
    }
  });

  it("varies lightness monotonically across the shades", () => {
    const shades = shadesOf("#3b82f6", 5);
    const lightnesses = shades.map((shade) => hexToOklch(shade).l);

    for (let i = 1; i < lightnesses.length; i++) {
      expect(lightnesses[i]).toBeGreaterThan(lightnesses[i - 1]);
    }
  });

  it("returns a single shade without dividing by zero when count is 1", () => {
    const shades = shadesOf("#3b82f6", 1);

    expect(shades).toHaveLength(1);
    expect(isHexColor(shades[0])).toBe(true);
  });
});
```

Add `shadesOf` to the existing `import { ... } from "./colorPresets"` line at the top of the test file (alongside whatever's already imported — likely `hexToOklch`, `isHexColor`, etc., per the functions referenced above).

- [ ] **Step 2: Run the tests to confirm they fail**

Run: `npx vitest run colorPresets -t shadesOf`
Expected: FAIL — `shadesOf is not a function` / not exported.

- [ ] **Step 3: Implement `shadesOf`**

Edit `src/lib/colorPresets.ts`. Add after `neonCardColor`'s existing definition (and its `NEON_CARD_LIGHTNESS`/`NEON_CARD_CHROMA_BOOST` constants):

```ts
/** Lightness range `shadesOf` spreads its suggestions across — stays clear of near-black/near-white extremes where hue becomes visually indistinct. */
const SHADE_MIN_LIGHTNESS = 0.3;
const SHADE_MAX_LIGHTNESS = 0.7;

/**
 * Returns `count` color suggestions derived from `parentHex`'s hue and
 * chroma, varying only lightness, spread evenly across
 * `[SHADE_MIN_LIGHTNESS, SHADE_MAX_LIGHTNESS]` — the model for a
 * subproject's color picker defaults, so its suggested colors read as
 * "shades of the parent" rather than unrelated hues. Built on
 * `neonCardColor` (same hue/chroma, different fixed lightness) — the
 * existing precedent for this exact kind of derivation — converted to hex
 * since `Project.color` is hex-only.
 */
export function shadesOf(parentHex: string, count: number): string[] {
  const step = count > 1 ? (SHADE_MAX_LIGHTNESS - SHADE_MIN_LIGHTNESS) / (count - 1) : 0;
  return Array.from({ length: count }, (_, index) =>
    cssColorToHex(neonCardColor(parentHex, SHADE_MIN_LIGHTNESS + step * index)),
  );
}
```

- [ ] **Step 4: Run the tests to confirm they pass**

Run: `npx vitest run colorPresets -t shadesOf`
Expected: all 5 tests pass.

- [ ] **Step 5: Run the full frontend verification suite**

Run: `npx vitest run && npx svelte-check`
Expected: no regressions.

- [ ] **Step 6: Commit**

```bash
git add src/lib/colorPresets.ts src/lib/colorPresets.test.ts
git commit -m "feat: add shadesOf parent-derived color shade suggestions"
```

---

## Task 10: `ColorPicker` shade suggestions + `NewProjectModal` parent-scoping

**Files:**
- Modify: `src/lib/components/ColorPicker.svelte`
- Modify: `src/lib/components/NewProjectModal.svelte`

**Interfaces:**
- Consumes: `shadesOf` (Task 9).
- Produces: `ColorPicker` gains optional `parentColor?: string`/`parentName?: string` props. `NewProjectModal` gains an optional `parentProject?: Project` prop.

No backend/store changes in this task — purely UI. Since these are presentational Svelte components without dedicated unit tests in this codebase's existing convention (no `.svelte.test.ts` exists for either today), this task is verified via `svelte-check`/`build` plus your manual visual check, same disclosed-limitation pattern as prior recurrence-builder UI work in this codebase.

- [ ] **Step 1: Add shade suggestions to `ColorPicker.svelte`**

Replace the entire file `src/lib/components/ColorPicker.svelte` with:

```svelte
<script lang="ts">
  import { cssColorToHex, isHexColor, PRESET_COLOR_NAMES, PRESET_COLORS, shadesOf } from "$lib/colorPresets";

  interface Props {
    value: string;
    label: string;
    /** When set (creating/editing a subproject), shows shade suggestions derived from this color ahead of the fixed presets. */
    parentColor?: string;
    /** The parent project's name, used to label the shade suggestions (e.g. "Shades of Homework"). Ignored if `parentColor` isn't set. */
    parentName?: string;
  }

  let { value = $bindable(), label, parentColor, parentName }: Props = $props();

  let isValid = $derived(value.trim() !== "" && CSS.supports("color", value));
  let shadeSuggestions = $derived(parentColor ? shadesOf(parentColor, 5) : []);

  // Migrates legacy non-hex (e.g. oklch) color values to hex on display, so
  // saving the form persists the hex form without further action.
  $effect(() => {
    if (value.trim() === "" || isHexColor(value)) return;
    const hex = cssColorToHex(value);
    if (hex !== value) value = hex;
  });
</script>

{#snippet swatchButton(color: string, swatchLabel: string)}
  <button
    type="button"
    class="color-swatch"
    class:selected={value === color}
    style="background: {color}"
    aria-pressed={value === color}
    aria-label={swatchLabel}
    onclick={() => (value = color)}
  ></button>
{/snippet}

<div class="color-picker">
  {#if shadeSuggestions.length > 0}
    <div class="color-group">
      <span class="group-label">Shades of {parentName ?? "parent"}</span>
      <div class="color-grid" role="group" aria-label={`Shades of ${parentName ?? "parent"}`}>
        {#each shadeSuggestions as shade, index (shade)}
          {@render swatchButton(shade, `Shade ${index + 1} (${shade})`)}
        {/each}
      </div>
    </div>
    <span class="group-label">Presets</span>
  {/if}
  <div class="color-grid" role="group" aria-label={label}>
    {#each PRESET_COLORS as preset, index (preset)}
      {@render swatchButton(preset, `${PRESET_COLOR_NAMES[index]} (${preset})`)}
    {/each}
  </div>
  <div class="custom-color">
    <span class="preview-swatch" style="background: {value}" aria-hidden="true"></span>
    <input
      type="text"
      class="color-input"
      class:invalid={!isValid}
      bind:value
      aria-label={`${label} (custom value)`}
      aria-invalid={!isValid}
    />
  </div>
</div>

<style>
  .color-picker {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-sm);
  }

  .color-group {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
  }

  .group-label {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--color-ink-muted);
  }

  .color-grid {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-xs);
  }

  .color-swatch {
    width: 1.5rem;
    height: 1.5rem;
    border-radius: var(--radius-pill);
    border: 1px solid var(--color-border);
    cursor: pointer;
    box-shadow: var(--shadow-sm);
    transition:
      box-shadow var(--duration-fast) var(--ease-out-expo),
      transform var(--duration-fast) var(--ease-out-expo);
  }

  .color-swatch:hover {
    transform: translateY(-1px);
  }

  .color-swatch.selected {
    box-shadow:
      0 0 0 2px var(--color-surface-raised),
      0 0 0 4px var(--color-accent);
  }

  .color-swatch:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  .custom-color {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
  }

  .preview-swatch {
    width: 1.25rem;
    height: 1.25rem;
    border-radius: var(--radius-pill);
    border: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .color-input {
    width: 9rem;
    padding: var(--space-2xs) var(--space-sm);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-ink);
    font-size: var(--text-xs);
    box-shadow: var(--shadow-sm);
    transition:
      border-color var(--duration-fast) var(--ease-out-expo),
      box-shadow var(--duration-fast) var(--ease-out-expo);
  }

  .color-input:focus-visible {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-soft);
    outline: none;
  }

  .color-input.invalid {
    border-color: var(--color-danger);
  }

  .color-input.invalid:focus-visible {
    box-shadow: 0 0 0 3px var(--color-danger-soft);
  }
</style>
```

- [ ] **Step 2: Run typecheck**

Run: `npx svelte-check`
Expected: 0 errors.

- [ ] **Step 3: Add parent-scoping to `NewProjectModal.svelte`**

Find:

```ts
  import { createProject } from "$lib/api";
  import { DEFAULT_PROJECT_COLOR, type Project } from "$lib/types";
  import ColorPicker from "$lib/components/ColorPicker.svelte";

  interface Props {
    open: boolean;
    onClose: () => void;
    onCreated: (project: Project) => void;
  }

  let { open, onClose, onCreated }: Props = $props();
```

Replace with:

```ts
  import { createProject } from "$lib/api";
  import { shadesOf } from "$lib/colorPresets";
  import { DEFAULT_PROJECT_COLOR, type Project } from "$lib/types";
  import ColorPicker from "$lib/components/ColorPicker.svelte";

  interface Props {
    open: boolean;
    onClose: () => void;
    onCreated: (project: Project) => void;
    /** When set, the created project becomes a subproject of this one. */
    parentProject?: Project;
  }

  let { open, onClose, onCreated, parentProject }: Props = $props();
```

Find the `$effect` that resets the form on open:

```ts
  $effect(() => {
    if (!dialogEl) return;
    if (open) {
      if (!dialogEl.open) {
        name = "";
        color = DEFAULT_PROJECT_COLOR;
        errorMessage = "";
        dialogEl.showModal();
        inputEl?.focus();
      }
    } else if (dialogEl.open) {
      dialogEl.close();
    }
  });
```

Replace with:

```ts
  $effect(() => {
    if (!dialogEl) return;
    if (open) {
      if (!dialogEl.open) {
        name = "";
        color = parentProject ? shadesOf(parentProject.color, 5)[0] : DEFAULT_PROJECT_COLOR;
        errorMessage = "";
        dialogEl.showModal();
        inputEl?.focus();
      }
    } else if (dialogEl.open) {
      dialogEl.close();
    }
  });
```

Find `handleSubmit`:

```ts
    try {
      const project = await createProject(trimmed, color);
      errorMessage = "";
```

Replace with:

```ts
    try {
      const project = await createProject(trimmed, color, parentProject?.id);
      errorMessage = "";
```

Find the dialog heading:

```svelte
      <h2 id="new-project-heading">New project</h2>
```

Replace with:

```svelte
      <h2 id="new-project-heading">{parentProject ? `New subproject of ${parentProject.name}` : "New project"}</h2>
```

Find the `ColorPicker` usage:

```svelte
      <ColorPicker bind:value={color} label="Project color" />
```

Replace with:

```svelte
      <ColorPicker
        bind:value={color}
        label="Project color"
        parentColor={parentProject?.color}
        parentName={parentProject?.name}
      />
```

- [ ] **Step 4: Run the full frontend verification suite**

Run: `npx vitest run && npx svelte-check && npm run build`
Expected: no regressions, 0 type errors, build succeeds. `Sidebar.svelte`'s existing `<NewProjectModal open={...} onClose={...} onCreated={...} />` call site (no `parentProject` passed) still compiles unchanged, since the new prop is optional.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/ColorPicker.svelte src/lib/components/NewProjectModal.svelte
git commit -m "feat: parent-derived color shade suggestions for subproject creation"
```

---

## Task 11: Sidebar tree UI + expand-state persistence

**Files:**
- Create: `src/lib/projectTree.svelte.ts`
- Create: `src/lib/projectTree.svelte.test.ts`
- Create: `src/lib/components/ProjectTreeNode.svelte`
- Modify: `src/lib/components/Sidebar.svelte`
- Modify: `src/routes/+layout.svelte`

**Interfaces:**
- Consumes: `childrenOf` (Task 8), `Project.parent_id` (Task 7).
- Produces: `projectTreeState: { expanded: Record<string, boolean> }`, `isExpanded(projectId)`, `setExpanded(projectId, expanded)`, `toggleExpanded(projectId)`, `expandIfUnset(projectId)`, `initProjectTree()`.

- [ ] **Step 1: Write the failing tests for the expand-state store**

Create `src/lib/projectTree.svelte.test.ts`:

```ts
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  expandIfUnset,
  initProjectTree,
  isExpanded,
  projectTreeState,
  setExpanded,
  toggleExpanded,
} from "./projectTree.svelte";

const STORAGE_KEY = "taskmancer:project-tree-expanded";

beforeEach(() => {
  localStorage.clear();
  projectTreeState.expanded = {};
});

describe("isExpanded", () => {
  it("defaults to false for a project never toggled", () => {
    expect(isExpanded("unset-project")).toBe(false);
  });
});

describe("setExpanded", () => {
  it("updates state and persists to localStorage", () => {
    setExpanded("project-1", true);

    expect(isExpanded("project-1")).toBe(true);
    expect(JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "{}")).toEqual({ "project-1": true });
  });

  it("does not affect other projects' state", () => {
    setExpanded("project-1", true);
    setExpanded("project-2", false);

    expect(isExpanded("project-1")).toBe(true);
    expect(isExpanded("project-2")).toBe(false);
  });
});

describe("toggleExpanded", () => {
  it("flips from the default (false) to true", () => {
    toggleExpanded("project-1");

    expect(isExpanded("project-1")).toBe(true);
  });

  it("flips back to false on a second call", () => {
    toggleExpanded("project-1");
    toggleExpanded("project-1");

    expect(isExpanded("project-1")).toBe(false);
  });
});

describe("expandIfUnset", () => {
  it("expands a project with no recorded preference", () => {
    expandIfUnset("project-1");

    expect(isExpanded("project-1")).toBe(true);
  });

  it("does not override an explicit prior collapse", () => {
    setExpanded("project-1", false);
    expandIfUnset("project-1");

    expect(isExpanded("project-1")).toBe(false);
  });

  it("does not override an explicit prior expand", () => {
    setExpanded("project-1", true);
    expandIfUnset("project-1");

    expect(isExpanded("project-1")).toBe(true);
  });
});

describe("initProjectTree", () => {
  it("restores previously persisted state", () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ "project-1": true }));

    initProjectTree();

    expect(isExpanded("project-1")).toBe(true);
  });

  it("defaults to no expanded projects when nothing is stored", () => {
    initProjectTree();

    expect(isExpanded("project-1")).toBe(false);
  });

  it("falls back to defaults when localStorage throws", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });

    expect(() => initProjectTree()).not.toThrow();

    vi.restoreAllMocks();
  });
});
```

- [ ] **Step 2: Run the tests to confirm they fail**

Run: `npx vitest run projectTree.svelte`
Expected: FAIL — `Cannot find module './projectTree.svelte'`.

- [ ] **Step 3: Implement `projectTree.svelte.ts`**

Create `src/lib/projectTree.svelte.ts`, following the exact pattern established by `sidebar.svelte.ts`:

```ts
const STORAGE_KEY = "taskmancer:project-tree-expanded";

/** Boxed in an object because Svelte 5 forbids exporting a reassigned `$state` binding directly from a module — only its properties may be mutated. */
export const projectTreeState = $state<{ expanded: Record<string, boolean> }>({ expanded: {} });

function persist(): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(projectTreeState.expanded));
  } catch {
    // Persistence is best-effort; the choice still applies for this session.
  }
}

/** Returns whether `projectId`'s subproject list should render expanded. Defaults to collapsed for a project never explicitly toggled. */
export function isExpanded(projectId: string): boolean {
  return projectTreeState.expanded[projectId] === true;
}

/** Sets whether `projectId`'s subproject list is expanded, persisting the choice. */
export function setExpanded(projectId: string, expanded: boolean): void {
  projectTreeState.expanded = { ...projectTreeState.expanded, [projectId]: expanded };
  persist();
}

export function toggleExpanded(projectId: string): void {
  setExpanded(projectId, !isExpanded(projectId));
}

/**
 * Marks `projectId` expanded only if it has no explicit recorded
 * preference yet — called the moment a project gains its first subproject,
 * so a newly created subproject is immediately visible without a manual
 * toggle, while never overriding a preference the user already set
 * (including an explicit collapse).
 */
export function expandIfUnset(projectId: string): void {
  if (projectId in projectTreeState.expanded) return;
  setExpanded(projectId, true);
}

/** Restores previously persisted expand state, falling back to all-collapsed. */
export function initProjectTree(): void {
  let stored: string | null = null;
  try {
    stored = localStorage.getItem(STORAGE_KEY);
  } catch {
    return;
  }
  if (!stored) return;

  try {
    const parsed = JSON.parse(stored);
    if (parsed && typeof parsed === "object") {
      projectTreeState.expanded = parsed;
    }
  } catch {
    // Fall back to the default (all collapsed).
  }
}
```

- [ ] **Step 4: Run the tests to confirm they pass**

Run: `npx vitest run projectTree.svelte`
Expected: all 12 tests pass.

- [ ] **Step 5: Wire `initProjectTree` into the root layout**

Edit `src/routes/+layout.svelte`. Find:

```ts
  import { initGeneral } from "$lib/generalSettings.svelte";
  import { refreshProjects } from "$lib/projects.svelte";
  import { refreshSettings } from "$lib/settings.svelte";
  import { initSidebar } from "$lib/sidebar.svelte";
```

Replace with:

```ts
  import { initGeneral } from "$lib/generalSettings.svelte";
  import { initProjectTree } from "$lib/projectTree.svelte";
  import { refreshProjects } from "$lib/projects.svelte";
  import { refreshSettings } from "$lib/settings.svelte";
  import { initSidebar } from "$lib/sidebar.svelte";
```

Find:

```ts
  initTheme();
  initSidebar();
  initDisplay();
  initGeneral();
```

Replace with:

```ts
  initTheme();
  initSidebar();
  initDisplay();
  initGeneral();
  initProjectTree();
```

- [ ] **Step 6: Create `ProjectTreeNode.svelte`**

Create `src/lib/components/ProjectTreeNode.svelte`:

```svelte
<script lang="ts">
  import { page } from "$app/state";
  import { projectsState } from "$lib/projects.svelte";
  import { childrenOf } from "$lib/projectTree";
  import { isExpanded, toggleExpanded } from "$lib/projectTree.svelte";
  import { sidebarState } from "$lib/sidebar.svelte";
  import type { Project } from "$lib/types";
  import ProjectTreeNode from "./ProjectTreeNode.svelte";

  interface Props {
    project: Project;
    depth: number;
    onCreateSubproject: (parent: Project) => void;
  }

  let { project, depth, onCreateSubproject }: Props = $props();

  let children = $derived(childrenOf(projectsState.items, project.id));
  let hasChildren = $derived(children.length > 0);
  let expanded = $derived(isExpanded(project.id));
</script>

<li>
  <div class="project-row" style="--depth: {depth}">
    {#if hasChildren}
      <button
        type="button"
        class="expand-toggle"
        class:expanded
        onclick={() => toggleExpanded(project.id)}
        aria-expanded={expanded}
        aria-label={expanded ? `Collapse ${project.name}` : `Expand ${project.name}`}
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <polyline points="9 18 15 12 9 6" />
        </svg>
      </button>
    {:else}
      <span class="expand-spacer" aria-hidden="true"></span>
    {/if}
    <a
      href="/projects/{project.id}"
      class="nav-link"
      class:active={page.url.pathname === `/projects/${project.id}`}
      title={sidebarState.collapsed ? project.name : undefined}
    >
      <span class="color-dot" style="background: {project.color}" aria-hidden="true"></span>
      {#if !sidebarState.collapsed}<span class="project-name">{project.name}</span>{/if}
    </a>
    {#if !sidebarState.collapsed}
      <button
        type="button"
        class="add-subproject-button"
        onclick={() => onCreateSubproject(project)}
        aria-label={`New subproject of ${project.name}`}
        title={`New subproject of ${project.name}`}
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <line x1="12" y1="5" x2="12" y2="19" />
          <line x1="5" y1="12" x2="19" y2="12" />
        </svg>
      </button>
    {/if}
  </div>
  {#if hasChildren && expanded}
    <ul class="subproject-list">
      {#each children as child (child.id)}
        <ProjectTreeNode project={child} depth={depth + 1} {onCreateSubproject} />
      {/each}
    </ul>
  {/if}
</li>

<style>
  .project-row {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    padding-left: calc(var(--depth) * 0.9rem);
  }

  .project-row:hover .add-subproject-button {
    opacity: 1;
  }

  .expand-toggle,
  .expand-spacer {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1rem;
    height: 1rem;
    flex-shrink: 0;
  }

  .expand-toggle {
    border: none;
    background: transparent;
    color: var(--color-ink-muted);
    cursor: pointer;
    padding: 0;
  }

  .expand-toggle svg {
    transition: transform var(--duration-fast) var(--ease-out-expo);
  }

  .expand-toggle.expanded svg {
    transform: rotate(90deg);
  }

  .nav-link {
    flex: 1;
    min-width: 0;
  }

  .add-subproject-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.25rem;
    height: 1.25rem;
    flex-shrink: 0;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-ink-muted);
    cursor: pointer;
    opacity: 0;
    transition: opacity var(--duration-fast) var(--ease-out-expo);
  }

  .add-subproject-button:hover,
  .add-subproject-button:focus-visible {
    opacity: 1;
    color: var(--color-accent);
  }

  .subproject-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
    list-style: none;
    margin: 0;
    padding: 0;
  }
</style>
```

(`.color-dot`/`.project-name`/`.nav-link`'s base appearance classes are defined globally in `Sidebar.svelte`'s `<style>` block, not duplicated here — Svelte scopes a component's own `<style>` to elements it renders directly, so `Sidebar.svelte`'s existing `.nav-link`/`.color-dot`/`.project-name` rules need to move or be shared. See Step 7.)

- [ ] **Step 7: Rewire `Sidebar.svelte` to render the tree**

Edit `src/lib/components/Sidebar.svelte`. Find:

```ts
  import { page } from "$app/state";
  import NewProjectModal from "./NewProjectModal.svelte";
  import { projectsState } from "$lib/projects.svelte";
  import { sidebarState, toggleSidebar } from "$lib/sidebar.svelte";
  import type { Project } from "$lib/types";

  let newProjectOpen = $state(false);

  function handleProjectCreated(project: Project) {
    projectsState.items = [...projectsState.items, project].sort((a, b) => a.order - b.order);
  }
```

Replace with:

```ts
  import { page } from "$app/state";
  import NewProjectModal from "./NewProjectModal.svelte";
  import ProjectTreeNode from "./ProjectTreeNode.svelte";
  import { projectsState } from "$lib/projects.svelte";
  import { childrenOf } from "$lib/projectTree";
  import { expandIfUnset } from "$lib/projectTree.svelte";
  import { sidebarState, toggleSidebar } from "$lib/sidebar.svelte";
  import type { Project } from "$lib/types";

  let newProjectOpen = $state(false);
  let subprojectParent: Project | undefined = $state(undefined);

  let topLevelProjects = $derived(childrenOf(projectsState.items, undefined));

  function openNewProjectModal() {
    subprojectParent = undefined;
    newProjectOpen = true;
  }

  function openNewSubprojectModal(parent: Project) {
    subprojectParent = parent;
    newProjectOpen = true;
  }

  function handleProjectCreated(project: Project) {
    projectsState.items = [...projectsState.items, project].sort((a, b) => a.order - b.order);
    if (project.parent_id) expandIfUnset(project.parent_id);
  }
```

Find:

```svelte
    <ul class="project-list">
      {#each projectsState.items as project (project.id)}
        <li>
          <a
            href="/projects/{project.id}"
            class="nav-link"
            class:active={page.url.pathname === `/projects/${project.id}`}
            title={sidebarState.collapsed ? project.name : undefined}
          >
            <span class="color-dot" style="background: {project.color}" aria-hidden="true"></span>
            {#if !sidebarState.collapsed}<span class="project-name">{project.name}</span>{/if}
          </a>
        </li>
      {/each}
    </ul>
    <button
      type="button"
      class="new-project-button"
      onclick={() => (newProjectOpen = true)}
      title={sidebarState.collapsed ? "New project" : undefined}
    >
```

Replace with:

```svelte
    <ul class="project-list">
      {#each topLevelProjects as project (project.id)}
        <ProjectTreeNode {project} depth={0} onCreateSubproject={openNewSubprojectModal} />
      {/each}
    </ul>
    <button
      type="button"
      class="new-project-button"
      onclick={openNewProjectModal}
      title={sidebarState.collapsed ? "New project" : undefined}
    >
```

Find:

```svelte
<NewProjectModal
  open={newProjectOpen}
  onClose={() => (newProjectOpen = false)}
  onCreated={handleProjectCreated}
/>
```

Replace with:

```svelte
<NewProjectModal
  open={newProjectOpen}
  onClose={() => (newProjectOpen = false)}
  onCreated={handleProjectCreated}
  parentProject={subprojectParent}
/>
```

`page` is no longer referenced directly in `Sidebar.svelte`'s own markup for project rows (that logic moved into `ProjectTreeNode.svelte`), but it's still used by the "All Tasks"/"Settings" nav links elsewhere in the same file — leave the `import { page } from "$app/state";` line as-is. The `.color-dot`/`.project-name` CSS rules in `Sidebar.svelte`'s `<style>` block become unused (their elements moved to `ProjectTreeNode.svelte`, which has its own copies) — delete the now-dead `.color-dot` and `.project-name` rule blocks from `Sidebar.svelte`'s `<style>` section (search for them — they're small, a handful of lines each, right after `.project-list`).

- [ ] **Step 8: Run the full frontend verification suite**

Run: `npx vitest run && npx svelte-check && npm run build`
Expected: no regressions, 0 type errors, build succeeds.

- [ ] **Step 9: Commit**

```bash
git add src/lib/projectTree.svelte.ts src/lib/projectTree.svelte.test.ts src/lib/components/ProjectTreeNode.svelte src/lib/components/Sidebar.svelte src/routes/+layout.svelte
git commit -m "feat: nested sidebar tree with persisted expand state"
```

---

## Task 12: Drag-and-drop re-parenting in the sidebar

**Files:**
- Modify: `src/lib/projectTree.ts`
- Modify: `src/lib/projectTree.test.ts`
- Modify: `src/lib/components/ProjectTreeNode.svelte`
- Modify: `src/lib/components/Sidebar.svelte`

**Interfaces:**
- Consumes: `wouldCreateCycle` (Task 8), `updateProject` (existing), `refreshProjects` (existing).
- Produces: `computeZoneOrderUpdates(allProjects, zoneParentId, zoneItems, orderStep?) -> { updates: ProjectOrderUpdate[]; rejected: boolean }`.

**Documented scope boundary:** dragging only works between zones that are already rendered — i.e. an already-expanded subproject list, or the top-level list. A currently-childless project renders no drop zone at all (there's nothing to expand), and a collapsed project's children aren't rendered either. Making a childless project a parent for the first time, or targeting a collapsed one, goes through the "Move to parent" picker (Task 13) instead — the spec's explicit choice to support both methods means each can cover the other's gap rather than drag-and-drop needing to handle every case alone.

- [ ] **Step 1: Write the failing tests for `computeZoneOrderUpdates`**

Add to `src/lib/projectTree.test.ts`:

```ts
import { computeZoneOrderUpdates } from "./projectTree";

// ... (alongside the existing describe blocks)

describe("computeZoneOrderUpdates", () => {
  function project(id: string, parentId: string | undefined, order: number): Project {
    return {
      id,
      name: id,
      color: "#111111",
      parent_id: parentId,
      order,
      created: "2026-06-11T10:00:00+00:00",
      board: { statuses: [] },
      defaults: { tags: [] },
    };
  }

  it("returns no updates when a zone is already correctly ordered", () => {
    const a = project("a", "parent", 1000);
    const b = project("b", "parent", 2000);
    const allProjects = [a, b];

    const { updates, rejected } = computeZoneOrderUpdates(allProjects, "parent", [a, b]);

    expect(rejected).toBe(false);
    expect(updates).toEqual([]);
  });

  it("returns an update for a reordered item within the same zone", () => {
    const a = project("a", "parent", 1000);
    const b = project("b", "parent", 2000);
    const allProjects = [a, b];

    const { updates, rejected } = computeZoneOrderUpdates(allProjects, "parent", [b, a]);

    expect(rejected).toBe(false);
    expect(updates).toEqual([
      { id: "b", parent_id: "parent", order: 1000 },
      { id: "a", parent_id: "parent", order: 2000 },
    ]);
  });

  it("returns a reparent update for an item moved in from a different zone", () => {
    const moved = project("moved", "old-parent", 1000);
    const sibling = project("sibling", "new-parent", 1000);
    const allProjects = [moved, sibling];

    const { updates, rejected } = computeZoneOrderUpdates(allProjects, "new-parent", [sibling, moved]);

    expect(rejected).toBe(false);
    expect(updates).toContainEqual({ id: "moved", parent_id: "new-parent", order: 2000 });
  });

  it("rejects moving a project into one of its own descendants", () => {
    const parent = project("parent", undefined, 1000);
    const child = project("child", "parent", 1000);
    const allProjects = [parent, child];

    const { rejected } = computeZoneOrderUpdates(allProjects, "child", [parent]);

    expect(rejected).toBe(true);
  });

  it("treats undefined zoneParentId (top-level) as never a cycle risk", () => {
    const a = project("a", undefined, 1000);
    const allProjects = [a];

    const { rejected } = computeZoneOrderUpdates(allProjects, undefined, [a]);

    expect(rejected).toBe(false);
  });
});
```

- [ ] **Step 2: Run the tests to confirm they fail**

Run: `npx vitest run projectTree -t computeZoneOrderUpdates`
Expected: FAIL — `computeZoneOrderUpdates is not a function`.

- [ ] **Step 3: Implement `computeZoneOrderUpdates`**

Edit `src/lib/projectTree.ts`. Add at the end:

```ts
export interface ProjectOrderUpdate {
  id: string;
  parent_id: string | undefined;
  order: number;
}

/**
 * Computes the project updates needed to persist `zoneItems` (a project
 * tree zone's current children, in display order, after a drag-and-drop
 * reorder/reparent) as the children of `zoneParentId` (`undefined` for the
 * top-level zone). Returns one `ProjectOrderUpdate` per item whose
 * `parent_id` and/or `order` actually needs to change — an item already
 * correctly parented and numbered isn't included, so callers only persist
 * what actually changed.
 *
 * `allProjects` (the full, current tree) is used for a cycle check: if any
 * item's `parent_id` would need to change to `zoneParentId`, but
 * `zoneParentId` is that item's own descendant (see `wouldCreateCycle`),
 * the whole result is `rejected: true` and `updates` is empty — callers
 * should treat a rejected drop as entirely invalid and re-sync from the
 * server rather than partially applying it.
 */
export function computeZoneOrderUpdates(
  allProjects: Project[],
  zoneParentId: string | undefined,
  zoneItems: Project[],
  orderStep = 1000,
): { updates: ProjectOrderUpdate[]; rejected: boolean } {
  const updates: ProjectOrderUpdate[] = [];

  for (const [index, item] of zoneItems.entries()) {
    const needsReparent = item.parent_id !== zoneParentId;
    if (needsReparent && zoneParentId !== undefined && wouldCreateCycle(allProjects, item.id, zoneParentId)) {
      return { updates: [], rejected: true };
    }

    const newOrder = (index + 1) * orderStep;
    if (needsReparent || item.order !== newOrder) {
      updates.push({ id: item.id, parent_id: zoneParentId, order: newOrder });
    }
  }

  return { updates, rejected: false };
}
```

- [ ] **Step 4: Run the tests to confirm they pass**

Run: `npx vitest run projectTree -t computeZoneOrderUpdates`
Expected: all 5 tests pass.

- [ ] **Step 5: Wire drag-and-drop into `ProjectTreeNode.svelte`**

Find:

```ts
  import { page } from "$app/state";
  import { projectsState } from "$lib/projects.svelte";
  import { childrenOf } from "$lib/projectTree";
  import { isExpanded, toggleExpanded } from "$lib/projectTree.svelte";
  import { sidebarState } from "$lib/sidebar.svelte";
  import type { Project } from "$lib/types";
  import ProjectTreeNode from "./ProjectTreeNode.svelte";

  interface Props {
    project: Project;
    depth: number;
    onCreateSubproject: (parent: Project) => void;
  }

  let { project, depth, onCreateSubproject }: Props = $props();

  let children = $derived(childrenOf(projectsState.items, project.id));
  let hasChildren = $derived(children.length > 0);
  let expanded = $derived(isExpanded(project.id));
</script>
```

Replace with:

```ts
  import { dndzone, type DndEvent } from "svelte-dnd-action";
  import { page } from "$app/state";
  import { updateProject } from "$lib/api";
  import { projectsState, refreshProjects } from "$lib/projects.svelte";
  import { childrenOf, computeZoneOrderUpdates } from "$lib/projectTree";
  import { isExpanded, toggleExpanded } from "$lib/projectTree.svelte";
  import { sidebarState } from "$lib/sidebar.svelte";
  import type { Project } from "$lib/types";
  import ProjectTreeNode from "./ProjectTreeNode.svelte";

  /** Matches `KanbanGrid.svelte`'s own drag animation duration. */
  const FLIP_DURATION_MS = 150;

  interface Props {
    project: Project;
    depth: number;
    onCreateSubproject: (parent: Project) => void;
  }

  let { project, depth, onCreateSubproject }: Props = $props();

  let children = $derived(childrenOf(projectsState.items, project.id));
  let hasChildren = $derived(children.length > 0);
  let expanded = $derived(isExpanded(project.id));

  let zoneItems = $state<Project[]>([]);
  $effect(() => {
    zoneItems = children;
  });

  let dropError = $state("");

  function handleConsider(event: CustomEvent<DndEvent<Project>>) {
    zoneItems = event.detail.items;
  }

  async function handleFinalize(event: CustomEvent<DndEvent<Project>>) {
    zoneItems = event.detail.items;
    const { updates, rejected } = computeZoneOrderUpdates(projectsState.items, project.id, zoneItems);
    if (rejected) {
      dropError = "Can't move a project into one of its own subprojects.";
      await refreshProjects();
      return;
    }

    dropError = "";
    try {
      for (const update of updates) {
        const target = projectsState.items.find((p) => p.id === update.id);
        if (!target) continue;
        await updateProject({ ...target, parent_id: update.parent_id, order: update.order });
      }
      await refreshProjects();
    } catch (error) {
      dropError = error instanceof Error ? error.message : "Failed to move project";
      await refreshProjects();
    }
  }
</script>
```

Find the rendering of the subproject `<ul>`:

```svelte
  {#if hasChildren && expanded}
    <ul class="subproject-list">
      {#each children as child (child.id)}
        <ProjectTreeNode project={child} depth={depth + 1} {onCreateSubproject} />
      {/each}
    </ul>
  {/if}
</li>
```

Replace with:

```svelte
  {#if hasChildren && expanded}
    <ul
      class="subproject-list"
      use:dndzone={{ items: zoneItems, flipDurationMs: FLIP_DURATION_MS, dropTargetStyle: {} }}
      onconsider={handleConsider}
      onfinalize={handleFinalize}
    >
      {#each zoneItems as child (child.id)}
        <ProjectTreeNode project={child} depth={depth + 1} {onCreateSubproject} />
      {/each}
    </ul>
    {#if dropError}<p class="drop-error" role="alert">{dropError}</p>{/if}
  {/if}
</li>
```

Add to the `<style>` block:

```css
  .drop-error {
    margin: 0;
    padding: var(--space-2xs) var(--space-sm);
    font-size: var(--text-xs);
    color: var(--color-danger);
  }
```

- [ ] **Step 6: Wire drag-and-drop into `Sidebar.svelte`'s top-level list**

Find:

```ts
  import { page } from "$app/state";
  import NewProjectModal from "./NewProjectModal.svelte";
  import ProjectTreeNode from "./ProjectTreeNode.svelte";
  import { projectsState } from "$lib/projects.svelte";
  import { childrenOf } from "$lib/projectTree";
  import { expandIfUnset } from "$lib/projectTree.svelte";
  import { sidebarState, toggleSidebar } from "$lib/sidebar.svelte";
  import type { Project } from "$lib/types";

  let newProjectOpen = $state(false);
  let subprojectParent: Project | undefined = $state(undefined);

  let topLevelProjects = $derived(childrenOf(projectsState.items, undefined));
```

Replace with:

```ts
  import { dndzone, type DndEvent } from "svelte-dnd-action";
  import { page } from "$app/state";
  import { updateProject } from "$lib/api";
  import NewProjectModal from "./NewProjectModal.svelte";
  import ProjectTreeNode from "./ProjectTreeNode.svelte";
  import { projectsState, refreshProjects } from "$lib/projects.svelte";
  import { childrenOf, computeZoneOrderUpdates } from "$lib/projectTree";
  import { expandIfUnset } from "$lib/projectTree.svelte";
  import { sidebarState, toggleSidebar } from "$lib/sidebar.svelte";
  import type { Project } from "$lib/types";

  const FLIP_DURATION_MS = 150;

  let newProjectOpen = $state(false);
  let subprojectParent: Project | undefined = $state(undefined);
  let dropError = $state("");

  let topLevelProjects = $derived(childrenOf(projectsState.items, undefined));
  let zoneItems = $state<Project[]>([]);
  $effect(() => {
    zoneItems = topLevelProjects;
  });

  function handleConsider(event: CustomEvent<DndEvent<Project>>) {
    zoneItems = event.detail.items;
  }

  async function handleFinalize(event: CustomEvent<DndEvent<Project>>) {
    zoneItems = event.detail.items;
    const { updates, rejected } = computeZoneOrderUpdates(projectsState.items, undefined, zoneItems);
    if (rejected) {
      dropError = "Can't move a project into one of its own subprojects.";
      await refreshProjects();
      return;
    }

    dropError = "";
    try {
      for (const update of updates) {
        const target = projectsState.items.find((p) => p.id === update.id);
        if (!target) continue;
        await updateProject({ ...target, parent_id: update.parent_id, order: update.order });
      }
      await refreshProjects();
    } catch (error) {
      dropError = error instanceof Error ? error.message : "Failed to move project";
      await refreshProjects();
    }
  }
```

Find:

```svelte
    <ul class="project-list">
      {#each topLevelProjects as project (project.id)}
        <ProjectTreeNode {project} depth={0} onCreateSubproject={openNewSubprojectModal} />
      {/each}
    </ul>
```

Replace with:

```svelte
    <ul
      class="project-list"
      use:dndzone={{ items: zoneItems, flipDurationMs: FLIP_DURATION_MS, dropTargetStyle: {} }}
      onconsider={handleConsider}
      onfinalize={handleFinalize}
    >
      {#each zoneItems as project (project.id)}
        <ProjectTreeNode {project} depth={0} onCreateSubproject={openNewSubprojectModal} />
      {/each}
    </ul>
    {#if dropError}<p class="drop-error" role="alert">{dropError}</p>{/if}
```

Add to `Sidebar.svelte`'s `<style>` block (mirroring `ProjectTreeNode.svelte`'s):

```css
  .drop-error {
    margin: 0;
    padding: var(--space-2xs) var(--space-sm);
    font-size: var(--text-xs);
    color: var(--color-danger);
  }
```

- [ ] **Step 7: Run the full frontend verification suite**

Run: `npx vitest run && npx svelte-check && npm run build`
Expected: no regressions, 0 type errors, build succeeds. Drag-and-drop interaction itself (does dropping into a zone actually feel right, does the cycle-rejection error display correctly) needs your manual test — same disclosed limitation as the rest of this codebase's prior drag-and-drop and dialog UI work.

- [ ] **Step 8: Commit**

```bash
git add src/lib/projectTree.ts src/lib/projectTree.test.ts src/lib/components/ProjectTreeNode.svelte src/lib/components/Sidebar.svelte
git commit -m "feat: drag-and-drop project re-parenting in the sidebar"
```

---

## Task 13: Frontend ancestor-chain board resolution + "Parent project" picker

**Files:**
- Modify: `src/lib/projectColor.ts`, `src/lib/projectColor.test.ts`
- Modify: `src/lib/projectBoardSettings.ts`
- Create: `src/lib/projectBoardSettings.test.ts` (if it doesn't already exist — check first)
- Modify: `src/lib/components/KanbanBoard.svelte`
- Modify: `src/lib/components/ProjectBoardSettings.svelte`

**Interfaces:**
- Consumes: `selfAndAncestors`, `ancestorsOf`, `descendantsOf` (Task 8), `wouldCreateCycle` (Task 8).
- Produces: `resolveProjectColor`/`resolveCardLightness`/`resolveBarLightness`/`resolveInkMode` (same signatures as Task 7, now internally chain-aware), `effectiveBoardStatuses(boardChain: ProjectBoard[], allStatusIds: string[])` (signature change — now takes a chain, not one board), `resolveShowPreviousWeeks(boardChain, globalDefault)` (new).

Task 7 made these resolvers id-based but still single-project (own board vs. global only). This task makes them walk the ancestor chain, matching the backend's Task 3 — and adds the settings-page UI to move a project to a different parent, plus visibility into which ancestor an inherited value actually comes from.

- [ ] **Step 1: Write the failing tests for chain-aware `projectColor.ts`**

Edit `src/lib/projectColor.test.ts`. Find the fixture helper:

```ts
const project = (name: string, color: string, board: Partial<ProjectBoard> = {}): Project => ({
  id: name.toLowerCase(),
  name,
  color,
  order: 0,
  created: "2026-06-18T00:00:00Z",
  board: { statuses: [], default_status: undefined, ...board },
  defaults: { tags: [] },
});
```

Replace with:

```ts
const project = (
  name: string,
  color: string,
  board: Partial<ProjectBoard> = {},
  parentId?: string,
): Project => ({
  id: name.toLowerCase(),
  name,
  color,
  parent_id: parentId,
  order: 0,
  created: "2026-06-18T00:00:00Z",
  board: { statuses: [], default_status: undefined, ...board },
  defaults: { tags: [] },
});
```

Add a new `describe` block (alongside the existing ones, using the now-id-based call style from Task 7):

```ts
describe("ancestor-chain resolution", () => {
  test("resolveCardLightness falls through to a grandparent's override", () => {
    const grandparent = project("Grandparent", "#111111", { card_lightness: 0.7 });
    const parent = project("Parent", "#222222", {}, grandparent.id);
    const child = project("Child", "#333333", {}, parent.id);
    const projects = [grandparent, parent, child];

    expect(resolveCardLightness(child.id, projects, 0.5)).toBe(0.7);
  });

  test("resolveCardLightness prefers the nearest override over a further one", () => {
    const grandparent = project("Grandparent", "#111111", { card_lightness: 0.7 });
    const parent = project("Parent", "#222222", { card_lightness: 0.9 }, grandparent.id);
    const child = project("Child", "#333333", {}, parent.id);
    const projects = [grandparent, parent, child];

    expect(resolveCardLightness(child.id, projects, 0.5)).toBe(0.9);
  });

  test("resolveBarLightness falls through to a grandparent's override", () => {
    const grandparent = project("Grandparent", "#111111", { bar_lightness: 0.6 });
    const parent = project("Parent", "#222222", {}, grandparent.id);
    const child = project("Child", "#333333", {}, parent.id);
    const projects = [grandparent, parent, child];

    expect(resolveBarLightness(child.id, projects, 0.38)).toBe(0.6);
  });

  test("resolveInkMode falls through to a grandparent's override", () => {
    const grandparent = project("Grandparent", "#111111", { ink_mode: "white" });
    const parent = project("Parent", "#222222", {}, grandparent.id);
    const child = project("Child", "#333333", {}, parent.id);
    const projects = [grandparent, parent, child];

    expect(resolveInkMode(child.id, projects, "auto")).toBe("white");
  });

  test("falls back to global when no project in the chain has an override", () => {
    const grandparent = project("Grandparent", "#111111");
    const parent = project("Parent", "#222222", {}, grandparent.id);
    const child = project("Child", "#333333", {}, parent.id);
    const projects = [grandparent, parent, child];

    expect(resolveCardLightness(child.id, projects, 0.5)).toBe(0.5);
  });
});
```

- [ ] **Step 2: Run the tests to confirm they fail**

Run: `npx vitest run projectColor -t "ancestor-chain"`
Expected: FAIL — the grandparent's override isn't being found (current implementation only checks the project named by `projectId` directly, not its ancestors).

- [ ] **Step 3: Make `projectColor.ts` walk the ancestor chain**

Replace the whole file `src/lib/projectColor.ts`:

```ts
import type { InkMode } from "./colorPresets";
import { selfAndAncestors } from "./projectTree";
import { DEFAULT_PROJECT_COLOR, type Project } from "./types";

/** The project's stored color, or `DEFAULT_PROJECT_COLOR` for tasks with no project or an unrecognized one. Color itself is never inherited — only board overrides are — so this still checks just the one project, not its ancestors. */
export function resolveProjectColor(projectId: string | undefined, projects: Project[]): string {
  if (!projectId) return DEFAULT_PROJECT_COLOR;
  return projects.find((project) => project.id === projectId)?.color ?? DEFAULT_PROJECT_COLOR;
}

/**
 * The nearest `board.card_lightness` override in `projectId`'s ancestor
 * chain (the project itself, then its ancestors nearest-first — see
 * `selfAndAncestors`), or `globalLightness` if none of them have one.
 * `?? globalLightness` (not `||`) so an override of exactly `0` is
 * respected rather than treated as unset.
 */
export function resolveCardLightness(
  projectId: string | undefined,
  projects: Project[],
  globalLightness: number,
): number {
  if (!projectId) return globalLightness;
  const chain = selfAndAncestors(projects, projectId);
  return chain.find((p) => p.board.card_lightness !== undefined)?.board.card_lightness ?? globalLightness;
}

/** Same as `resolveCardLightness`, but for `board.bar_lightness` (week/calendar-view bars). */
export function resolveBarLightness(
  projectId: string | undefined,
  projects: Project[],
  globalLightness: number,
): number {
  if (!projectId) return globalLightness;
  const chain = selfAndAncestors(projects, projectId);
  return chain.find((p) => p.board.bar_lightness !== undefined)?.board.bar_lightness ?? globalLightness;
}

/** Same as `resolveCardLightness`, but for `board.ink_mode` (color-coded card/bar text). */
export function resolveInkMode(
  projectId: string | undefined,
  projects: Project[],
  globalInkMode: InkMode,
): InkMode {
  if (!projectId) return globalInkMode;
  const chain = selfAndAncestors(projects, projectId);
  return chain.find((p) => p.board.ink_mode !== undefined)?.board.ink_mode ?? globalInkMode;
}
```

- [ ] **Step 4: Run the tests to confirm they pass**

Run: `npx vitest run projectColor`
Expected: every test in the file passes, including the 5 new chain tests.

- [ ] **Step 5: Make `effectiveBoardStatuses` chain-aware**

Check whether `src/lib/projectBoardSettings.test.ts` already exists (`ls src/lib/projectBoardSettings.test.ts`). If it does, add the new tests below into its existing `describe("effectiveBoardStatuses", ...)` block, updating the existing tests' calls to wrap their single board in a one-element array. If it doesn't exist, create it with the existing tests reconstructed plus the new ones — read `boardsEqual`'s and `effectiveBoardStatuses`'s current behavior from `src/lib/projectBoardSettings.ts` directly (already shown above) to write accurate "before" assertions.

Add (or update) these tests:

```ts
describe("effectiveBoardStatuses", () => {
  test("returns the global list when nothing in the chain is customized", () => {
    expect(effectiveBoardStatuses([{ statuses: [], default_status: undefined }], ["a", "b"])).toEqual(["a", "b"]);
  });

  test("returns the project's own customization when set", () => {
    expect(
      effectiveBoardStatuses([{ statuses: ["b", "a"], default_status: undefined }], ["a", "b"]),
    ).toEqual(["b", "a"]);
  });

  test("falls through to a grandparent's customization", () => {
    const own = { statuses: [], default_status: undefined };
    const parent = { statuses: [], default_status: undefined };
    const grandparent = { statuses: ["b", "a"], default_status: undefined };

    expect(effectiveBoardStatuses([own, parent, grandparent], ["a", "b"])).toEqual(["b", "a"]);
  });

  test("prefers the nearest customization over a further one", () => {
    const own = { statuses: [], default_status: undefined };
    const parent = { statuses: ["a", "b"], default_status: undefined };
    const grandparent = { statuses: ["b", "a"], default_status: undefined };

    expect(effectiveBoardStatuses([own, parent, grandparent], ["a", "b"])).toEqual(["a", "b"]);
  });
});
```

- [ ] **Step 6: Run the tests to confirm they fail, then update the implementation**

Run: `npx vitest run projectBoardSettings`
Expected: FAIL — `effectiveBoardStatuses` still takes a single `ProjectBoard`, not an array.

Edit `src/lib/projectBoardSettings.ts`. Find:

```ts
/**
 * Returns the status ids shown as columns on a board, in display order:
 * `board.statuses` if the board has been customized, otherwise every id in
 * `allStatusIds` (the global status list, in its configured order).
 */
export function effectiveBoardStatuses(board: ProjectBoard, allStatusIds: string[]): string[] {
  return board.statuses.length > 0 ? board.statuses : allStatusIds;
}
```

Replace with:

```ts
/**
 * Returns the status ids shown as columns on a board, in display order:
 * the nearest customized board in `boardChain` (a project's own board,
 * then its ancestors' boards, nearest-first — see
 * `crate::project_tree::self_and_ancestors`'s frontend mirror,
 * `selfAndAncestors`) if any has a non-empty `statuses` list, otherwise
 * every id in `allStatusIds` (the global status list, in its configured
 * order).
 */
export function effectiveBoardStatuses(boardChain: ProjectBoard[], allStatusIds: string[]): string[] {
  const customized = boardChain.find((board) => board.statuses.length > 0);
  return customized ? customized.statuses : allStatusIds;
}
```

- [ ] **Step 7: Run the tests to confirm they pass**

Run: `npx vitest run projectBoardSettings`
Expected: all tests pass.

- [ ] **Step 8: Update `KanbanBoard.svelte`'s callers to pass a chain**

Find:

```ts
  /**
   * The status ids shown as columns on this board, in display order: the
   * project's configured board subset if it has one, otherwise every status
   * in the global list.
   */
  let boardStatusIds = $derived(
    project && project.board.statuses.length > 0
      ? project.board.statuses
      : statuses.map((status) => status.id),
  );

  /** This project's `board.show_previous_weeks` override if set, else the global default. */
  let showPreviousWeeksColumn = $derived(
    project?.board.show_previous_weeks ?? settingsState.current?.show_previous_weeks_column ?? false,
  );
```

Replace with:

```ts
  /** The full ancestor chain for the current project (own board first, then ancestors' boards, nearest-first), or empty when this board isn't project-scoped. */
  let projectChain = $derived(project ? selfAndAncestors(projectsState.items, project.id) : []);

  /**
   * The status ids shown as columns on this board, in display order: the
   * nearest customized board in `projectChain` if any has one, otherwise
   * every status in the global list.
   */
  let boardStatusIds = $derived(
    effectiveBoardStatuses(
      projectChain.map((p) => p.board),
      statuses.map((status) => status.id),
    ),
  );

  /** The nearest `board.show_previous_weeks` override in `projectChain`, else the global default. */
  let showPreviousWeeksColumn = $derived(
    projectChain.find((p) => p.board.show_previous_weeks !== undefined)?.board.show_previous_weeks ??
      settingsState.current?.show_previous_weeks_column ??
      false,
  );
```

Add the two new imports at the top of `KanbanBoard.svelte` (alongside its existing `$lib/...` imports — check the current import list first with `grep -n "^  import" src/lib/components/KanbanBoard.svelte` and add these two without duplicating any already present):

```ts
  import { effectiveBoardStatuses } from "$lib/projectBoardSettings";
  import { selfAndAncestors } from "$lib/projectTree";
```

- [ ] **Step 9: Run the full frontend verification suite**

Run: `npx vitest run && npx svelte-check`
Expected: no regressions, 0 type errors. (`npm run build` is deferred to the end of this task, after Step 13.)

- [ ] **Step 10: Add the "Parent project" picker and "inherited from" indicators to `ProjectBoardSettings.svelte`**

Find the imports:

```ts
  import { updateProject } from "$lib/api";
  import {
    legibleInkColor,
    NEON_CARD_CHROMA_BOOST,
    WEEK_BAR_CHROMA_BOOST,
    neonCardColor,
    type InkMode,
  } from "$lib/colorPresets";
  import { boardsEqual, effectiveBoardStatuses } from "$lib/projectBoardSettings";
  import { refreshProjects } from "$lib/projects.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import { FALLBACK_STATUSES, sortedStatuses, statusColor, statusLabel } from "$lib/statuses.svelte";
  import type { Project } from "$lib/types";

  interface Props {
    project: Project;
  }

  let { project }: Props = $props();

  let statuses = $derived(sortedStatuses(settingsState.current?.statuses ?? FALLBACK_STATUSES));
  let allStatusIds = $derived(statuses.map((status) => status.id));

  let baselineStatuses = $derived(effectiveBoardStatuses(project.board, allStatusIds));
  let baselineDefault = $derived(project.board.default_status ?? "");
```

Replace with:

```ts
  import { updateProject } from "$lib/api";
  import {
    legibleInkColor,
    NEON_CARD_CHROMA_BOOST,
    WEEK_BAR_CHROMA_BOOST,
    neonCardColor,
    type InkMode,
  } from "$lib/colorPresets";
  import { boardsEqual, effectiveBoardStatuses } from "$lib/projectBoardSettings";
  import { projectsState, refreshProjects } from "$lib/projects.svelte";
  import { ancestorsOf, descendantsOf, selfAndAncestors } from "$lib/projectTree";
  import { settingsState } from "$lib/settings.svelte";
  import { FALLBACK_STATUSES, sortedStatuses, statusColor, statusLabel } from "$lib/statuses.svelte";
  import type { Project } from "$lib/types";

  interface Props {
    project: Project;
  }

  let { project }: Props = $props();

  /** Every project this one could be moved under, without creating a cycle: everything except itself and its own descendants. */
  let parentCandidates = $derived(
    projectsState.items.filter((p) => p.id !== project.id && !descendantsOf(projectsState.items, project.id).some((d) => d.id === p.id)),
  );

  /** This project's own board, then its ancestors' boards, nearest-first — the chain `effectiveBoardStatuses`/etc. resolve through. */
  let boardChain = $derived(selfAndAncestors(projectsState.items, project.id).map((p) => p.board));
  /** The nearest *ancestor* (not including this project itself) whose board customizes its status subset, if any — used to label an inherited (not self-set) value. */
  let statusesInheritedFrom = $derived(
    project.board.statuses.length === 0
      ? ancestorsOf(projectsState.items, project.id).find((p) => p.board.statuses.length > 0)?.name
      : undefined,
  );
  let cardLightnessInheritedFrom = $derived(
    project.board.card_lightness === undefined
      ? ancestorsOf(projectsState.items, project.id).find((p) => p.board.card_lightness !== undefined)?.name
      : undefined,
  );
  let barLightnessInheritedFrom = $derived(
    project.board.bar_lightness === undefined
      ? ancestorsOf(projectsState.items, project.id).find((p) => p.board.bar_lightness !== undefined)?.name
      : undefined,
  );
  let inkModeInheritedFrom = $derived(
    project.board.ink_mode === undefined
      ? ancestorsOf(projectsState.items, project.id).find((p) => p.board.ink_mode !== undefined)?.name
      : undefined,
  );

  let statuses = $derived(sortedStatuses(settingsState.current?.statuses ?? FALLBACK_STATUSES));
  let allStatusIds = $derived(statuses.map((status) => status.id));

  let baselineStatuses = $derived(effectiveBoardStatuses(boardChain, allStatusIds));
  let baselineDefault = $derived(project.board.default_status ?? "");
  let draftParentId = $state("");
  let baselineParentId = $derived(project.parent_id ?? "");
```

Find the rest of the `baseline*`/`draft*` declarations a little further down (already shown earlier in this plan's research) and add, right after the existing `draftInkModeOverride`/`draftInkMode` declarations:

```ts
  let draftCardLightnessOverride = $state(false);
  let draftCardLightness = $state(50);
  let draftBarLightnessOverride = $state(false);
  let draftBarLightness = $state(38);
  let draftInkModeOverride = $state(false);
  let draftInkMode: InkMode = $state("auto");
```

(this block is unchanged — shown only to locate the insertion point) — directly below it, the `$effect` that seeds drafts from baseline once settings load: find

```ts
  $effect(() => {
    if (settingsState.current && !initialized) {
      draftStatuses = [...baselineStatuses];
      draftDefault = baselineDefault;
      draftShowPreviousWeeks = baselineShowPreviousWeeks;
      draftCardLightnessOverride = baselineCardLightnessOverride;
      draftCardLightness = baselineCardLightness;
      draftBarLightnessOverride = baselineBarLightnessOverride;
      draftBarLightness = baselineBarLightness;
      draftInkModeOverride = baselineInkModeOverride;
```

Replace its first two assignment lines with (adding the new field, leaving the rest of the block exactly as it already continues below):

```ts
  $effect(() => {
    if (settingsState.current && !initialized) {
      draftParentId = baselineParentId;
      draftStatuses = [...baselineStatuses];
      draftDefault = baselineDefault;
      draftShowPreviousWeeks = baselineShowPreviousWeeks;
      draftCardLightnessOverride = baselineCardLightnessOverride;
      draftCardLightness = baselineCardLightness;
      draftBarLightnessOverride = baselineBarLightnessOverride;
      draftBarLightness = baselineBarLightness;
      draftInkModeOverride = baselineInkModeOverride;
```

Find `isDirty`:

```ts
  let isDirty = $derived(
    !boardsEqual(
      { statuses: draftStatuses, default_status: draftDefault || undefined },
      { statuses: baselineStatuses, default_status: project.board.default_status },
    ) ||
```

Replace with:

```ts
  let isDirty = $derived(
    draftParentId !== baselineParentId ||
    !boardsEqual(
      { statuses: draftStatuses, default_status: draftDefault || undefined },
      { statuses: baselineStatuses, default_status: project.board.default_status },
    ) ||
```

Find `discardChanges`:

```ts
  function discardChanges() {
    draftStatuses = [...baselineStatuses];
```

Replace with:

```ts
  function discardChanges() {
    draftParentId = baselineParentId;
    draftStatuses = [...baselineStatuses];
```

Find `save`:

```ts
  async function save() {
    isSaving = true;
    try {
      await updateProject({
        ...project,
        board: {
```

Replace with:

```ts
  async function save() {
    isSaving = true;
    try {
      await updateProject({
        ...project,
        parent_id: draftParentId || undefined,
        board: {
```

- [ ] **Step 11: Add the "Parent project" picker and inherited-from labels to the template**

Find the section start:

```svelte
<section aria-labelledby="board-heading">
  <div class="section-header">
    <h2 id="board-heading">Board columns</h2>
  </div>
```

Replace with (adds a new field block before the existing "Board columns" section):

```svelte
<section aria-labelledby="parent-heading">
  <div class="section-header">
    <h2 id="parent-heading">Parent project</h2>
  </div>
  <p class="description">Move this project under a different parent, or leave it at the top level.</p>
  <div class="field">
    <label for="parent-project">Parent</label>
    <select id="parent-project" bind:value={draftParentId}>
      <option value="">No parent (top level)</option>
      {#each parentCandidates as candidate (candidate.id)}
        <option value={candidate.id}>{candidate.name}</option>
      {/each}
    </select>
  </div>
</section>

<section aria-labelledby="board-heading">
  <div class="section-header">
    <h2 id="board-heading">Board columns</h2>
  </div>
```

Find the status-list field (around the existing `<div class="field">` blocks at lines ~227/237 per the earlier research — search for whichever one renders `draftStatuses`/the status reorder list) and add an inherited-from note directly after its closing `</div>`, before the next field — e.g.:

```svelte
  {#if statusesInheritedFrom}
    <p class="inherited-note">Inherited from {statusesInheritedFrom}.</p>
  {/if}
```

Apply the equivalent `{#if X}<p class="inherited-note">Inherited from {X}.</p>{/if}` pattern after each of the card-lightness, bar-lightness, and ink-mode field blocks, using `cardLightnessInheritedFrom`, `barLightnessInheritedFrom`, and `inkModeInheritedFrom` respectively — find each field's existing markup block (its own `<div class="field">...</div>` for that specific setting) and insert the note immediately after it, only shown when that specific override isn't set directly on this project but is on some ancestor.

Add to the `<style>` block:

```css
  .inherited-note {
    margin: calc(-1 * var(--space-2xs)) 0 0;
    font-size: var(--text-xs);
    color: var(--color-ink-muted);
    font-style: italic;
  }
```

- [ ] **Step 12: Run the full frontend verification suite**

Run: `npx vitest run && npx svelte-check && npm run build`
Expected: no regressions, 0 type errors, build succeeds.

- [ ] **Step 13: Commit**

```bash
git add src/lib/projectColor.ts src/lib/projectColor.test.ts src/lib/projectBoardSettings.ts src/lib/projectBoardSettings.test.ts src/lib/components/KanbanBoard.svelte src/lib/components/ProjectBoardSettings.svelte
git commit -m "feat: board settings inherit through the full ancestor chain; add parent picker"
```

---

## Task 14: Breadcrumb on the project page

**Files:**
- Modify: `src/routes/projects/[id]/+page.svelte`

**Interfaces:**
- Consumes: `ancestorsOf` (Task 8).

No store/logic changes — purely a small template addition, verified by `svelte-check`/`build` plus your manual visual check.

- [ ] **Step 1: Add the breadcrumb**

Replace the whole file `src/routes/projects/[id]/+page.svelte`:

```svelte
<script lang="ts">
  import { page } from "$app/state";
  import KanbanBoard from "$lib/components/KanbanBoard.svelte";
  import { projectsState } from "$lib/projects.svelte";
  import { ancestorsOf } from "$lib/projectTree";

  let project = $derived(projectsState.items.find((p) => p.id === page.params.id));
  /** Root-first ancestor trail, for the breadcrumb above the board. Empty for a top-level project. */
  let breadcrumb = $derived(project ? [...ancestorsOf(projectsState.items, project.id)].reverse() : []);
</script>

{#if project}
  {#key project.id}
    {#if breadcrumb.length > 0}
      <nav aria-label="Project breadcrumb" class="breadcrumb">
        {#each breadcrumb as ancestor, index (ancestor.id)}
          <a href="/projects/{ancestor.id}">{ancestor.name}</a>
          <span aria-hidden="true">/</span>
        {/each}
        <span class="current">{project.name}</span>
      </nav>
    {/if}
    <KanbanBoard title={project.name} accentColor={project.color} projectFilter={project.id} />
  {/key}
{:else}
  <main class="page">
    <p class="placeholder">Project not found.</p>
  </main>
{/if}

<style>
  .page {
    max-width: 1200px;
    margin: 0 auto;
    padding: var(--space-xl) var(--space-lg) var(--space-2xl);
  }

  .placeholder {
    color: var(--color-ink-muted);
    font-size: var(--text-sm);
  }

  .breadcrumb {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    max-width: 1200px;
    margin: 0 auto;
    padding: var(--space-md) var(--space-lg) 0;
    font-size: var(--text-sm);
    color: var(--color-ink-muted);
  }

  .breadcrumb a {
    color: var(--color-ink-muted);
    text-decoration: none;
  }

  .breadcrumb a:hover {
    color: var(--color-accent);
    text-decoration: underline;
  }

  .breadcrumb .current {
    color: var(--color-ink);
    font-weight: 600;
  }
</style>
```

- [ ] **Step 2: Run the full frontend verification suite**

Run: `npx vitest run && npx svelte-check && npm run build`
Expected: no regressions, 0 type errors, build succeeds.

- [ ] **Step 3: Commit**

```bash
git add "src/routes/projects/[id]/+page.svelte"
git commit -m "feat: breadcrumb trail on subproject pages"
```

---

## Task 15: Task rollup across descendant subprojects + origin badge

**Files:**
- Modify: `src/lib/components/KanbanBoard.svelte`
- Modify: `src/lib/components/TaskCard.svelte`
- Modify: `src/lib/components/WeekBarItem.svelte`

**Interfaces:**
- Consumes: `descendantsOf` (Task 8).

**Design choice — no new props needed for the origin badge:** rather than threading a new prop through `KanbanGrid.svelte`/`WeekView.svelte`/`CalendarView.svelte` down to `TaskCard`/`WeekBarItem`, both components read the currently-viewed project directly from the URL via `page.params.id` (`$app/state`) — already available globally, already `undefined` on the non-project-scoped "All Tasks" route, and exactly equal to the viewed project's id on `/projects/[id]`. This avoids prop-drilling through three intermediate components that otherwise have no other reason to know about rollup at all.

- [ ] **Step 1: Roll up descendant tasks in `KanbanBoard.svelte`**

Find:

```ts
  /**
   * The current project, looked up by id, when this board is scoped to a
   * project via `projectFilter`.
   */
  let project = $derived(
    projectFilter ? projectsState.items.find((p) => p.id === projectFilter) : undefined,
  );
```

Replace with:

```ts
  /**
   * The current project, looked up by id, when this board is scoped to a
   * project via `projectFilter`.
   */
  let project = $derived(
    projectFilter ? projectsState.items.find((p) => p.id === projectFilter) : undefined,
  );

  /** `projectFilter` plus every one of its descendant subprojects' ids — viewing a parent project rolls up all of its descendants' tasks too. */
  let rollupProjectIds = $derived(
    projectFilter
      ? [projectFilter, ...descendantsOf(projectsState.items, projectFilter).map((p) => p.id)]
      : [],
  );
```

Find (inside `refresh()`):

```ts
      const visible = projectFilter
        ? allTasks.filter((task) => task.project_id === projectFilter)
        : allTasks;
```

Replace with:

```ts
      const visible = projectFilter
        ? allTasks.filter((task) => task.project_id !== undefined && rollupProjectIds.includes(task.project_id))
        : allTasks;
```

Find (inside `replaceTask`):

```ts
    if (projectFilter && updated.project_id !== projectFilter) {
      removeTask(updated.id);
      return;
    }
```

Replace with:

```ts
    if (projectFilter && (updated.project_id === undefined || !rollupProjectIds.includes(updated.project_id))) {
      removeTask(updated.id);
      return;
    }
```

Add the import (alongside `KanbanBoard.svelte`'s existing `$lib/...` imports):

```ts
  import { descendantsOf } from "$lib/projectTree";
```

- [ ] **Step 2: Add the origin badge to `TaskCard.svelte`**

Find:

```ts
  const projectColor = $derived(resolveProjectColor(task.project_id, projectsState.items));
  const projectName = $derived(projectsState.items.find((p) => p.id === task.project_id)?.name);
```

Replace with:

```ts
  const projectColor = $derived(resolveProjectColor(task.project_id, projectsState.items));
  const projectName = $derived(projectsState.items.find((p) => p.id === task.project_id)?.name);
  /** The project currently being viewed (from the URL), or `undefined` on the non-project-scoped "All Tasks" route. */
  let viewedProjectId = $derived(page.params.id);
  /** `true` when this card is shown via a parent project's rolled-up view — its own project differs from the one currently being viewed. */
  let isRolledUp = $derived(
    viewedProjectId !== undefined && task.project_id !== undefined && task.project_id !== viewedProjectId,
  );
```

Add the import for `page` (check the existing import list first — `TaskCard.svelte` doesn't import `$app/state` today):

```ts
  import { page } from "$app/state";
```

Find the project chip in the template:

```svelte
      {#if projectName && !isColorCoded}
        <span class="chip project" style="--chip-color: {projectColor}; --chip-text-color: {projectChipTextColor}">
          {projectName}
        </span>
      {/if}
```

Replace with:

```svelte
      {#if projectName && !isColorCoded}
        <span class="chip project" style="--chip-color: {projectColor}; --chip-text-color: {projectChipTextColor}">
          {projectName}
        </span>
      {/if}
      {#if isRolledUp && projectName}
        <span class="origin-badge" title={`From ${projectName}`}>
          <span class="origin-dot" style="background: {projectColor}" aria-hidden="true"></span>
          {projectName}
        </span>
      {/if}
```

Add to the `<style>` block:

```css
  .origin-badge {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2xs);
    font-size: var(--text-xs);
    color: var(--color-ink-muted);
  }

  .origin-dot {
    width: 0.5rem;
    height: 0.5rem;
    flex-shrink: 0;
    border-radius: var(--radius-sm);
  }
```

- [ ] **Step 3: Add the origin badge to `WeekBarItem.svelte`**

Find:

```ts
  let barColor = $derived(resolveProjectColor(task.project_id, projectsState.items));
  let projectName = $derived(projectsState.items.find((p) => p.id === task.project_id)?.name);
```

Replace with:

```ts
  let barColor = $derived(resolveProjectColor(task.project_id, projectsState.items));
  let projectName = $derived(projectsState.items.find((p) => p.id === task.project_id)?.name);
  /** The project currently being viewed (from the URL), or `undefined` on the non-project-scoped "All Tasks" route. */
  let viewedProjectId = $derived(page.params.id);
  /** `true` when this bar is shown via a parent project's rolled-up view — its own project differs from the one currently being viewed. */
  let isRolledUp = $derived(
    viewedProjectId !== undefined && task.project_id !== undefined && task.project_id !== viewedProjectId,
  );
```

Add the import for `page`:

```ts
  import { page } from "$app/state";
```

Find:

```svelte
        {#if projectName}
          <span class="chip project" style="--chip-color: {barColor}">{projectName}</span>
        {/if}
```

Replace with:

```svelte
        {#if projectName}
          <span class="chip project" style="--chip-color: {barColor}">{projectName}</span>
        {/if}
        {#if isRolledUp && projectName}
          <span class="origin-badge" title={`From ${projectName}`}>
            <span class="origin-dot" style="background: {barColor}" aria-hidden="true"></span>
          </span>
        {/if}
```

Add to the `<style>` block (mirroring `TaskCard.svelte`'s, sized for the more compact bar layout):

```css
  .origin-badge {
    display: inline-flex;
    align-items: center;
  }

  .origin-dot {
    width: 0.5rem;
    height: 0.5rem;
    flex-shrink: 0;
    border-radius: var(--radius-sm);
  }
```

- [ ] **Step 4: Run the full frontend verification suite**

Run: `npx vitest run && npx svelte-check && npm run build`
Expected: no regressions, 0 type errors, build succeeds. Visual confirmation of the rollup behavior and origin badge placement needs your manual test, same disclosed limitation as the rest of this codebase's UI work.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/KanbanBoard.svelte src/lib/components/TaskCard.svelte src/lib/components/WeekBarItem.svelte
git commit -m "feat: roll up descendant subproject tasks into the parent view, with an origin badge"
```

---

## Task 16: Cascade-delete UI

**Files:**
- Modify: `src/lib/deleteProject.ts`
- Modify: `src/lib/deleteProject.test.ts` (check it exists first; if not, create it)
- Modify: `src/lib/components/DeleteProjectSection.svelte`
- Modify: `src/lib/components/DeleteProjectDialog.svelte`

**Interfaces:**
- Consumes: `descendantsOf` (Task 8).
- Produces: `tasksForProjects(tasks, projectIds)` (new — plural, multi-id version of the existing `tasksForProject`), `reassignTargets` (signature unchanged, now excludes the whole doomed subtree, not just the one project).

The backend (Task 4) already rejects reassigning to a project being deleted as part of the same cascade — this task makes the frontend show accurate counts and exclude doomed descendants from the reassignment picker before the user even gets to that rejection.

- [ ] **Step 1: Write the failing tests**

Check whether `src/lib/deleteProject.test.ts` already exists (`ls src/lib/deleteProject.test.ts`). Add (creating the file with its existing-function tests reconstructed first, if it doesn't exist yet — read `tasksForProject`'s and `reassignTargets`' current behavior from `src/lib/deleteProject.ts`, already shown in full during Task 7, to write accurate "before" tests):

```ts
describe("tasksForProjects", () => {
  test("matches tasks belonging to any of several project ids", () => {
    const tasks: Task[] = [
      { ...baseTask, id: "1", project_id: "a" },
      { ...baseTask, id: "2", project_id: "b" },
      { ...baseTask, id: "3", project_id: "c" },
    ];

    const matching = tasksForProjects(tasks, ["a", "b"]);

    expect(matching.map((t) => t.id)).toEqual(["1", "2"]);
  });

  test("returns an empty array when no task matches", () => {
    const tasks: Task[] = [{ ...baseTask, id: "1", project_id: "a" }];

    expect(tasksForProjects(tasks, ["z"])).toEqual([]);
  });
});

describe("reassignTargets (with descendants)", () => {
  test("excludes the project being deleted and its descendants", () => {
    const parent = { ...baseProject, id: "parent" };
    const child = { ...baseProject, id: "child", parent_id: "parent" };
    const unrelated = { ...baseProject, id: "unrelated" };
    const projects = [parent, child, unrelated];

    const targets = reassignTargets(projects, "parent");

    expect(targets.map((p) => p.id)).toEqual(["unrelated"]);
  });
});
```

If creating the file fresh, add the necessary `baseTask`/`baseProject` fixture objects and the `import { describe, expect, test } from "vitest";` / `import { reassignTargets, tasksForProjects } from "./deleteProject";` / `import type { Project, Task } from "./types";` lines, matching whatever minimal-but-complete `Task`/`Project` shape the rest of this codebase's test fixtures already use (see `projectColor.test.ts`'s `project()` helper from Task 13 for the `Project` shape pattern; build an equivalent minimal `Task` fixture with all required fields filled in).

- [ ] **Step 2: Run the tests to confirm they fail**

Run: `npx vitest run deleteProject`
Expected: FAIL — `tasksForProjects is not a function` (and/or the `reassignTargets` descendant-exclusion test fails against the current single-project-only implementation).

- [ ] **Step 3: Implement the changes**

Edit `src/lib/deleteProject.ts`. Find:

```ts
/**
 * Returns the tasks currently filed under `projectId` - mirrors
 * `tasks_for_projects` in `src-tauri/src/commands.rs`.
 */
export function tasksForProject(tasks: Task[], projectId: string): Task[] {
  return tasks.filter((task) => task.project_id === projectId);
}

/** Projects that can be picked as a reassignment target: every project other than the one being deleted. */
export function reassignTargets(projects: Project[], excludeProjectId: string): Project[] {
  return projects.filter((project) => project.id !== excludeProjectId);
}
```

Replace with:

```ts
/**
 * Returns the tasks currently filed under `projectId` - mirrors
 * `tasks_for_projects` (singular form) in `src-tauri/src/commands.rs`.
 */
export function tasksForProject(tasks: Task[], projectId: string): Task[] {
  return tasks.filter((task) => task.project_id === projectId);
}

/**
 * Returns the tasks currently filed under any id in `projectIds` - mirrors
 * `tasks_for_projects` in `src-tauri/src/commands.rs`, used when previewing
 * or performing a cascading delete across a project and its descendants.
 */
export function tasksForProjects(tasks: Task[], projectIds: string[]): Task[] {
  return tasks.filter((task) => task.project_id !== undefined && projectIds.includes(task.project_id));
}

/**
 * Projects that can be picked as a reassignment target: every project
 * other than the one being deleted and its own descendants — both are
 * about to be deleted too, so reassigning into either would just create
 * tasks with nowhere to go once the delete completes.
 */
export function reassignTargets(projects: Project[], excludeProjectId: string): Project[] {
  const excludedIds = new Set([
    excludeProjectId,
    ...descendantsOf(projects, excludeProjectId).map((p) => p.id),
  ]);
  return projects.filter((project) => !excludedIds.has(project.id));
}
```

Add the import at the top of the file:

```ts
import { descendantsOf } from "./projectTree";
```

- [ ] **Step 4: Run the tests to confirm they pass**

Run: `npx vitest run deleteProject`
Expected: all tests pass.

- [ ] **Step 5: Update `DeleteProjectSection.svelte`'s preview to include descendants**

Find:

```ts
  import { deleteProject, listTasks } from "$lib/api";
  import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
  import DeleteProjectDialog from "$lib/components/DeleteProjectDialog.svelte";
  import {
    buildTaskStrategy,
    isDefaultProject,
    reassignTargets,
    tasksForProject,
    type DeleteStrategyKind,
  } from "$lib/deleteProject";
  import { projectsState, refreshProjects } from "$lib/projects.svelte";
  import { settingsState } from "$lib/settings.svelte";
  import type { Project } from "$lib/types";
```

Replace with:

```ts
  import { deleteProject, listTasks } from "$lib/api";
  import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
  import DeleteProjectDialog from "$lib/components/DeleteProjectDialog.svelte";
  import {
    buildTaskStrategy,
    isDefaultProject,
    reassignTargets,
    tasksForProjects,
    type DeleteStrategyKind,
  } from "$lib/deleteProject";
  import { projectsState, refreshProjects } from "$lib/projects.svelte";
  import { descendantsOf } from "$lib/projectTree";
  import { settingsState } from "$lib/settings.svelte";
  import type { Project } from "$lib/types";
```

Find:

```ts
  let errorMessage = $state("");
  let isDeleting = $state(false);
  let simpleConfirmOpen = $state(false);
  let strategyDialogOpen = $state(false);
  let pendingTaskCount = $state(0);

  async function startDelete() {
    errorMessage = "";
    try {
      const tasks = await listTasks();
      pendingTaskCount = tasksForProject(tasks, project.id).length;
      if (pendingTaskCount === 0) {
        simpleConfirmOpen = true;
      } else {
        strategyDialogOpen = true;
      }
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : "Failed to check project tasks";
    }
  }
```

Replace with:

```ts
  let errorMessage = $state("");
  let isDeleting = $state(false);
  let simpleConfirmOpen = $state(false);
  let strategyDialogOpen = $state(false);
  let pendingTaskCount = $state(0);
  let pendingDescendantCount = $state(0);

  async function startDelete() {
    errorMessage = "";
    try {
      const tasks = await listTasks();
      const descendantIds = descendantsOf(projectsState.items, project.id).map((p) => p.id);
      pendingDescendantCount = descendantIds.length;
      pendingTaskCount = tasksForProjects(tasks, [project.id, ...descendantIds]).length;
      if (pendingTaskCount === 0) {
        simpleConfirmOpen = true;
      } else {
        strategyDialogOpen = true;
      }
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : "Failed to check project tasks";
    }
  }
```

Find:

```svelte
<ConfirmDialog
  open={simpleConfirmOpen}
  title="Delete project?"
  message={`Are you sure you want to delete "${project.name}"? This can't be undone.`}
  confirmLabel="Delete"
  onConfirm={confirmSimpleDelete}
  onCancel={() => (simpleConfirmOpen = false)}
/>

<DeleteProjectDialog
  open={strategyDialogOpen}
  projectName={project.name}
  taskCount={pendingTaskCount}
  {otherProjects}
  onConfirm={confirmStrategyDelete}
  onCancel={() => (strategyDialogOpen = false)}
/>
```

Replace with:

```svelte
<ConfirmDialog
  open={simpleConfirmOpen}
  title="Delete project?"
  message={pendingDescendantCount > 0
    ? `Are you sure you want to delete "${project.name}" and its ${pendingDescendantCount} ${pendingDescendantCount === 1 ? "subproject" : "subprojects"}? This can't be undone.`
    : `Are you sure you want to delete "${project.name}"? This can't be undone.`}
  confirmLabel="Delete"
  onConfirm={confirmSimpleDelete}
  onCancel={() => (simpleConfirmOpen = false)}
/>

<DeleteProjectDialog
  open={strategyDialogOpen}
  projectName={project.name}
  taskCount={pendingTaskCount}
  descendantCount={pendingDescendantCount}
  {otherProjects}
  onConfirm={confirmStrategyDelete}
  onCancel={() => (strategyDialogOpen = false)}
/>
```

- [ ] **Step 6: Update `DeleteProjectDialog.svelte`'s messaging**

Find:

```ts
  interface Props {
    open: boolean;
    projectName: string;
    taskCount: number;
    otherProjects: Project[];
    onConfirm: (strategy: DeleteStrategyKind, targetProjectId: string) => void;
    onCancel: () => void;
  }

  let { open, projectName, taskCount, otherProjects, onConfirm, onCancel }: Props = $props();
```

Replace with:

```ts
  interface Props {
    open: boolean;
    projectName: string;
    taskCount: number;
    /** How many descendant subprojects will also be deleted (0 if none). */
    descendantCount: number;
    otherProjects: Project[];
    onConfirm: (strategy: DeleteStrategyKind, targetProjectId: string) => void;
    onCancel: () => void;
  }

  let { open, projectName, taskCount, descendantCount, otherProjects, onConfirm, onCancel }: Props = $props();
```

Find:

```svelte
  <h2 id="delete-project-dialog-heading">Delete "{projectName}"?</h2>
  <p>
    {taskCount} {taskCount === 1 ? "task" : "tasks"} still
    {taskCount === 1 ? "belongs" : "belong"} to this project. Choose what to do with
    {taskCount === 1 ? "it" : "them"} before deleting the project.
  </p>
```

Replace with:

```svelte
  <h2 id="delete-project-dialog-heading">
    Delete "{projectName}"{descendantCount > 0 ? ` and its ${descendantCount} ${descendantCount === 1 ? "subproject" : "subprojects"}` : ""}?
  </h2>
  <p>
    {taskCount} {taskCount === 1 ? "task" : "tasks"}
    {descendantCount > 0 ? "across this project and its subprojects" : ""} still
    {taskCount === 1 ? "belongs" : "belong"} {descendantCount > 0 ? "here" : "to this project"}. Choose what to
    do with {taskCount === 1 ? "it" : "them"} before deleting.
  </p>
```

- [ ] **Step 7: Run the full frontend verification suite**

Run: `npx vitest run && npx svelte-check && npm run build`
Expected: no regressions, 0 type errors, build succeeds.

- [ ] **Step 8: Commit**

```bash
git add src/lib/deleteProject.ts src/lib/deleteProject.test.ts src/lib/components/DeleteProjectSection.svelte src/lib/components/DeleteProjectDialog.svelte
git commit -m "feat: cascade-delete confirmation shows descendant subproject/task counts"
```

---

## Task 17: NL parser disambiguation note (scope-reduced)

**Finding from plan research:** `filterSuggestions` (used by every quick-add prefix, not just `+`) dedupes by string value before `Autocomplete.svelte` ever renders anything — two same-named projects collapse into one dropdown entry today, and the `+` token's parser only ever inserts a bare single-word name (a label like "Homework (CS101)" would break the parser, since it splits on whitespace). Reliable dropdown disambiguation for same-named projects under different parents would require extending the shared `Autocomplete.svelte`/`filterSuggestions` pipeline (used by `#tag`/`!priority`/`@status` too) to carry a display label distinct from the inserted value — a real, separate piece of work, not a one-line fix.

**Decision:** ship the documented limitation as-is for this feature's first version — typing/autocompleting `+Homework` resolves to the first project named "Homework" found in the loaded list, matching the pre-subprojects behavior exactly. Users who need to target a specific same-named subproject unambiguously can do so today via the project-scoped board's own "Add task" (where the project is already pinned by id through `projectFilter`, never resolved by name at all) or by giving subprojects distinct names. Richer dropdown disambiguation can be a focused follow-up once this ships and it's clear how often the collision actually comes up in practice.

No code changes in this task — it exists to record the decision in the plan and log, rather than silently shipping a half-implemented disambiguation feature.

- [ ] **Step 1: No-op — record the decision**

Nothing to implement. When this task is reached during execution, skip straight to confirming the existing (pre-Task-17) behavior already matches this decision — `AddTaskModal.svelte`'s `matchedProject` (from Task 7) already does first-match-by-name, which is the intended final behavior, not an intermediate state needing further work.

---

## Plan-wide final verification

- [ ] **Step 1: Full backend suite**

Run: `cd src-tauri && cargo test --lib && cargo fmt -- --check && cargo clippy --lib -- -D warnings && cargo build`

- [ ] **Step 2: Full frontend suite**

Run: `npx vitest run && npx svelte-check && npm run build`

- [ ] **Step 3: Update `log.md` and `next_instructions.md`**

Per `CLAUDE.md`'s standing rules: append a `log.md` entry at `/home/dohrenator/Master/dev/CLAUDE_HOME/projects/task_wizard/log.md` summarizing what shipped, and append a "completed on <date>" note to the relevant line of `next_instructions.md` (append-only — never overwrite). Update `README.md` to document subprojects (nesting, color shading, settings inheritance, drag-and-drop/picker re-parenting, rollup views, cascade delete) per CLAUDE.md's documentation requirement.

- [ ] **Step 4: Hand off to the user for manual testing**

Everything marked "needs your manual test" throughout this plan (sidebar tree rendering and depth, drag-and-drop feel, cascade-delete confirmation copy, breadcrumb rendering, color-shade swatches) should be tested together once all 17 tasks are merged, before considering this feature done.
