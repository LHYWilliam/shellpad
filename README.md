# Launcher

A Ratatui-based TUI for managing and executing collections of shell commands.

![screenshot](docs/screenshot.png)

## Features

- **Command Sets** — Organize shell commands into named groups and sets
- **Dual Execution Modes** — Stop on error or continue on error
- **Variables** — Template substitution with `{{var}}` syntax
- **Real-time Output** — Stream command output in the TUI execution screen
- **CLI Mode** — Execute command sets directly from the terminal (`launcher run --id <uuid>`)
- **Search** — Filter command sets by name across all groups
- **Atomic Persistence** — Crash-safe JSON save with `.tmp` → `rename` pattern

## Installation

```bash
git clone <repo-url>
cd launcher
cargo build --release
```

The binary will be at `target/release/launcher`.

## Usage

### TUI mode

Run without arguments to start the interactive TUI:

```bash
launcher
```

**Keyboard shortcuts:**

| Key | Action |
|-----|--------|
| `↑/↓` / `j/k` | Navigate lists |
| `←/→` | Switch panel (Groups / Sets) |
| `Enter` | Execute a command set |
| `e` | Edit a command set |
| `n` | New command set |
| `d` | Delete command set |
| `Shift+D` | Delete group |
| `g` | New group |
| `R` | Rename group |
| `/` | Search command sets |
| `?` / `Ctrl+H` | Help overlay |
| `Tab` | Next focus (edit screen) |
| `Ctrl+S` | Save (edit screen) |
| `Esc` | Cancel / Back |
| `q` | Quit |

### CLI mode

```bash
# Run a command set by UUID
launcher run --id <uuid>

# Run a command set by group and set name
launcher run --group "My Group" --set "My Set"

# Use variable defaults without prompting
launcher run --group G --set S --var default

# Override variables
launcher run --group G --set S --var host=prod

# Search command sets
launcher search --set "deploy"

# Search groups
launcher search --group "dev"
```

## Data

Command sets are stored at `~/.config/launcher/sets.json`. The file is atomically written (write to `.tmp` → `fsync` → `rename`). If the file becomes corrupted, it is automatically backed up to `sets.json.bak`.

## Architecture

See [docs/architecture.md](docs/architecture.md) for a detailed architecture overview.

## Dependencies

- [Ratatui](https://ratatui.rs/) — TUI framework
- [Crossterm](https://github.com/crossterm-rs/crossterm) — Terminal backend
- [Clap](https://docs.rs/clap/) — CLI argument parsing
- [Serde](https://serde.rs/) — JSON serialization
- [UUID](https://docs.rs/uuid/) — Unique identifiers
- [Chrono](https://docs.rs/chrono/) — Timestamps
- [Directories](https://docs.rs/directories/) — XDG config paths
- [unicode-width](https://docs.rs/unicode-width/) — Unicode-safe cursor positioning
