# 项目文档补全 — 实施计划

> **For agentic workers:** 文档编写任务，不需要编译检查。完成后运行 `cargo doc --no-deps` 确认格式正确。

**Goal:** 创建 README.md、给 7 个模块加 `//!` 文档、更新 CLAUDE.md。

---

### Task 1: 创建 `README.md`

- [ ] **Step 1: 写入 README.md**

```markdown
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
```

### Task 2: 添加 7 个模块的 `//!` 文档

- [ ] **Step 2.1: `src/action.rs`** — 文件顶部加：

```rust
//! Unified action enum for screen-to-app communication.
//!
//! All screens return [`AppAction`] variants from their `handle_key()` methods.
//! The `app::handler` module processes these centrally in `App::handle_action()`.
```

- [ ] **Step 2.2: `src/app.rs`** — 文件顶部加：

```rust
//! Application state machine and event loop.
//!
//! [`App`] holds all mutable state (`data`, `mode`, screen states) and runs the
//! main event loop. Sub-modules handle rendering (`app::render`), action
//! dispatch (`app::handler`), toast notifications (`app::toast`), and execution
//! lifecycle (`app::execution`).
```

- [ ] **Step 2.3: `src/config.rs`** — 文件顶部加：

```rust
//! XDG configuration paths and terminal size constraints.
//!
//! Data is stored at `~/.config/launcher/sets.json` (Linux).
//! Minimum terminal dimensions are 80×24.
```

- [ ] **Step 2.4: `src/error.rs`** — 文件顶部加：

```rust
//! Structured error types for the application.
//!
//! [`StorageError`] covers data-load/corruption/save failures.
//! [`CliError`] covers argument parsing and resolution errors,
//! and can convert from [`StorageError`] via `#[from]`.
```

- [ ] **Step 2.5: `src/executor/mod.rs`** — 文件顶部加：

```rust
//! Command execution engine with two entry points.
//!
//! - [`execute_set`] runs commands asynchronously on a background thread,
//!   streaming output via `mpsc` channel (used by the TUI).
//! - [`execute_set_blocking`] runs commands synchronously with inherited
//!   stdio (used by CLI mode).
```

- [ ] **Step 2.6: `src/storage.rs`** — 文件顶部加：

```rust
//! Atomic JSON persistence for command set data.
//!
//! Writes to a `.tmp` file first, then `fsync`s and `rename`s to the target
//! path for crash safety. Falls back to `copy` + `remove` on cross-filesystem
//! moves (EXDEV). Corrupted files are backed up to `sets.json.bak`.
```

- [ ] **Step 2.7: `src/ui/mod.rs`** — 文件顶部加：

```rust
//! Terminal UI components and screen implementations.
//!
//! - [`render`] — Pure rendering helpers (blocks, scrollbars, status bars)
//! - [`widget`] — Reusable widgets (TextInput, ScrollableList, InlineEdit)
//! - [`*_screen`] — Full-screen state machines with render + key handling
//! - [`theme`] — Color palettes and style helpers
//! - [`notification`] — Toast notification types
```

### Task 3: 更新 CLAUDE.md

- [ ] **Step 3: 重写 CLAUDE.md 的架构部分**

读取当前 `CLAUDE.md`，将其中的架构描述（Key modules 表格、Data flow 章节、模式描述）更新为重构后的模块结构。保留 Build & Test 和 Known Gotchas 等不变内容。

需要更新的部分：
- `Key modules` 表 → 反映重构后的文件分布
- `Mode-based navigation` → 保持正确
- `Data flow` → 反映 Action 系统的统一（AppAction → handle_action）
- 测试数量 → 128 而非 30

### Task 4: 验证

- [ ] **Step 4.1: 确认 README 渲染**

```bash
# 检查 markdown 格式
grep -c '^## ' README.md
# 预期输出 > 0
```

- [ ] **Step 4.2: 确认 rustdoc 编译**

```bash
cargo doc --no-deps 2>&1 | tail -5
```
预期输出：无错误。

- [ ] **Step 4.3: 提交**

```bash
git add README.md src/action.rs src/app.rs src/config.rs src/error.rs src/executor/mod.rs src/storage.rs src/ui/mod.rs CLAUDE.md
git commit -m "docs: 添加 README、7 个模块 rustdoc、更新 CLAUDE.md"
```
