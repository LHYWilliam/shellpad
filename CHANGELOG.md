# Changelog

## v0.3.2 (2026-06-18) — Undo delete & output search

### Features
- **Undo Delete** — Ctrl+Z restores deleted groups/sets (multi-level LIFO stack, status bar hint)
- **Output Search** — `/` in execution screen to search output lines, real-time substring highlighting, ↑/↓ jump between matches

### Fixes
- Search match navigation: smart scrolling avoids unnecessary viewport jumps
- Enter exits search without resetting scroll position
- Perf: search match highlighting computes once per frame instead of per-command

### Refactor
- `items_offset_for_command` documented off-by-one bug at defer boundaries (test only)

## v0.3.1 (2026-06-18) — Scroll fixes & code quality

### Fixes
- Auto-scroll follows output tail, not command header
- Gauge progress bar: clamp to `[0, 100]` to prevent Ctrl+C overflow panic
- Ctrl+C double-count on defer commands fixed
- Follow→Free scroll transition preserves visual position
- ←/→ now works with single-command sets

### Refactor
- `ScrollMode` enum replaces auto_scroll/focus_index/scroll_offset fields
- `ExecutionThread` struct bundles rx+handle
- `MainScreenMode` enum replaces search/rename bools
- `VariableOverlay` struct replaces active/inputs/names/focus/gi/si fields
- `EditingState` + `ListEditor` replace 10 editing fields in DetailScreen
- `scroll_by`, `browse_command`, `reorder_focused` and 6 other methods extracted
  to eliminate handler duplication (~117 lines saved)
- `execute_single_cmd` helper eliminates blocking executor duplication
- `bordered_block_primary` API for self-documenting primary panels

### Style
- `flat_map` unification for Unicode case-folding in fuzzy search
- `let-else` for early-return guard patterns in CLI
- `impl Default` for ToastManager
- `const fn` for ShellType::executable()

## v0.3.0 (2026-06-18) — Defer commands & execution redesign

### Features
- **Defer commands**: independent `defer_commands` list on CommandSet, runs after all
  normal commands regardless of success/failure/interrupt. Defer phase is unkillable.
- **Execution flow redesign**: `s` (skip current + pause), `n` (continue from pause),
  `Ctrl+C` (abort all normals, run defers). Dual-signal executor (`kill_signal` + `skip_signal`).
- **Fuzzy search**: character-level sequential matching (`"dpl"` matches `"Deploy"`).
  Search scope extended to command text, not just set names.
- **Import/export**: `shellpad export --id/--all`, `shellpad import --input`.
  Pipe-friendly stdin/stdout support. Unified `AppData` JSON container format.
- **Distinct defer separator**: `═` double-line boundary between normal and defer
  commands in execution output. Highlighted Output block border.

### Internal
- Extract `run_phase` closure in executor, parameterized with `check_signals`
- `Finished` event: add `skipped: bool` field for signal-kill distinction
- Gauge progress bar: clamp to `[0, 100]` to prevent overflow panic
- `mark_remaining_as_skipped` excludes defer commands to avoid double-counting
- `BackToMain` handler gated on `screen.completed` for defense-in-depth

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
