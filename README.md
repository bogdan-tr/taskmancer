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

- **Kanban board** with drag-and-drop across Backlog / Do / In Progress /
  Blocked / Done.
- **Projects** as first-class entities: a collapsible sidebar lists every
  project (with its own color), and each project has its own filtered board.
- **Natural-language quick add** (`Ctrl+T` or the `+` button) — recognizes
  `#tag`, `+Project`, bare `high`/`medium`/`low` priority words, and
  `due <phrase>` / `sch <phrase>` (e.g. `due next friday`, `sch tomorrow`).
- **Tag & project autocomplete** in the Add Task modal and on task cards.
- **Themes** — Light, Dark, and Dark Blue, all built from a shared OKLCH
  design-token system, switchable and persisted from the Settings page.
- **Subtasks, dependencies, notes** — every task supports a markdown notes
  body and a `depends_on` list of other task IDs.

### Planned

Time tracking, Pomodoro/focus mode, idle detection, calendar/daily/weekly
views, recurring tasks, habit tracking, analytics dashboards, plugin system,
calendar sync, and import/export. See the in-app roadmap for current
priorities.

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

### Keyboard shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+T` | Open the Add Task modal (from anywhere, except while editing a text field) |

### Themes

Visit **Settings** (gear icon in the sidebar) to switch between Light, Dark,
and Dark Blue. Your choice is saved locally and restored on next launch.

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
