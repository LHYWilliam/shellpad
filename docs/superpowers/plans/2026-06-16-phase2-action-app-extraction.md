# Phase 2: `action.rs` + `app/` 目录重构 — 详细实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: 使用 `superpowers:subagent-driven-development` 执行。步骤使用 checkbox (`- [ ]`) 语法。

**Goal:** 创建全局 `AppAction` 枚举，提取 `app/` 子模块（toast/execution/render/handler），将 `app.rs` 从 537 行精简到约 80 行。

**Architecture:** 所有 Screen 的 `handle_key()` 返回 `AppAction` 统一类型。`app.rs` 只保留 struct + `new()` + `run()` + `Drop`。`impl App` 分布在 `app/handler.rs`（动作处理）、`app/render.rs`（渲染）、`app/toast.rs`（Toast 管理）、`app/execution.rs`（执行生命周期）。

**范围:**
- 创建 5 个新文件 (`action.rs`, `app/toast.rs`, `app/execution.rs`, `app/render.rs`, `app/handler.rs`)
- 修改 5 个现有文件 (`app.rs`, plus 4 screens)

**验证:** `cargo check` → `cargo test`(≥60) → `cargo clippy` → `cargo fmt` → commit

---

## 前置知识

### 4 个当前 Action 枚举 → 统一 AppAction

```rust
// 融合 MainScreenAction + DetailScreenAction + ExecutionScreenAction + VariableScreenAction
pub enum AppAction {
    None,
    Quit,
    Help,

    // Main screen
    ExecuteSet(usize, usize),       // (group_index, set_index)
    EditSet(usize, usize),          // (group_index, set_index) — handler resolves data
    NewSet(usize),                  // group_index
    DeleteSet(usize, usize),        // (group_index, set_index)
    NewGroup,
    RenameGroup(usize, String),     // (group_index, new_name)
    DeleteGroup(usize),

    // Detail screen
    SaveSet(CommandSet),
    CancelEdit,
    DeleteVariable(usize),
    DeleteCommand(usize),

    // Execution screen
    KillExec,
    SkipCurrent,
    ContinueFrom(usize),
    ReExec,
    ToggleAutoScroll,
    BackToMain,

    // Variable overlay
    ConfirmVariables,               // Handler reads values from variable_screen.inputs
    CancelVariables,
}
```

### 当前 app.rs 各段代码 → 目标位置映射

| 当前代码段 | 行号范围 | 目标文件 |
|-----------|---------|---------|
| struct App 定义 + new() + run() | 26-99 | `app.rs` 保留 |
| render() | 101-186 | `app/render.rs` |
| handle_key() 分派 | 188-213 | `app/handler.rs` |
| handle_variable_action() | 217-240 | `app/handler.rs` |
| on_main_action() | 244-326 | `app/handler.rs` |
| on_detail_action() | 330-380 | `app/handler.rs` |
| teardown_execution() | 385-398 | `app/execution.rs` |
| on_exec_action() | 403-448 | `app/handler.rs` |
| do_execute() + do_execute_with() | 452-494 | `app/execution.rs` |
| auto_save() | 496-500 | `app/handler.rs` |
| push_toast() + clean_toasts() | 502-510 | `app/toast.rs` |
| Drop impl + kill_execution() | 513-536 | `app.rs` 保留 `Drop`, `kill_execution` 在 `execution.rs` |

### 屏幕 handle_key 签名变更

| 文件 | 当前签名 | 新签名 |
|------|---------|-------|
| `main_screen.rs` | `-> MainScreenAction` | `-> AppAction` |
| `detail_screen.rs` | `-> DetailScreenAction` | `-> AppAction` |
| `execution_screen.rs` | `-> ExecutionScreenAction` | `-> AppAction` |
| `variable_screen.rs` | `-> VariableScreenAction` | `-> AppAction` |

---

### Task 1: 创建 `action.rs`

**文件:** Create: `src/action.rs`

- [ ] **Step 1: 创建 `action.rs`**

```rust
use crate::models::CommandSet;

/// Unified action enum returned by all screens.
/// The `app/handler.rs` handles all variants centrally.
pub enum AppAction {
    None,
    Quit,
    Help,

    // === Main screen ===
    ExecuteSet(usize, usize),         // (group_index, set_index)
    EditSet(usize, usize),            // (group_index, set_index) — handler resolves data
    NewSet(usize),                    // group_index
    DeleteSet(usize, usize),          // (group_index, set_index)
    NewGroup,
    RenameGroup(usize, String),       // (group_index, new_name)
    DeleteGroup(usize),

    // === Detail screen ===
    SaveSet(CommandSet),
    CancelEdit,
    DeleteVariable(usize),
    DeleteCommand(usize),

    // === Execution screen ===
    KillExec,
    SkipCurrent,
    ContinueFrom(usize),
    ReExec,
    ToggleAutoScroll,
    BackToMain,

    // === Variable overlay ===
    ConfirmVariables,                  // handler reads from variable_screen.inputs
    CancelVariables,
}
```

### Task 2: 更新所有 4 个屏幕的 handle_key 签名

- [ ] **Step 2.1: 更新 `main_screen.rs` 的 import + 返回值类型**

当前 import 不需要再 import `MainScreenAction`（将删除该枚举）。

在 `main_screen.rs` 文件头部添加：
```rust
use crate::action::AppAction;
```

将函数签名：
```rust
) -> MainScreenAction {
```
替换为：
```rust
) -> AppAction {
```

在函数体内，将所有返回语句做以下替换：
| 原返回值 | 替换为 |
|---------|--------|
| `MainScreenAction::None` | `AppAction::None` |
| `MainScreenAction::Quit` | `AppAction::Quit` |
| `MainScreenAction::Help` | `AppAction::Help` |
| `MainScreenAction::ExecuteSet(gi, si)` | `AppAction::ExecuteSet(gi, si)` |
| `MainScreenAction::EditSet(gi, si)` | `AppAction::EditSet(gi, si)` |
| `MainScreenAction::NewSet(gi)` | `AppAction::NewSet(gi)` |
| `MainScreenAction::DeleteSet(gi, si)` | `AppAction::DeleteSet(gi, si)` |
| `MainScreenAction::NewGroup` | `AppAction::NewGroup` |
| `MainScreenAction::RenameGroup(gi, name)` | `AppAction::RenameGroup(gi, name)` |
| `MainScreenAction::DeleteGroup(gi)` | `AppAction::DeleteGroup(gi)` |

完成后删除 `MainScreenAction` 枚举定义（第 51-62 行）。

- [ ] **Step 2.2: 更新 `detail_screen.rs` 的 import + 返回值类型**

在文件头部添加：
```rust
use crate::action::AppAction;
```

`DetailScreenAction` 枚举定义在第 25-31 行，函数签名在第 375 行：

```rust
) -> DetailScreenAction {
→ ) -> AppAction {
```

所有 return 替换：
| 原返回值 | 替换为 |
|---------|--------|
| `DetailScreenAction::None` | `AppAction::None` |
| `DetailScreenAction::Save(set)` | `AppAction::SaveSet(set)` |
| `DetailScreenAction::Cancel` | `AppAction::CancelEdit` |
| `DetailScreenAction::DeleteVariable(idx)` | `AppAction::DeleteVariable(idx)` |
| `DetailScreenAction::DeleteCommand(idx)` | `AppAction::DeleteCommand(idx)` |

完成后删除 `DetailScreenAction` 枚举定义。

- [ ] **Step 2.3: 更新 `execution_screen.rs` 的 import + 返回值类型**

在文件头部添加：
```rust
use crate::action::AppAction;
```

`ExecutionScreenAction` 枚举在第 29-35 行。函数签名在第 ~105 行：

```rust
) -> ExecutionScreenAction {
→ ) -> AppAction {
```

所有 return 替换：
| 原返回值 | 替换为 |
|---------|--------|
| `ExecutionScreenAction::BackToMain` | `AppAction::BackToMain` |
| `ExecutionScreenAction::Interrupt` | `AppAction::KillExec` |
| `ExecutionScreenAction::Skip` | `AppAction::SkipCurrent` |
| `ExecutionScreenAction::Continue` | `AppAction::ContinueFrom(..)` — 需要使用当前 continue_from 值填充 |
| `ExecutionScreenAction::Reexecute` | `AppAction::ReExec` |
| `ExecutionScreenAction::None` | `AppAction::None` |
| `ExecutionScreenAction::ToggleAutoScroll` | `AppAction::ToggleAutoScroll` |

关于 `Continue` 变体的特例处理：当前 `ExecutionScreenAction::Continue` 不携带数据——`app.rs` 从 `exec_screen.continue_from` 读取起始位置。在新系统中，`execution_screen/handle_key` 需要在返回 `ContinueFrom` 时带上当前位置。找到 `ExecutionScreenAction::Continue` 的返回点，改为：

```rust
let start = self.continue_from.unwrap_or(0);
AppAction::ContinueFrom(start)
```

完成后删除 `ExecutionScreenAction` 枚举定义。

- [ ] **Step 2.4: 更新 `variable_screen.rs` 的 import + 返回值类型**

在文件头部添加：
```rust
use crate::action::AppAction;
```

`VariableScreenAction` 枚举在第 13-17 行。函数签名在第 ~70 行：

```rust
) -> VariableScreenAction {
→ ) -> AppAction {
```

所有 return 替换：
| 原返回值 | 替换为 |
|---------|--------|
| `VariableScreenAction::Execute { gi, si }` | `AppAction::ConfirmVariables` （不再需要 gi/si，变量屏幕已存储在 self.gi/self.si） |
| `VariableScreenAction::Cancel` | `AppAction::CancelVariables` |
| `VariableScreenAction::None` | `AppAction::None` |

完成后删除 `VariableScreenAction` 枚举定义。

### Task 3: 创建 `app/toast.rs` + `app/execution.rs`

- [ ] **Step 3.1: 创建 `app/toast.rs`**

```rust
use crate::ui::notification::{Toast, ToastSeverity};
use std::time::Duration;

const TOAST_DURATION: Duration = Duration::from_secs(3);

/// Manages toast notifications. Data-only — rendering happens in `app/render.rs`.
pub struct ToastManager {
    pub toasts: Vec<Toast>,
}

impl ToastManager {
    pub fn new() -> Self {
        Self { toasts: Vec::new() }
    }

    pub fn add(&mut self, message: impl Into<String>, severity: ToastSeverity) {
        self.toasts.push(Toast::new(message, severity));
    }

    /// Remove expired toasts.
    pub fn clean_expired(&mut self) {
        self.toasts.retain(|t| t.created_at.elapsed() < TOAST_DURATION);
    }
}
```

- [ ] **Step 3.2: 创建 `app/execution.rs`**

```rust
use crate::executor::{ExecutionEvent, execute_set};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;

/// Manages the lifecycle of a background execution thread.
pub struct ExecutionManager {
    pub rx: Option<mpsc::Receiver<ExecutionEvent>>,
    pub handle: Option<thread::JoinHandle<()>>,
    pub kill_signal: Arc<AtomicBool>,
}

impl ExecutionManager {
    pub fn new() -> Self {
        Self {
            rx: None,
            handle: None,
            kill_signal: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start execution of a command set.
    pub fn start(
        &mut self,
        commands: Vec<crate::models::Command>,
        exec_mode: crate::models::ExecMode,
        variables: Vec<crate::models::Variable>,
        shell_cmd: crate::models::ShellCommand,
        index_offset: usize,
    ) {
        let (tx, rx) = mpsc::channel();
        let handle = execute_set(
            commands,
            exec_mode,
            variables,
            shell_cmd,
            tx,
            Arc::clone(&self.kill_signal),
            index_offset,
        );
        self.rx = Some(rx);
        self.handle = Some(handle);
    }

    /// Kill the running execution thread.
    pub fn kill(&mut self) {
        self.kill_signal.store(true, Ordering::Relaxed);
        self.rx = None;
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        self.kill_signal.store(false, Ordering::Relaxed);
    }
}
```

### Task 4: 创建 `app/render.rs` + `app/handler.rs`

- [ ] **Step 4.1: 创建 `app/render.rs`**

完整渲染逻辑，从 `app.rs` 的 `render()` 方法搬移，略作调整（`self.toasts` → `self.toasts.toasts`）。

```rust
use crate::config::MIN_TERMINAL_HEIGHT;
use crate::config::MIN_TERMINAL_WIDTH;
use crate::mode::AppMode;
use crate::ui::help_screen::draw_help;
use crate::ui::notification::ToastSeverity;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use super::App;

impl App {
    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        if area.width < MIN_TERMINAL_WIDTH || area.height < MIN_TERMINAL_HEIGHT {
            let warning = Paragraph::new(Line::from(format!(
                "Terminal too small: {}x{} (min: {}x{})",
                area.width, area.height, MIN_TERMINAL_WIDTH, MIN_TERMINAL_HEIGHT
            )))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Red));
            frame.render_widget(warning, area);
            return;
        }

        // Split off title bar
        let layout = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]);
        let [title_area, content_area] = layout.areas(area);

        // Render title bar
        let mode_str = match self.mode {
            AppMode::Main => "Main",
            AppMode::Detail => "Edit",
            AppMode::Execution => "Run",
            AppMode::Help => "Help",
        };
        let group_count = self.data.groups.len();
        let set_count: usize = self.data.groups.iter().map(|g| g.sets.len()).sum();
        let title_text = format!(
            " Launcher  |  {}  |  {} groups, {} sets  |  ? Help  q Quit",
            mode_str, group_count, set_count,
        );
        let title_paragraph = Paragraph::new(Line::from(Span::styled(
            title_text,
            Style::default()
                .fg(self.theme.text_secondary)
                .add_modifier(Modifier::DIM),
        )));
        frame.render_widget(title_paragraph, title_area);

        match self.mode {
            AppMode::Main => {
                self.main_screen
                    .render(frame, content_area, &self.data, &self.theme);
            }
            AppMode::Detail => {
                if let Some(ref mut ds) = self.detail_screen {
                    ds.render(frame, content_area, &self.theme);
                }
            }
            AppMode::Execution => {
                if let Some(ref es) = self.exec_screen {
                    es.render(frame, content_area, &self.theme);
                }
            }
            AppMode::Help => {
                self.main_screen
                    .render(frame, content_area, &self.data, &self.theme);
                draw_help(frame, content_area, &self.theme);
            }
        }

        self.variable_screen
            .render(frame, content_area, &self.theme);

        // Render toast notification (centered on title bar)
        if let Some(toast) = self.toasts.toasts.last() {
            let (toast_fg, toast_label) = match toast.severity {
                ToastSeverity::Success => (self.theme.accent_success, " ✓ "),
                ToastSeverity::Error => (self.theme.accent_error, " ✗ "),
                ToastSeverity::Info => (self.theme.accent_info, " ● "),
            };
            let toast_msg = format!("{}{}", toast_label, toast.message);
            let toast_display_width = unicode_width::UnicodeWidthStr::width(toast_msg.as_str());
            let toast_width = (toast_display_width as u16 + 2).min(area.width.saturating_sub(4));
            let x = (area.width.saturating_sub(toast_width)) / 2;
            let toast_area = Rect::new(x, title_area.y, toast_width, 1);
            frame.render_widget(Clear, toast_area);
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    toast_msg,
                    Style::default().fg(toast_fg).add_modifier(Modifier::BOLD),
                ))),
                toast_area,
            );
        }
    }
}
```

- [ ] **Step 4.2: 创建 `app/handler.rs`**

将 `app.rs` 中的 `handle_key()`、`handle_variable_action()`、`on_main_action()`、`on_detail_action()`、`on_exec_action()`、`auto_save()` 统一为 `handle_action()`。

```rust
use crate::action::AppAction;
use crate::mode::AppMode;
use crate::models::CommandSet;
use crate::ui::detail_screen::DetailScreenState;
use crate::ui::main_screen::Panel;
use crate::ui::notification::ToastSeverity;
use crate::storage;

use super::App;

impl App {
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        if self.variable_screen.active {
            let action = self.variable_screen.handle_key(key);
            self.handle_action(action);
            return;
        }
        match self.mode {
            AppMode::Main => {
                let action = self.main_screen.handle_key(key, &self.data);
                self.handle_action(action);
            }
            AppMode::Detail => {
                if let Some(ref mut ds) = self.detail_screen {
                    let action = ds.handle_key(key);
                    self.handle_action(action);
                }
            }
            AppMode::Execution => {
                if let Some(ref mut es) = self.exec_screen {
                    let action = es.handle_key(key);
                    self.handle_action(action);
                }
            }
            AppMode::Help => self.mode = AppMode::Main,
        }
    }

    pub fn handle_action(&mut self, action: AppAction) {
        match action {
            AppAction::None => {}
            AppAction::Quit => self.running = false,
            AppAction::Help => self.mode = AppMode::Help,

            // ---- Main screen ----
            AppAction::ExecuteSet(gi, si) => {
                if gi < self.data.groups.len() && si < self.data.groups[gi].sets.len() {
                    let set = &self.data.groups[gi].sets[si];
                    if !set.variables.is_empty() {
                        self.variable_screen.activate(set, gi, si);
                    } else {
                        self.pending_set = Some((gi, si));
                        self.do_execute();
                    }
                }
            }
            AppAction::EditSet(gi, si) => {
                if gi < self.data.groups.len() && si < self.data.groups[gi].sets.len() {
                    let set = self.data.groups[gi].sets[si].clone();
                    let groups = self.data.groups.clone();
                    self.detail_screen = Some(DetailScreenState::new(set, groups));
                    self.mode = AppMode::Detail;
                }
            }
            AppAction::NewSet(gi) => {
                if gi < self.data.groups.len() {
                    let gid = self.data.groups[gi].id;
                    let set = CommandSet::new("New Command Set".to_string(), gid);
                    let si = (self.main_screen.set_list.selected + 1)
                        .min(self.data.groups[gi].sets.len());
                    self.data.groups[gi].sets.insert(si, set.clone());
                    self.main_screen.set_list.selected = si;
                    self.auto_save();
                    self.toasts.add("Set created", ToastSeverity::Info);
                    let groups = self.data.groups.clone();
                    self.detail_screen = Some(DetailScreenState::new(set, groups));
                    self.mode = AppMode::Detail;
                }
            }
            AppAction::DeleteSet(gi, si) => {
                if gi < self.data.groups.len() && si < self.data.groups[gi].sets.len() {
                    self.data.groups[gi].sets.remove(si);
                    self.main_screen
                        .set_list
                        .clamp_selected(self.data.groups[gi].sets.len());
                    if self.data.groups[gi].sets.is_empty() {
                        self.main_screen.active_panel = Panel::Groups;
                    }
                    self.auto_save();
                    self.toasts.add("Set deleted", ToastSeverity::Info);
                }
            }
            AppAction::NewGroup => {
                let gi = (self.main_screen.group_list.selected + 1).min(self.data.groups.len());
                let n = self.data.groups.len() + 1;
                self.data
                    .groups
                    .insert(gi, crate::models::Group::new(format!("Group {}", n)));
                self.main_screen.group_list.selected = gi;
                self.main_screen.set_list.reset();
                self.auto_save();
                self.toasts.add("Group created", ToastSeverity::Info);
            }
            AppAction::RenameGroup(gi, new_name) => {
                if gi < self.data.groups.len() {
                    self.data.groups[gi].name = new_name;
                    self.auto_save();
                    self.toasts.add("Group renamed", ToastSeverity::Info);
                }
            }
            AppAction::DeleteGroup(gi) => {
                if gi < self.data.groups.len() {
                    self.data.groups.remove(gi);
                    self.main_screen
                        .group_list
                        .clamp_selected(self.data.groups.len());
                    self.main_screen.set_list.reset();
                    if self.data.groups.is_empty() {
                        self.main_screen.group_list.reset();
                        self.main_screen.active_panel = Panel::Groups;
                    }
                    self.auto_save();
                    self.toasts.add("Group deleted", ToastSeverity::Info);
                }
            }

            // ---- Detail screen ----
            AppAction::SaveSet(set) => {
                let sid = set.id;
                for group in &mut self.data.groups {
                    if let Some(existing) = group.sets.iter_mut().find(|s| s.id == sid) {
                        *existing = set;
                        existing.updated_at = chrono::Utc::now();
                        break;
                    }
                }
                self.detail_screen = None;
                self.mode = AppMode::Main;
                self.auto_save();
                self.toasts.add("Command set saved", ToastSeverity::Success);
            }
            AppAction::CancelEdit => {
                self.detail_screen = None;
                self.mode = AppMode::Main;
            }
            AppAction::DeleteVariable(idx) => {
                if let Some(ref mut ds) = self.detail_screen
                    && idx < ds.set.variables.len()
                {
                    ds.set.variables.remove(idx);
                    ds.variable_list.clamp_selected(ds.set.variables.len());
                    if ds.set.variables.is_empty() {
                        ds.focus = crate::ui::detail_screen::DetailFocus::Name;
                    }
                    self.toasts.add("Variable deleted", ToastSeverity::Info);
                }
            }
            AppAction::DeleteCommand(idx) => {
                if let Some(ref mut ds) = self.detail_screen
                    && idx < ds.set.commands.len()
                {
                    ds.set.commands.remove(idx);
                    for (i, c) in ds.set.commands.iter_mut().enumerate() {
                        c.position = i;
                    }
                    ds.command_list.clamp_selected(ds.set.commands.len());
                    if ds.set.commands.is_empty() {
                        ds.focus = crate::ui::detail_screen::DetailFocus::Name;
                    }
                    self.toasts.add("Command deleted", ToastSeverity::Info);
                }
            }

            // ---- Execution screen ----
            AppAction::BackToMain => {
                if let Some(ref es) = self.exec_screen
                    && es.completed
                {
                    let summary = format!(
                        "Done: {}/{}",
                        es.succeeded + es.failed + es.skipped,
                        es.total,
                    );
                    let severity = if es.failed > 0 {
                        ToastSeverity::Error
                    } else if es.skipped > 0 {
                        ToastSeverity::Info
                    } else {
                        ToastSeverity::Success
                    };
                    self.toasts.add(summary, severity);
                }
                self.teardown_execution(false, false);
                self.mode = AppMode::Main;
            }
            AppAction::KillExec | AppAction::SkipCurrent => {
                self.teardown_execution(true, true);
                self.mode = AppMode::Execution;
            }
            AppAction::ContinueFrom(start) => {
                if let Some((gi, si)) = self.pending_set {
                    self.do_execute_with(gi, si, start);
                }
            }
            AppAction::ReExec => {
                self.teardown_execution(false, false);
                if let Some((gi, si)) = self.pending_set {
                    self.do_execute_with(gi, si, 0);
                }
            }
            AppAction::ToggleAutoScroll => {
                // Handled internally by execution_screen
            }

            // ---- Variable overlay ----
            AppAction::ConfirmVariables => {
                let gi = self.variable_screen.gi;
                let si = self.variable_screen.si;
                if gi < self.data.groups.len() && si < self.data.groups[gi].sets.len() {
                    let set = &mut self.data.groups[gi].sets[si];
                    for (i, input) in self.variable_screen.inputs.iter().enumerate() {
                        if i < set.variables.len() {
                            set.variables[i].default_value = input.content.clone();
                        }
                    }
                }
                self.variable_screen = crate::ui::variable_screen::VariableScreenState::new();
                self.auto_save();
                self.pending_set = Some((gi, si));
                self.do_execute();
            }
            AppAction::CancelVariables => {
                self.variable_screen = crate::ui::variable_screen::VariableScreenState::new();
                self.pending_set = None;
            }
        }
    }

    fn auto_save(&mut self) {
        if let Err(e) = storage::save_app_data(&self.data) {
            self.toasts.add(format!("Save failed: {}", e), ToastSeverity::Error);
        }
    }
}
```

### Task 5: 精简 `app.rs`

- [ ] **Step 5: 将 `app.rs` 精简为 struct + new() + run() + Drop**

将之前 537 行的文件缩减到约 80 行。保留：
- struct App 定义（字段调整：`toasts: Vec<Toast>` → `toasts: ToastManager`，`execution_rx/handle/kill_signal` → `execution: ExecutionManager`）
- `App::new()` — 完善初始化
- `App::run()` — 事件循环（加入 `self.execution.rx` 的轮询）
- `impl Drop for App`
- 声明子模块

```rust
use crate::mode::AppMode;
use crate::models::AppData;
use crate::storage;
use crate::tui::TuiTerminal;
use crate::ui::detail_screen::DetailScreenState;
use crate::ui::execution_screen::ExecutionScreenState;
use crate::ui::main_screen::MainScreenState;
use crate::ui::theme::Theme;
use crate::ui::variable_screen::VariableScreenState;
use crate::app::toast::ToastManager;
use crate::app::execution::ExecutionManager;
use crossterm::event::{self, Event, KeyEventKind};
use std::io;
use std::time::Duration;

pub(crate) mod toast;
pub(crate) mod execution;
pub(crate) mod render;
pub(crate) mod handler;

pub struct App {
    pub data: AppData,
    pub mode: AppMode,
    pub running: bool,

    pub main_screen: MainScreenState,
    pub detail_screen: Option<DetailScreenState>,
    pub exec_screen: Option<ExecutionScreenState>,

    pub execution: ExecutionManager,
    pub variable_screen: VariableScreenState,
    pub pending_set: Option<(usize, usize)>,

    pub theme: Theme,
    pub toasts: ToastManager,
}

impl App {
    pub fn new() -> Self {
        let data = storage::load_app_data().unwrap_or_else(|e| {
            eprintln!("{}", e);
            AppData::empty()
        });
        Self {
            main_screen: MainScreenState::new(),
            detail_screen: None,
            exec_screen: None,
            data,
            mode: AppMode::Main,
            running: true,
            execution: ExecutionManager::new(),
            variable_screen: VariableScreenState::new(),
            pending_set: None,
            theme: Theme::default_dark(),
            toasts: ToastManager::new(),
        }
    }

    pub fn run(&mut self, terminal: &mut TuiTerminal) -> io::Result<()> {
        let tick_rate = Duration::from_millis(100);

        while self.running {
            self.toasts.clean_expired();
            terminal.draw(|f| self.render(f))?;

            let timeout = tick_rate;
            if event::poll(timeout)?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                self.handle_key(key);
            }

            // Collect execution events on each tick
            if self.mode == AppMode::Execution
                && let Some(ref rx) = self.execution.rx
                && let Some(ref mut es) = self.exec_screen
            {
                es.process_events(rx);
            }
        }
        Ok(())
    }

    fn do_execute(&mut self) {
        if let Some((gi, si)) = self.pending_set.take() {
            self.do_execute_with(gi, si, 0);
        }
    }

    fn do_execute_with(&mut self, gi: usize, si: usize, start_from: usize) {
        if gi >= self.data.groups.len() || si >= self.data.groups[gi].sets.len() {
            return;
        }
        let set = &self.data.groups[gi].sets[si];
        let shell_cmd = set.shell.resolve_command();

        let (commands, index_offset) = if start_from == 0 {
            let cmds = set.commands.clone();
            self.exec_screen = Some(ExecutionScreenState::new(set.name.clone(), &cmds));
            self.pending_set = Some((gi, si));
            (cmds, 0usize)
        } else {
            let cmds = set.commands[start_from..].to_vec();
            if let Some(ref mut es) = self.exec_screen {
                es.reset_from(start_from);
            }
            (cmds, start_from)
        };

        self.execution.start(commands, set.exec_mode, set.variables.clone(), shell_cmd, index_offset);
        self.mode = AppMode::Execution;
    }

    fn teardown_execution(&mut self, keep_screen: bool, mark_skipped: bool) {
        self.execution.kill();
        if mark_skipped {
            if let Some(ref mut es) = self.exec_screen {
                es.mark_remaining_as_skipped();
            }
        }
        if !keep_screen {
            self.exec_screen = None;
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.execution.kill();
        let _ = storage::save_app_data(&self.data);
    }
}
```

**重要注意事项：**
- `render()` 方法定义在 `app/render.rs` 中，但 `app.rs` 中需要保留一个方法的签名作为转发。实际上，由于 `impl App` 块可以分散在多个文件中，`render()` 方法直接在 `app/render.rs` 中定义即可，`app.rs` 中不需要声明。Rust 允许同一个 crate 中多个 `impl App { }` 块。
- 直接删除 `app.rs` 中的 `render()` 方法定义（因为它在 `app/render.rs` 中定义）。但 `app.rs` 需要 `use ratatui::Frame;` 因为 `run()` 涉及到 `Frame` 的类型检查——实际上不直接使用 `Frame` 就不需要 import。不过 `run()` 中的 `terminal.draw(|f| self.render(f))` 会在调用处检查返回值类型，不需要在 app.rs 中 import Frame。
- 同理 `handle_key()` 在 `app/handler.rs` 中，不需要在 `app.rs` 中声明。

执行这个步骤时，可能会有编译错误需要逐个解决。最可能的问题：
1. 模块路径：`pub(crate) mod toast;` 声明的是 `crate::app::toast`，但在 `app.rs` 中 `use crate::app::toast::ToastManager;` 需要确保路径一致。
2. `render()` 的递归调用：需要确保没有循环调用（`app.rs` 中的转发调用 → `app/render.rs` 的实现）。

### Task 6: 验证并提交

- [ ] **Step 6.1: 编译检查**

```bash
cargo check 2>&1
```
预期：编译通过。如果报错，检查：
- 各子模块的 `use` 路径是否正确
- `focus` 类型 `DetailFocus` 是否已公开（`pub use` 或 `pub(crate)`）
- `exec_screen` 是 `Option` 类型，匹配时用 `ref`/`ref mut`
- `AppAction` 的 `use crate::action::AppAction` 在所有文件可用

- [ ] **Step 6.2: 单元测试**

```bash
cargo test 2>&1
```
预期：≥60 个测试全部通过。

- [ ] **Step 6.3: Clippy**

```bash
cargo clippy 2>&1
```
预期：无新增 warning。

- [ ] **Step 6.4: 格式化**

```bash
cargo fmt
```

- [ ] **Step 6.5: 提交**

```bash
git add src/action.rs src/app.rs src/app/ src/ui/ Cargo.toml
git commit -m "refactor(phase2): 提取 action.rs + app/ 目录（toast/execution/render/handler）"
```

注意：`Cargo.toml` 可能没有被修改（如果不需要加新依赖）。只需 add 变更过的 `.rs` 文件。

---

## 回滚指南

如果编译错误过多无法快速修复：

```bash
git checkout -- src/action.rs src/app.rs src/app/ src/ui/main_screen.rs src/ui/detail_screen.rs src/ui/execution_screen.rs src/ui/variable_screen.rs
```

或者 `git stash` 后从上次 commit 重置：

```bash
git reset --hard HEAD
```
