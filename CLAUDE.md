# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test

```bash
cargo build              # Build the project
cargo run                # Run the TUI app (requires a real terminal)
cargo test               # Run all tests (162 tests)
cargo check              # Fast compilation check
cargo clippy             # Lint the project
```

## Architecture Overview

Launcher is a Ratatui-based TUI for managing and executing collections of shell commands ("command sets").

### Mode-based navigation

The app uses a 4-mode state machine, one screen visible at a time:

- **Main** (`AppMode::Main`) — dual-panel list: groups (left) + command sets (right). Search (`/`), group CRUD (`g`/`D`/`R`), set CRUD (`n`/`d`/`e`/`Enter`).
- **Detail** (`AppMode::Detail`) — full-screen form for editing a command set: name, group, shell, execution mode, variables, commands. Focus regions navigated by Tab.
- **Execution** (`AppMode::Execution`) — full-screen real-time command output streamed from a background thread via `mpsc` channel.
- **Help** (`AppMode::Help`) — overlay showing keyboard shortcuts.

### Key modules

| File | Responsibility |
|------|---------------|
| `src/mode.rs` | `AppMode` enum defining the 4 application modes |
| `src/action.rs` | Unified `AppAction` enum returned by all screen `handle_key()` methods |
| `src/app.rs` + `src/app/` | App state machine with `ExecutionState` enum (Idle/Running), event loop (100ms tick), mode dispatch; sub-modules: `handler.rs` (action dispatch + 24 tests), `render.rs` (main frame render), `execution.rs` (thread/kill_signal management), `toast.rs` (notification manager) |
| `src/models/` | Data model: `Group`, `CommandSet`, `Command`, `Variable`, `ShellType`, `ExecMode`, `AppData` (types.rs); query/filter helpers (queries.rs). All serde-serialized. |
| `src/storage.rs` | JSON persistence at `~/.config/launcher/sets.json`. Atomic save: write `.tmp` → `fsync` → `rename`. EXDEV fallback to copy+remove. |
| `src/error.rs` | All error types via thiserror: `StorageError` (IO/corruption), `CliError` (parsing/resolution), `ExecuteError` (spawn/fail) |
| `src/executor/` | Background execution: `mod.rs` (re-exports + `substitute_variables_core`), `async_executor.rs` (TUI mode, mpsc streaming), `blocking.rs` (CLI mode, synchronous), `events.rs` (event types) |
| `src/cli.rs` | CLI argument parsing with Clap (`run`, `search` subcommands) |
| `src/config.rs` | XDG config path, minimum terminal dimensions |
| `src/tui.rs` | Terminal init/restore (crossterm raw mode + alternate screen) |
| `src/ui/` | Screens: `main_screen/` (handler, render, search), `detail_screen/` (handler, render, editor), `execution_screen/` (events, render), `help_screen.rs`, `variable_screen.rs` |
| `src/ui/widget/` | Shared widgets: `TextInput`, `ScrollableList`, `InlineEdit` |
| `src/ui/render.rs` | Pure rendering helpers (bordered_block_zone, scrollbars, status bars, fill_row) |
| `src/ui/theme.rs` | Central Theme struct (dark/simple palettes, style helpers) |

### Data flow

```
User input → app.rs:handle_key() → screen.handle_key() → AppAction
  → app/handler.rs:handle_action() → mutate self.data → auto_save()
  → frame redraw → app/render.rs → screen.render()
```

Execution uses a separate thread:
```
handler:ExecuteSet/ConfirmVariables → do_execute() → do_execute_with()
  → ExecutionState::Running { screen, manager, pending_set } set
  → executor::execute_set() on new thread
  → events via mpsc::channel (ExecutionEvent)
  → app.rs event loop polls rx each tick (always drained, even after mode switch)
  → screen.process_events() updates CmdStates
  → kill_signal: Arc<AtomicBool> aborts running commands
```

### Key design decisions

- **ExecutionState enum** — `Idle { pending_set }` / `Running { screen, manager, pending_set }` replaces three separate fields, compiler-enforced consistency.
- **No async runtime** — execution thread uses `std::thread` + `mpsc`. Event loop polls with `TICK_RATE_MS = 100`.
- **Unified AppAction** — all screens return the same action enum; `app/handler.rs` centralizes mode transitions and data mutations.
- **Atomic persistence** — write to `.tmp` → `sync_all()` → `rename()`. Parent dir `sync_all()` after rename. `Drop` impl for shutdown save.
- **Unicode safety** — cursor movement uses `floor_char_boundary` and `unicode-width` crate for correct CJK/emoji handling.
- **Variable substitution** — `substitute_variables_core(template, vars)` generic function, three thin wrappers in async/blocking executors.
- **Panel focus** — `MainScreenState.active_panel: Panel` tracks left/right focus, prevents accidental cross-panel operations.

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

## Coding Conventions

These conventions are enforced by code review, not by tooling. When adding new
code, follow the patterns below. When refactoring, do not deviate from these
without updating this section.

### Handler Functions

- **Default return** — every `match key.code { ... }` block ends with
  `_ => AppAction::None`. No key event should fall through silently.

- **Return style** — use expression style in match arms, not `return`:
  ```rust
  // Prefer:
  KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
      AppAction::Help
  }
  KeyCode::Char('h') => AppAction::None,
  // Over:
  KeyCode::Char('h') => {
      if key.modifiers.contains(CONTROL) { return AppAction::Help; }
      AppAction::None
  }
  ```
  Exception: `return match ...` is acceptable for inline-editing guard clauses
  (rename mode, search mode, variable/command editing) that must exit early.

- **Modifier key checks** — use match guard, not body `if`:
  `KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) =>`

- **Inline editing modes** — `rename_mode`, `search_mode`, `editing_name`,
  `var_edit.editing`, `cmd_edit.editing` all follow the same pattern:
  1. Guard clause at the top of `handle_key`: check flag, `return` early
     with an inner match.
  2. Enter mode by setting `flag = true` on a trigger key.
  3. Inside the mode: `Esc` sets `flag = false` + returns `None`;
     `Enter` commits; other keys delegate to `handle_text_input`.

- **Esc handling** — two-tier convention:
  - Inline editing context: set the edit flag to `false`, return
    `AppAction::None`.
  - Top-level context: return a cancel action (`AppAction::CancelEdit`,
    `AppAction::CancelVariables`).

- **`handle_key` signatures** — `MainScreenState::handle_key(&mut self, key:
  KeyEvent, data: &AppData)` takes `&AppData` because it queries live data.
  All other screens take only `&mut self, key: KeyEvent`. Do not refactor
  to unify signatures — the difference is intentional (ownership/lifetime).

### Render Functions

- **Block construction** — always use `bordered_block_zone(frame, area,
  theme, " Title ", focused)` from `crate::ui::render`. Never construct
  `Block::default().borders(Borders::ALL)` directly in screen code.

- **Block titles** — always include surrounding spaces: `" Groups "`,
  `" Properties "`, `" Output "`. Dynamic titles use
  `format!(" Variables ({}) ", count)`.

- **List items** — use `styled_list_item(label, style, width)` from
  `src/ui/render.rs` instead of manual `fill_row` + `ListItem::new`.
  Exception: items with mixed styles (e.g. search highlighting with
  multiple `Span` colors) may construct `Vec<Span>` directly.

- **Widget choice** — use `List` + `ListItem` for selectable/scrollable
  collections (groups, sets, variables, commands). Use `Paragraph` for
  single-line labels, headers, and status text.

- **Empty-state hints** — when a list or body area is empty, append
  `empty_hint(theme, " (hint text) ")`. Import from `crate::ui::render`.

- **Separator character** — use `"─"` (U+2500 BOX DRAWINGS LIGHT HORIZONTAL)
  for all repeat-based separators. No `"╌"`, `"-"`, or `"━"`.

- **Status bar** — call `render_status_bar(frame, area, theme, text)` from
  `crate::ui::render`. Do not inline status bar rendering.

- **Scrollbar** — every scrollable list must call `render_scrollbar(...)`.
  Use `list_scrollbar_areas(inner)` to split the container before rendering.

### Tests

- **Shared helpers** — `make_key(KeyCode) -> KeyEvent` and
  `make_app() -> App` live in `src/test_utils.rs` (`#[cfg(test)]`).
  Do not redefine these; import: `use crate::test_utils::make_key;`

- **Import style** — use explicit imports (`use super::TypeName`,
  `use crate::module::Type`) in test modules. Avoid `use super::*`
  — it obscures where names come from. Import only what the tests use.

- **Integration tests** (`src/integration_tests.rs`) use `crate::` paths.
  No `super` available — this is correct.

- **Module-specific helpers** (e.g. `make_data()`, `make_state()`) stay in
  their module's test block. Only extract to `test_utils.rs` when shared
  across 3+ modules.

- **Naming** — `test_<descriptive_snake_case>`. Handler tests that verify
  an action variant use `assert!(matches!(action, AppAction::Variant(...)))`.

- **Key events** — use `make_key(KeyCode::Enter)`. For modifier-key tests,
  inline is fine: `KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL)`.

### Data Mutations

- **Auto-save** — every action that mutates `self.data` (create, update,
  delete of groups/sets/variables/commands) MUST call `self.auto_save()`
  immediately after the mutation and before the toast. No exceptions.

- **Toast severity** — `ToastSeverity::Info` for create/update/delete
  confirmations. `ToastSeverity::Success` only for execution-completed
  summaries. `ToastSeverity::Error` for failures.

- **Toast order** — `self.auto_save()` before `self.toasts.add(...)`.
  If `auto_save()` fails it will add its own error toast.

- **List clamping after deletion** — after any `remove()` that shrinks a
  list tracked by a `ScrollableList`, call `list.clamp_selected(new_len)`.
  This applies to: sets, groups, variables, commands.

- **Detail screen cleanup** — SaveSet and CancelEdit both follow:
  `self.detail_screen = None` + `self.mode = AppMode::Main`. SaveSet
  copies changes into `self.data` first; CancelEdit discards.

- **Mode transitions** — all transitions happen in `handle_action` via
  `self.mode = AppMode::X`. The `prev_mode` field is used exclusively
  for Help screen overlay restoration.

### Error Handling

- **Channel send failures** in `executor/async_executor.rs` — use
  `if tx.send(...).is_err() { return; }`. Do not use `let _ = tx.send(...);`.
  Applies to all event types: Starting, StderrLine, StdoutLine, Finished,
  CompletedAll.

- **Error output** — use `eprintln!("{e}")`. Error types implement
  `Display` via `thiserror` and already produce descriptive messages.
  Do not add a redundant `"Error: "` prefix.

- **UI error paths** — do not propagate `Result` through handler or render
  functions. Handle errors locally: `unwrap_or_else(|| fallback)` for
  initialization, `eprintln!` for startup/background failures. This is a
  design decision — UI paths should not fail on errors.

- **Error types** — all errors live in `src/error.rs` via `thiserror`.
  Follow existing patterns for new variants. Do not refactor
  tuple-variant vs. struct-variant style.

### Module & Type Conventions

- **Sub-module visibility** — screen sub-modules under `app/` and `ui/`
  use `pub(crate) mod`. Top-level modules in `lib.rs` use `pub mod`.

- **Import order** — `crate::` imports first, then external crates
  (`ratatui`, `crossterm`, etc.), then `super::` imports.

- **Derive macros** — domain types (`models/types.rs`):
  `Debug, Clone, Serialize, Deserialize, PartialEq, Eq`. Enums with
  `Copy` semantics: add `Copy` (e.g. `Panel`, `DetailFocus`, `CmdStatus`,
  `AppMode`). Widget structs: `Clone` only. Error types: `Debug, Error`
  (thiserror).

- **Doc comments** — `///` for public API documentation; `//` for internal
  implementation notes and section dividers (`// ---- Section ----`).

## Testing

- 165 tests total: executor (20), handler (24), CLI (12), models (14), storage (5), widgets (13), editor (4), integration (5), screen handlers + config
- Storage tests use `with_temp_dir` pattern: create temp dir → run closure → clean up
- Executor tests pass `Arc::new(AtomicBool::new(false))` as kill_signal (never triggered in tests)
