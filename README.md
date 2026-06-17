# ShellPad

[![CI](https://github.com/LHYLiuWilliam/launcher/actions/workflows/test.yml/badge.svg)](https://github.com/LHYLiuWilliam/launcher/actions/workflows/test.yml)
![Rust](https://img.shields.io/badge/rust-1.85%2B-blue)
![License](https://img.shields.io/badge/license-MIT-green)

A Ratatui-based TUI for organising and executing collections of shell commands.
Inspired by task runners like `just` and `make`, but interactive.

![screenshot](docs/screenshot.png)

## Features

- **Command Sets** — Group shell commands into named groups and sets, edit inline
- **Dual Execution Modes** — Stop on error or continue on error per command set
- **Variables** — Template substitution with `{{var}}` syntax, configure per-execution
- **Real-time Output** — Stream stdout/stderr with per-command status, auto-scroll, skip
- **Working Directory** — Set a per-command-set working directory, defaults to launcher CWD
- **Search** — Filter command sets across all groups, with match highlighting
- **Reordering** — Ctrl+Up/Down reorder groups, sets, variables, and commands
- **Delete Confirmation** — Modal confirmation dialog with Confirm/Cancel buttons
- **Three-layer Tab Navigation** — Tab cycles Properties → Variables → Commands, ↑/↓ selects within region
- **Option Picker** — Browse available Group/Shell/ExecMode choices in a side panel
- **Atomic Persistence** — Crash-safe JSON save at `~/.config/launcher/sets.json`
- **CLI Mode** — Execute command sets directly from the terminal with variable overrides
- **228 Tests** — Comprehensive unit, handler, and integration test coverage

## Installation

```bash
# From source
git clone https://github.com/LHYLiuWilliam/launcher
cd launcher
cargo install --path .
```

The binary is `launcher`. It requires a terminal ≥ 80×24.

## Usage

### TUI mode

```bash
launcher
```

**Main Screen:**

| Key | Action |
|-----|--------|
| `↑/↓` / `j/k` | Navigate list |
| `←/→` | Switch between Groups / Sets panel |
| `Ctrl+↑/↓` | Reorder group or set |
| `Enter` | Execute selected command set |
| `e` | Edit selected command set |
| `n` | New command set |
| `d` | Delete (with confirmation dialog) |
| `D` | Delete group (with confirmation dialog) |
| `g` | New group |
| `R` | Rename group |
| `/` | Search command sets |
| `q` | Quit |
| `?` | Help overlay |

**Detail/Edit Screen:**

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Cycle between Properties / Variables / Commands |
| `↑/↓` | Within Properties: cycle fields. Within lists: navigate items |
| `←/→` | Change group, shell, or execution mode |
| `Ctrl+↑/↓` | Reorder variable or command |
| `Enter` | Edit focused field / item |
| `a` | Add new variable or command |
| `d` | Delete (with confirmation dialog) |
| `Ctrl+S` | Save and return to main screen |
| `Esc` | Cancel and return to main screen |

**Execution Screen:**

| Key | Action |
|-----|--------|
| `←/→` | Browse output of other commands |
| `z` | Toggle auto-scroll / follow current |
| `s` | Skip current command |
| `Ctrl+C` | Interrupt running command |
| `n` | Continue from next skipped command |
| `r` | Re-execute all from beginning |
| `q` | Back to main |
| `?` | Help overlay |

### CLI mode

```bash
# Execute a command set by UUID
launcher run --id <uuid>

# Execute by group and set name
launcher run --group "Deploy" --set "Prod"

# Use variable defaults (skip prompting)
launcher run --group Deploy --set Prod --var default

# Override variable values
launcher run --group Deploy --set Prod --var host=prod.example.com

# Search command sets
launcher search --set "deploy"

# Search groups
launcher search --group "infra"
```

## Storage

Data is stored at `~/.config/launcher/sets.json`. The file is atomically updated
(write to `.tmp` → `fsync` → `rename`). Corrupted files are backed up to
`sets.json.bak` on read.

## Architecture

```
src/
├── app/                    # App state machine
│   ├── handler.rs          # Action dispatch (50 handler tests)
│   ├── render.rs           # Main frame render
│   ├── execution.rs        # ExecutionManager (thread lifecycle)
│   └── toast.rs            # Toast notifications
├── executor/               # Background thread execution
│   ├── async_executor.rs   # TUI mode, mpsc streaming
│   ├── blocking.rs         # CLI mode, synchronous
│   └── events.rs           # Execution event types
├── ui/                     # Terminal UI
│   ├── main_screen/        # Dual-panel list (groups + sets), search
│   ├── detail_screen/      # Full-screen form editor, option picker
│   ├── execution_screen/   # Real-time command output
│   ├── help_screen.rs      # Keyboard shortcuts overlay
│   ├── variable_screen.rs  # Variable prompt dialog
│   ├── confirm_dialog.rs   # Delete confirmation dialog
│   ├── theme.rs            # Centralised colour palette
│   ├── render.rs           # Shared rendering helpers
│   └── widget/             # Reusable widgets (TextInput, List, etc.)
├── models/                 # Data model (serde-serialised)
├── cli.rs                  # Clap argument parsing
├── storage.rs              # Atomic JSON persistence
└── error.rs                # Error types (thiserror)
```

Data flow:

```
User keypress → screen.handle_key() → AppAction
  → app/handler.rs:handle_action() → mutate self.data
  → auto_save() → frame redraw → screen.render()
```

Execution runs on a background `std::thread` with `mpsc` channel streaming:

```
handler: confirm → do_execute()
  → ExecutionManager::start() → executor::execute_set()
  → spawn shell commands → pipe stdout/stderr
  → send ExecutionEvent via mpsc → event loop polls each tick
  → screen.process_events() updates command states
```

## Development

```bash
cargo build              # Build
cargo run                # Run TUI (requires real terminal)
cargo test               # Run all 228 tests
cargo check              # Fast compilation check
cargo clippy             # Lint
```

## License

MIT
