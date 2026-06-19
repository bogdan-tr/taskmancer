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
- **Week view** — a "Board" / "Week" / "Calendar" switcher (on both the
  global and per-project boards) shows a 7-day calendar grid with
  project-colored bars for tasks scheduled or due that day. Bars are
  **draggable** to reschedule a task to a different day, navigate with
  prev/next/Today, and click a bar to see task details and jump into editing.
  An optional leading "Previous" column lists unfinished tasks scheduled or
  due before the visible week, and a "tasks behind this week" indicator
  always shows the count even when that column is hidden. See
  [Week view](#week-view) for details.
- **Calendar view** — the same switcher's "Calendar" tab shows a full month
  grid (rows grow taller, rather than truncating, on days with many tasks),
  with the same project-colored, draggable bars, done/cancelled styling, and
  popovers as the Week view. Weeks start on the day configured in
  [Display settings](#display-settings). See [Calendar view](#calendar-view).
- **Compact task cards** — the title sits on one line, with a wrapping row
  of chips below it (priority badge, project, tags, due date) that only
  grows onto extra lines when needed. Done tasks show a strikethrough title,
  a checkmark, and a muted gray background with a faint tint of the done
  status's color; cancelled tasks get the same muted background plus an "×"
  — on both Kanban cards and week-view bars. In the week view, finished
  tasks sink to the bottom of their day so active tasks stay up top.
- **Projects** as first-class entities: a collapsible sidebar lists every
  project (with its own color), and each project has its own filtered board.
- **Natural-language quick add** (`Ctrl+T` or the `+` button) — recognizes
  `#tag`, `+Project`, bare `high`/`medium`/`low` priority words, `@status`,
  and `due <phrase>` / `sch <phrase>` (e.g. `due next friday`, `sch tomorrow`).
  Typing a bare `#`/`+`/`!`/`@` lists every available tag/project/priority/
  status. A "?" button next to the title field opens a syntax cheat-sheet
  popover.
- **Tag, project, priority & status autocomplete** in the Add Task modal and
  on task cards.
- **Themes & display settings** — Light, Dark, and Dark Blue themes (shared
  OKLCH design-token system), plus app-wide font size and Kanban column width
  sliders, a priority-grouping toggle, a card color mode (project-tag chip vs.
  a fully color-coded card), an optional due-date glow, and an optional
  natural-language due-date phrasing — all switchable and persisted from the
  Settings page.
- **Subtasks, dependencies, notes** — every task supports a markdown notes
  body and a `depends_on` list of other task IDs.

### Planned

Time tracking, Pomodoro/focus mode, idle detection, recurring tasks, habit
tracking, analytics dashboards, plugin system, calendar sync, and
import/export. See the in-app roadmap for current priorities.

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
| `@status`          | Sets the status (exact match against configured statuses) | `@do` |
| `due <phrase>` / `due:<token>` | Sets the due date    | `due next friday`, `due:tomorrow` |
| `sch <phrase>` / `sch:<token>` | Sets the scheduled date | `sch next monday` |

Anything left over becomes the task title. Typing a bare `#`, `+`, `!`, or
`@` with nothing after it lists every available tag, project, priority, or
status to pick from (tag listing is disabled once you have more than 10
distinct tags — spell it out instead).

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
navigation. A "tasks behind this week" pill appears next to the week range
whenever there are unfinished tasks scheduled or due before the visible week.

Each day column lists a small bar for every task that's **scheduled** for
that day or **due** that day (a task with both gets one bar per date),
tinted with its project's color and marked with a scheduled/due icon. **Drag
a bar to a different day** to reschedule it — a scheduled-type bar updates
the task's `scheduled` date, a due-type bar updates its `due` date. Clicking
a bar opens a popover with the task's details (project/priority/status chips
and both dates) and an **Edit** button that opens the full task editor; close
the popover by clicking elsewhere, pressing Escape, or its "×" button.

An optional leading **"Previous"** column lists unfinished tasks scheduled or
due before the visible week (read-only, not draggable), and only appears when
there's actually something to show. Toggle it from Settings → Display ("Week
view" section) for a global default, or override it per-project from that
project's settings page (Use global default / Always show / Always hide).

A finished (done/cancelled) task with both a scheduled and due date in the
visible week normally gets two bars, one per date. Turn on "Deduplicate
completed/cancelled tasks" (Settings → Display) to keep only one — pick
whether the due-date or scheduled-date bar wins when both exist.

### Calendar view

Switch to "Calendar" (alongside "Board" and "Week") for a full month grid —
the same look and interaction model as the [Week view](#week-view): bars are
tinted by project color, draggable to reschedule, clickable for a details
popover, and follow the same done/cancelled styling, priority sort, and
dedup setting. Weeks start per [Display settings](#display-settings)'s
"Week starts on" control. Days outside the current month (the leading/
trailing days needed to fill complete weeks) are shown dimmed but remain
fully interactive. A day's row grows taller to fit all of its bars rather
than truncating or scrolling within the cell — the page itself scrolls if
a month runs tall. Navigate with prev/next/Today, same as the Week view.

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
| Card color mode | Project tag / Color code | "Project tag" shows a colored project chip on each card (default). "Color code" hides the chip and tints the whole card/bar in the project's color instead, at the lightness configured under "Card & bar appearance" (see below) — text color always adjusts automatically (real WCAG contrast against the resolved background, not a fixed assumption) to stay readable regardless of the source color or chosen lightness. |
| Due-date glow | on/off | Adds a soft red halo around cards/bars that are overdue or due today. |
| Natural language due dates | on/off | Shows due dates as relative phrases ("due this Wednesday", "due next Saturday") instead of `due YYYY-MM-DD`, falling back to the absolute date outside a ~2-week window. |
| Deduplicate completed/cancelled tasks | on/off, + keep due/scheduled | In the Week view, keeps only one bar (instead of two) for a finished task that has both a scheduled and due date in the visible week. |

All settings above apply instantly and are saved in `localStorage`. Two
exceptions are saved to the backend `settings.json` instead, each with an
optional per-project override, since the per-project value needs to be
readable by the Rust backend too: the "Previous" week-view column toggle
(see [Week view](#week-view)), and Kanban card / week-bar lightness — see
"Card & bar appearance" below.

#### Card & bar appearance

Also on the Settings page: two sliders (0–100%) control how bright/dark the
"Color code" card mode's background is — one for Kanban cards, one shared by
week-view and calendar-view bars (bars default darker, by design). A live
swatch previews the result, including the automatically-adjusted text color,
before you save. Any project can override either value independently from
its own settings page (a checkbox reveals that project's own slider); leaving
it unchecked inherits the global default.

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
