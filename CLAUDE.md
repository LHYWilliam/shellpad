# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test

```bash
cargo build              # Build the project
cargo run                # Run the TUI app (requires a real terminal)
cargo test               # Run all tests (30 unit tests covering models/storage/executor)
cargo check              # Fast compilation check
cargo clippy             # Lint the project
```

## Architecture Overview

Launcher is a Ratatui-based TUI for managing and executing collections of shell commands ("command sets").

### Mode-based navigation

The app uses a 4-mode state machine, one screen visible at a time:

- **Main** (`app.rs:AppMode::Main`) — dual-panel list: groups (left) + command sets (right). Search (`/`), group CRUD (`g`/`D`/`R`), set CRUD (`n`/`d`/`e`/`Enter`).
- **Detail** (`AppMode::Detail`) — full-screen form for editing a command set: name, group, shell, execution mode, variables, commands. Focus regions navigated by Tab.
- **Execution** (`AppMode::Execution`) — full-screen real-time command output streamed from a background thread via `mpsc` channel.
- **Help** (`AppMode::Help`) — overlay showing keyboard shortcuts.

### Key modules

| File | Responsibility |
|------|---------------|
| `src/app.rs` | App state machine, event loop (100ms tick), mode dispatch, variable input overlay, execution lifecycle |
| `src/models.rs` | Data model structs: `Group`, `CommandSet`, `Command`, `Variable`, `ShellType`, `ExecMode`, `AppData`. All serde-serialized. |
| `src/storage.rs` | JSON persistence at `~/.config/launcher/sets.json`. Atomic save: write `.tmp` → `fsync` → `rename`. EXDEV fallback to copy+remove. |
| `src/executor.rs` | Background execution thread: spawns shell commands, pipes stdout/stderr, variable substitution (`{{var}}`). Supports `kill_signal` for abort. |
| `src/ui/main_screen.rs` | Main list rendering + keyboard handling. `Panel` enum tracks active panel (Groups vs Sets). |
| `src/ui/detail_screen.rs` | Edit form rendering. `DetailFocus` enum for 6 focusable regions. Supports insert/delete/edit of variables and commands. |
| `src/ui/execution_screen.rs` | Real-time output rendering via event channel polling. |
| `src/ui/components.rs` | Shared widgets: `TextInput`, `ScrollableList`, `ConfirmDialog`. |
| `src/config.rs` | XDG config path, minimum terminal dimensions. |
| `src/tui.rs` | Terminal init/restore (crossterm raw mode + alternate screen). |

### Data flow

```
User input → app.rs:handle_key → screen.handle_key() → Action enum
  → app.rs:on_*_action() → mutate self.data → auto_save()
  → frame redraw → screen.render()
```

Execution uses a separate thread:
```
app.rs:do_execute_with()
  → executor.rs:execute_set() on new thread
  → events via mpsc::channel
  → app.rs event loop polls rx each tick
  → execution_screen.rs:process_events() updates CmdStates
  → kill_signal: Arc<AtomicBool> aborts running commands
```

### Key design decisions

- **No async runtime** — execution thread uses `std::thread` + `mpsc`. Event loop polls with 100ms tick.
- **Screen Action dispatch** — each screen returns an action enum; `app.rs` centralizes mode transitions.
- **Panel focus** — `MainScreenState.active_panel: Panel` tracks left/right focus, prevents accidental cross-panel operations.
- **Atomic persistence** — write to `.tmp` → `sync_all()` → `rename()`. Parent dir `sync_all()` after rename. `Drop` impl for shutdown save.
- **Unicode safety** — cursor movement uses `floor_char_boundary` and `unicode-width` crate for correct CJK/emoji handling.
