# Subtasks — Design Spec

**Date:** 2026-06-21
**Status:** Approved by user, queued for implementation after the NL `+Project` path disambiguation work ships (see `2026-06-21-nl-project-path-design.md`) and after a manual-testing pass on Subprojects.
**Scope:** Subtasks only, as described in `next_instructions.md`. Depends on Subprojects (already shipped) — a subtask is implemented as an automatically-generated subproject for a task, reusing the existing project tree/board/settings-inheritance machinery rather than inventing a parallel system.

## Context

`next_instructions.md`'s original request: *"subtasks will essentially be automatically generated subprojects for a task, where the subproject name is the parent task name. The subtask inherits all parent task attributes but can be overridden... It might need to store who the parent is though as an attribute (and parent might need to store its own subtasks) - discuss this with me."* The request also specifies Kanban "glued" nesting, an NL `sub <parent task name>` keyword with autosuggest, a show/hide setting across all views, and an all-subtasks-done popup.

No subtask-specific design work had happened before this spec — the only prior decision on record (in `log.md` and the Subprojects design spec) was to explicitly *defer* Subtasks design to its own cycle, once Subprojects shipped and was tested. Everything below comes from a dedicated four-round Q&A with the user in this session, specifically to avoid building on unstated assumptions.

## Key decisions (all explicit user answers, none assumed)

### Data model
- The parent task gets a new field, `subtask_project_id: Option<String>`, pointing at an auto-generated container `Project`. Lazily created the first time a subtask is added to that task (not eagerly on every task).
- The container `Project` needs **no back-pointer** to the task that owns it — the owning task is whichever one has that id set in `subtask_project_id`. (Considered and rejected: storing the link on both sides for fast lookups either direction — rejected as two fields to keep in sync for a lookup direction nothing actually needs yet.)
- Nesting is **strictly one level deep** — a task that is itself a subtask never gets its own "Create subtask" entry point. (Considered and rejected: arbitrary nesting, reusing Subprojects' existing recursive depth — rejected as more generality than the spec's "flat breakdown of one task" framing calls for.)
- The container `Project`'s `parent_id` is set to the parent task's current `project_id` at creation time, so it inherits board/defaults settings through the **existing ancestor-chain machinery already built for Subprojects** — zero extra backend work for inheritance. It is, however, **filtered out of the sidebar project tree** entirely (hidden, not user-browsable there) — one auto-generated subproject per task-with-subtasks would otherwise clutter the tree fast. It's reachable only via the parent task's own card/views, never by browsing the sidebar.
- **Sync rules**, both explicit user decisions: renaming the parent task renames its container to match (never drifts out of sync with what it's named after); moving the parent task to a different project re-parents the container too (so its inherited settings and tree position always reflect where the parent task currently lives, not where it was created).
- **Empty-container cleanup:** if a task's last remaining subtask is deleted outright (not via the all-done popup, see below), the now-empty container auto-deletes itself. An empty, invisible (hidden from sidebar) container would otherwise linger forever with no way for the user to even notice it. Clicking "Create subtask" again later just creates a fresh one.
- **Parent task deletion cascades:** deleting a parent task deletes its container and every subtask in it, behind a confirmation showing the exact subtask count — mirroring the existing project cascade-delete UX almost exactly. (Considered and rejected: orphaning the subtasks as a parent-less subproject the user has to clean up manually; blocking deletion outright until subtasks are removed first.)

### Creating a subtask
- Entry points (all four, not a single one): a button on the Kanban task card, a button in the task edit dialog, a button in the Week/Calendar bar popover, and the NL `sub <parent task name>` keyword.
- Pre-filled from the parent task into the (otherwise normal) create-task modal, editable before saving: **tags, priority, estimated time, due, and scheduled**. **Not** the parent's notes text.
- **Status is never inherited** — a new subtask always starts at the normal default status (project/global default), regardless of what status the parent currently has. Avoids a brand-new subtask appearing to already be "Done" or "Blocked" just because that's where the parent happens to sit.
- NL autocomplete for `sub <name>` suggests any **active** (non-done, non-cancelled) task that **isn't itself a subtask** (consistent with the one-level rule) — matched case-insensitively, first match wins on a name collision, mirroring the `+Project` token's own documented limitation (now being improved separately, but `sub` is explicitly out of scope for that improvement — see "Relationship to the NL project-path work" below).

### Kanban behavior
- In the parent task's own board (wherever its card normally renders), subtasks appear as a **nested row glued to the parent** — not as independent draggable cards there. Each row shows priority on the left and a **clickable status dot** on the right; clicking it opens a small status picker that changes the subtask's status directly, without leaving the glued view.
- In the subtask container's **own** dedicated board (reached via whatever link/button surfaces it — not the sidebar), each subtask renders as a completely normal, independently-draggable card, exactly like any other project's board.
- The parent task's card shows a small progress badge (e.g. "2/5") summarizing subtask completion. Not explicitly requested in the original notes, but approved as a small, low-risk, in-the-spirit addition once asked.
- A global "show subtasks" setting applies across Board, Week, and Calendar views, in both the global ("All Tasks") and per-project scopes. Turning it off makes subtasks **vanish from view entirely** (not just un-nest into ordinary standalone cards) — turning it back on re-nests them under their parent exactly as before.

### Lifecycle: all subtasks done
- When every (non-cancelled — see below) subtask becomes done, a one-time popup offers to mark the parent task done and/or delete the container. It's a real choice: dismissing without picking either option means it **will not auto-reshow** for that completion. It can only re-trigger if a subtask is later un-done and then all become done again — a fresh transition, not a repeat of the old one.
- **Not explicitly asked, decided here as a small implementation detail:** cancelled subtasks don't count toward the "done" numerator (so a cancelled item doesn't read as "completed work"), but they also don't block the all-done popup from firing — the check is "every *non-cancelled* subtask is done," not "every subtask, full stop." A subtask that was cancelled is no longer active work either way.

## Relationship to the NL project-path work

The same session that produced this spec also produced `2026-06-21-nl-project-path-design.md` (extending `+Project` to support disambiguating same-named subprojects by path, e.g. `+Work/"Client A"`). The user explicitly scoped that work to the `+Project` token **only** — `sub <parent task name>` keeps its existing first-match-by-name behavior and is not touched by it. The two pieces of work are sequenced: NL project-path ships first, then Subtasks.

## Testing approach (anticipated, to be refined into a real plan at implementation time)

Following this codebase's established convention:
- **Rust:** container-project lifecycle helpers (create-on-first-subtask, rename-sync, move-sync, empty-auto-delete, cascade-delete-with-parent) as pure, directly-testable functions wherever the existing Subprojects code already has an equivalent shape to mirror (e.g. `projects_to_delete`, `ensure_default_project`).
- **Frontend (Vitest):** the all-done-popup trigger logic (including the cancelled-subtask exclusion and "don't reshow same transition" rule) and progress-badge counting as pure functions; the NL `sub` token's autocomplete-candidate filtering (active, non-subtask tasks).
- **Manual/visual (disclosed limitation, consistent with every prior UI entry in this codebase's log):** the glued nested-row layout, the clickable status dot's quick-picker feel, and the show/hide setting's effect across Board/Week/Calendar all need a real user-testing pass once implemented — same caveat as Subprojects' own drag-and-drop and tree rendering.

## Out of scope

- Arbitrary-depth subtask nesting (explicitly decided: one level only).
- Showing the subtask container in the sidebar project tree (explicitly decided: hidden).
- Extending the NL project-path disambiguation work to the `sub <task name>` keyword (explicitly scoped to a separate, prior piece of work).
