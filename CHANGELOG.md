# Changelog

## v0.2.6 (2026-06-17) — Exit codes & visual polish

### Features
- Display exit code on command failure (e.g. `[127] $ unknown-cmd`)
- Replace emoji with geometric symbols: `○` `▶` `✓` `✕` `~` `■` `→`
- Add screenshots to README (main + execution)
- CI: run test.yml gate before publish; extract changelog for release body

## v0.2.5 (2026-06-17) — Workflow test

## v0.2.4 (2026-06-17) — Free scroll & ring buffer

### Features
- Execution screen free scrolling: ↑/↓, PageUp/PageDown, j/k
- Output ring buffer: auto-truncation at 10,000 lines per command
- Toast notification when output is truncated
- Scrollbar now tracks line position, not command position

### Internal
- Fix items_offset_for_command: account for truncation marker and trailing separator
- Clamp scroll_offset to prevent overscrolling past content

## v0.2.3 (2026-06-17) — Fix

### Internal
- Switch crates.io badge from shields.io to badgen.net

## v0.2.2 (2026-06-17) — Housekeeping

### Internal
- Fix GitHub repository URL (LHYLiuWilliam → LHYWilliam)
- Add publish workflow for crates.io Trusted Publisher

## v0.2.1 (2026-06-17) — CI Polish

### Internal
- Fix clippy warnings: too_many_arguments, needless_return, manual_range_contains
- Run cargo fmt across all test code
- Remove CLI argument count warnings
- Add crates.io badge to README
- Add `--json` search example to CLI docs

## v0.2.0 (2026-06-18) — Initial Public Release

### Features
- TUI with four-mode navigation: Main, Detail, Execution, Help
- Command set CRUD: groups, sets, variables, commands
- Inline editing for Name, WorkDir, variables, commands
- Three-layer Tab navigation (Properties / Variables / Commands)
- Option picker panel for Group, Shell, ExecMode selection
- Arrow-button delete confirmation dialog
- Ctrl+Up/Down reordering for all four list types
- Per-command-set working directory support
- Search with match highlighting and standalone Search block
- Toast notification stack with bordered blocks, bottom-right
- CLI mode: `shellpad run`, `shellpad search --json`
- 231 tests across handler, executor, widget, integration, CLI

### Visual
- Unified highlight system: selected (translucent blue) + editing (translucent green)
- Properties block split into Text Fields / Options sections
- Vertical divider and side-panel picker layout
- Arrow decorators ◄ ► on focused Options
- Seven-row fixed picker layout with dim peek rows

### Internal
- Atomic JSON persistence with EXDEV fallback
- Background-threaded execution with mpsc streaming
- Centralized Theme struct (Catppuccin Mocha palette)
- Render abstraction extraction (render_editable_field, render_edit_cursor)
