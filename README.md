# taskmancer

A fast, local-first task manager with Kanban boards, projects, natural-language
task entry, and (eventually) time tracking and analytics — built with
[Tauri](https://tauri.app/) and [SvelteKit](https://kit.svelte.dev/).

> **Status:** early development. Core task/project management and theming are
> working; time tracking, recurring tasks, and analytics are on the roadmap.

## Why taskmancer?

- **Local-first & private.** Everything lives on your machine. No accounts,
  no servers, no telemetry.
- **Plain-text storage.** Every task is a markdown file with YAML frontmatter
  — readable, editable, diffable, and git-syncable outside the app.
- **Lightweight.** Tauri ships a native binary that uses your OS's built-in
  webview, so the app stays small (single-digit MB) instead of bundling a
  full browser.
- **Fast capture.** Type things like
  `Buy milk #personal +Errands high due tomorrow sch next monday` and watch
  the title, tags, project, priority, due date, and scheduled date populate
  live.

## Features

- **Kanban board** with drag-and-drop across status columns (Backlog / Do /
  In Progress / Blocked / Done by default — fully customizable, including
  per-project column subsets, from Settings). Columns scroll horizontally
  instead of wrapping, and within each column tasks are grouped by priority
  (toggleable — see [Display settings](#display-settings)). Tasks with a
  `scheduled` date in the future are hidden from the board until that day
  arrives (they remain visible in the Week view).
- **Week view** — a "Board" / "Week" switcher (on both the global and
  per-project boards) shows a 7-day calendar grid with project-colored bars
  for tasks scheduled or due that day. Navigate with prev/next/Today; click a
  bar to see task details and jump into editing. See
  [Week view](#week-view) for details.
- **Compact task cards** — the title sits on one line, with a wrapping row
  of chips below it (priority badge, project, tags, due date) that only
  grows onto extra lines when needed.
- **Projects** as first-class entities: a collapsible sidebar lists every
  project (with its own color), and each project has its own filtered board.
- **Natural-language quick add** (`Ctrl+T` or the `+` button) — recognizes
  `#tag`, `+Project`, bare `high`/`medium`/`low` priority words, and
  `due <phrase>` / `sch <phrase>` (e.g. `due next friday`, `sch tomorrow`).
  A "?" button next to the title field opens a syntax cheat-sheet popover.
- **Tag & project autocomplete** in the Add Task modal and on task cards.
- **Themes & display settings** — Light, Dark, and Dark Blue themes (shared
  OKLCH design-token system), plus app-wide font size and Kanban column width
  sliders and a priority-grouping toggle — all switchable and persisted from
  the Settings page.
- **Subtasks, dependencies, notes** — every task supports a markdown notes
  body and a `depends_on` list of other task IDs.

### Planned

Time tracking, Pomodoro/focus mode, idle detection, calendar/month view (in
addition to the existing Week view), recurring tasks, habit tracking,
analytics dashboards, plugin system, calendar sync, and import/export. See
the in-app roadmap for current priorities.

## Tech stack

| Layer        | Technology |
|--------------|------------|
| Shell        | [Tauri 2](https://tauri.app/) (Rust) |
| Frontend     | [SvelteKit 5](https://kit.svelte.dev/) + TypeScript, static adapter |
| Styling      | Plain CSS with OKLCH design tokens (no CSS framework) |
| Drag & drop  | [svelte-dnd-action](https://github.com/isaacHagoel/svelte-dnd-action) |
| Backend tests| `cargo test` |
| Frontend tests | [Vitest](https://vitest.dev/) |

## How data is stored

Tasks are **markdown files with YAML frontmatter** — the source of truth for
all task data:

```markdown
---
id: f778cfd8-04eb-415b-945e-8aef0a890b63
title: Buy milk
status: do
project: personal
tags:
  - errands
priority: high
due: 2026-06-19
scheduled: 2026-06-17
order: 2000
created: 2026-06-12T03:36:41.058774498+00:00
depends_on: []
---

Optional free-form markdown notes go here.
```

Projects are stored in a small `projects.json` (id, name, color, order).

This data lives **outside the repo**, in your OS's standard application-data
directory (derived from the Tauri app identifier `com.taskmancer.app`):

| OS      | Location |
|---------|----------|
| Linux   | `~/.local/share/com.taskmancer.app/` |
| macOS   | `~/Library/Application Support/com.taskmancer.app/` |
| Windows | `%APPDATA%\com.taskmancer.app\` |

Inside that directory: `tasks/*.md` (one file per task) and `projects.json`.
Because it's just markdown + JSON, you can edit it by hand, back it up, or
sync it with git — the app re-reads from disk.

## Getting started

### Prerequisites

- [Node.js](https://nodejs.org/) 18+ and npm
- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain via `rustup`)
- Platform dependencies for Tauri 2 — see the
  [Tauri prerequisites guide](https://v2.tauri.app/start/prerequisites/)
  (on Linux this includes `webkit2gtk`, `librsvg2`, and friends)

### Install

```bash
git clone <repo-url> taskmancer
cd taskmancer
npm install
```

### Run in development

```bash
npm run tauri dev
```

This starts the Vite dev server and opens the app in a native window with
hot-reload.

### Build a release binary

```bash
npm run tauri build
```

Produces a platform-native bundle (AppImage/deb on Linux, `.app`/`.dmg` on
macOS, etc.) under `src-tauri/target/release/bundle/`.

### Frontend-only commands

```bash
npm run dev      # Vite dev server only (no native window)
npm run build    # Production frontend build
npm run check    # svelte-check (type checking)
```

## Usage

### Quick add syntax

Open the Add Task modal with the **+** button or **Ctrl+T**, then type a
single line. Recognized tokens are stripped from the title and shown live in
the field list:

| Token              | Meaning                          | Example |
|--------------------|-----------------------------------|---------|
| `#tag`             | Adds a tag                       | `#personal` |
| `+Project`         | Assigns a project                | `+Errands` |
| `high` / `medium` / `low` | Sets priority (bare word)  | `high` |
| `due <phrase>` / `due:<token>` | Sets the due date    | `due next friday`, `due:tomorrow` |
| `sch <phrase>` / `sch:<token>` | Sets the scheduled date | `sch next monday` |

Anything left over becomes the task title.

#### Date phrases

`<phrase>` (after a bare `due`/`sch` keyword) accepts:

| Form | Meaning | Example |
|------|---------|---------|
| `today` / `tomorrow` | Relative day | `due today` |
| `YYYY-MM-DD` | Exact ISO date | `due 2026-12-25` |
| `<weekday>` | The next occurrence of that weekday, **including today** | `due friday` |
| `next <weekday>` | *Skips* the upcoming occurrence — one week later than the plain weekday form | `due next monday` |
| `<month> <day>` / `<day> <month>` | An absolute date. Month names may be full or abbreviated (`june`/`jun`); the day may have an ordinal suffix (`11th`). Defaults to this year, rolling over to next year if that date has already passed | `due june 11`, `sch 31st may` |
| `<month> <day> <year>` | Same as above with an explicit 4-digit year (used as-is, even if in the past) | `due june 11 2027` |

`due:<token>` / `sch:<token>` (the colon form) only support `today`, `tomorrow`, `YYYY-MM-DD`, and plain weekday names — not `next <weekday>` or absolute month/day dates.

### Week view

Switch from "Board" to "Week" using the view tabs at the top of the global
board or any project board. The week view shows a 7-day grid (Monday- or
Sunday-start, per [Display settings](#display-settings)) with prev/next/Today
navigation.

Each day column lists a small bar for every task that's **scheduled** for
that day or **due** that day (a task with both gets one bar per date),
tinted with its project's color and marked with a scheduled/due icon.
Clicking a bar opens a popover with the task's details and an **Edit**
button that opens the full task editor.

### Keyboard shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+T` | Open the Add Task modal (from anywhere, except while editing a text field) |

### Themes

Visit **Settings** (gear icon in the sidebar) to switch between Light, Dark,
and Dark Blue. Your choice is saved locally and restored on next launch.

### Display settings

Also on the **Settings** page, under "Display":

| Control | Range | Effect |
|---------|-------|--------|
| Font size | 80%–140% (5% steps) | Scales the whole app's text via the root `font-size`. |
| Status column width | 200px–400px (10px steps) | Width of each Kanban status column. Columns scroll horizontally as a single row instead of wrapping. |
| Group tasks by priority | on/off | When on, each status column is divided into labeled priority sections. When off, tasks stay sorted by priority but the column isn't visually divided — a small priority badge on each card still shows its level. |
| Week starts on | Monday / Sunday | Controls which day is the first column in the [Week view](#week-view). |

All settings apply instantly and are saved in `localStorage`, so they persist
across restarts.

## Testing

```bash
# Frontend unit tests (Vitest)
npm test

# Backend tests (Rust)
cd src-tauri
cargo test
cargo clippy --all-targets
```

## Project structure

```
taskmancer/
├── src/                  # SvelteKit frontend
│   ├── lib/              # Stores, parsers, API client, shared components
│   ├── routes/           # Pages (board, per-project boards, settings)
│   └── styles/           # Design tokens, themes, typography
├── src-tauri/            # Rust/Tauri backend
│   └── src/
│       ├── commands.rs   # Tauri commands (the frontend's API surface)
│       ├── task.rs       # Task model + markdown (de)serialization
│       ├── storage.rs    # Task file I/O
│       ├── project.rs    # Project model
│       └── project_storage.rs
└── static/               # Static assets
```

## License

MIT
