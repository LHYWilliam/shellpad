# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test

```bash
cargo build              # Build the project
cargo run                # Run the TUI app (requires a real terminal)
cargo test               # Run all tests (133 unit tests + 5 integration tests)
cargo check              # Fast compilation check
cargo clippy             # Lint the project
```

## Architecture Overview

Launcher is a Ratatui-based TUI for managing and executing collections of shell commands ("command sets").

### Mode-based navigation

The app uses a 4-mode state machine, one screen visible at a time:

- **Main** (`mode.rs:AppMode::Main`) — dual-panel list: groups (left) + command sets (right). Search (`/`), group CRUD (`g`/`D`/`R`), set CRUD (`n`/`d`/`e`/`Enter`).
- **Detail** (`AppMode::Detail`) — full-screen form for editing a command set: name, group, shell, execution mode, variables, commands. Focus regions navigated by Tab.
- **Execution** (`AppMode::Execution`) — full-screen real-time command output streamed from a background thread via `mpsc` channel.
- **Help** (`AppMode::Help`) — overlay showing keyboard shortcuts.

### Key modules

| File | Responsibility |
|------|---------------|
| `src/mode.rs` | `AppMode` enum defining the 4 application modes |
| `src/action.rs` | Unified `AppAction` enum returned by all screen `handle_key()` methods |
| `src/app.rs` + `src/app/` | App state machine, event loop (100ms tick), mode dispatch; sub-modules: `handler.rs` (action dispatch), `render.rs` (main frame render), `execution.rs` (background execution lifecycle), `toast.rs` (notification manager) |
| `src/models.rs` + `src/models/` | Data model structs: `Group`, `CommandSet`, `Command`, `Variable`, `ShellType`, `ExecMode`, `AppData` (`models.rs`); query/filter helpers (`models/queries.rs`); type aliases (`models/types.rs`). All serde-serialized. |
| `src/storage.rs` | JSON persistence at `~/.config/launcher/sets.json`. Atomic save: write `.tmp` → `fsync` → `rename`. EXDEV fallback to copy+remove. |
| `src/executor/` | Background execution: `mod.rs` re-exports, `async_executor.rs` (TUI mode, mpsc streaming), `blocking.rs` (CLI mode, synchronous), `events.rs` (event types) |
| `src/cli.rs` | CLI argument parsing with Clap (`run`, `search` subcommands) |
| `src/error.rs` | Structured error types: `StorageError` (IO/corruption), `CliError` (parsing/resolution) |
| `src/config.rs` | XDG config path, minimum terminal dimensions |
| `src/tui.rs` | Terminal init/restore (crossterm raw mode + alternate screen) |
| `src/ui/` | Screen implementations: `main_screen/` (3 sub-files: handler, render, search), `detail_screen/` (handler, render), `execution_screen/` (events, render), `help_screen.rs`, `variable_screen.rs`, `detail_editor.rs` |
| `src/ui/widget/` | Shared widgets: `TextInput`, `ScrollableList`, `InlineEdit` |
| `src/ui/render.rs` | Pure rendering helpers (blocks, scrollbars, status bars) |
| `src/ui/theme.rs` | Color palette definitions |

### Data flow

```
User input → app.rs:handle_key() → screen.handle_key() → AppAction
  → app/handler.rs:handle_action() → mutate self.data → auto_save()
  → frame redraw → app/render.rs → screen.render()
```

Execution uses a separate thread:
```
app.rs:do_execute_with()
  → executor::execute_set() on new thread
  → events via mpsc::channel (ExecutionEvent)
  → app.rs event loop polls rx each tick
  → execution_screen::events::process_events() updates CmdStates
  → kill_signal: Arc<AtomicBool> aborts running commands
```

## Rust Edition 2024 Notes

- `std::env::set_var`/`remove_var` are `unsafe` — use `unsafe {}` blocks or restructure to avoid env manipulation
- `directories::ProjectDirs` caches internally — tests cannot override via `XDG_CONFIG_HOME`; use path-accepting functions instead

## Known Gotchas

- `ExitStatus::default()` has exit code 0 (success) — use explicit `success: bool` when masking spawn failures
- `fs::rename(tmp, path)` fails with `EXDEV` cross-filesystem — handle by falling back to `copy` + `remove`
- `TextInput::render` with `Borders::ALL` needs ≥3 lines of vertical space — use plain text rendering in 1-line areas
- App panics with "No such device or address" when stdout is not a terminal — expected for TUI apps in non-TTY environments
- `render_stateful_widget(widget, area, &mut state)` takes 3 args, `render_widget(widget, area)` takes 2
- `ScrollableList::update_offset(vis_height)` must be called every render frame for proper list scrolling
- `Block` with borders: create it, call `block.inner(area)`, then render it with `frame.render_widget(&block, area)` — block is not auto-rendered

## Testing

- Storage tests use `with_temp_dir` pattern: create temp dir → run closure → clean up
- Executor tests pass `Arc::new(AtomicBool::new(false))` as kill_signal (never triggered in tests)

### Key design decisions

- **No async runtime** — execution thread uses `std::thread` + `mpsc`. Event loop polls with 100ms tick.
- **Screen Action dispatch** — each screen returns an action enum; `app.rs` centralizes mode transitions.
- **Panel focus** — `MainScreenState.active_panel: Panel` tracks left/right focus, prevents accidental cross-panel operations.
- **Atomic persistence** — write to `.tmp` → `sync_all()` → `rename()`. Parent dir `sync_all()` after rename. `Drop` impl for shutdown save.
- **Unicode safety** — cursor movement uses `floor_char_boundary` and `unicode-width` crate for correct CJK/emoji handling.
