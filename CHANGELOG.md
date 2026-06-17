# Changelog

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
- CLI mode: `launcher run`, `launcher search --json`
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
