# 状态栏与 Help 内容修正

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修正 Main/Detail/Execution 三界面的状态栏文本 + 更新 Help 屏幕快捷键列表

**Architecture:** 4 个独立任务，每个修改单个文件的纯文本字符串。无需单元测试——变更仅涉及静态字符串和条件分支，通过 cargo test + cargo run 手动验证。

**Tech Stack:** Rust 2024 edition

---

### T1: Main Screen 状态栏动态化

**文件：**
- Modify: `src/ui/main_screen/render.rs:259-265`

- [ ] **替换状态栏文本为条件分支**

```rust
// 将原静态文本：
            "[↑/↓] Nav  [←/→] Panel  [Enter] Run  [e] Edit  [n] New  [d] Del set  [Shift+D] Del group  [g] Group  [/] Search  [?] Help  [q] Quit",

// 改为条件分支：
            if self.rename_mode {
                "[Enter] Confirm  [Esc] Cancel — renaming group"
            } else if self.search_mode {
                "[Enter] Confirm  [Esc] Cancel  [↑/↓] Nav — searching"
            } else {
                "[↑/↓] Nav  [←/→] Panel  [Enter] Run  [e] Edit  [n] New  [R] Rename  [d] Del set  [D] Del group  [g] New group  [/] Search  [q] Quit"
            },
```

- [ ] **验证**

```bash
cargo check
cargo test      # 165 pass
cargo run       # 检查：普通、搜索、重命名三种状态
```

- [ ] **Commit**

```bash
git add src/ui/main_screen/render.rs
git commit -m "fix: make main screen status bar dynamic for rename/search modes"
```

---

### T2: Detail Screen 状态栏修正

**文件：**
- Modify: `src/ui/detail_screen/render.rs:293-309`

- [ ] **修正 Variables/Commands 焦点 + 内联编辑 Esc 冲突**

```rust
    pub(crate) fn render_status_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let is_editing = self.var_edit.is_editing() || self.cmd_edit.is_editing();
        let text: String = if is_editing {
            "[Enter] Confirm  [Esc] Cancel".into()
        } else {
            let status = match self.focus {
                DetailFocus::Name => "[Enter] Edit name  [Tab] Next".into(),
                DetailFocus::Group => "[←/→] Change group  [Tab] Next".into(),
                DetailFocus::Shell => "[←/→] Change shell  [Tab] Next".into(),
                DetailFocus::ExecMode => "[←/→] Change mode  [Tab] Next".into(),
                DetailFocus::Variables => {
                    "[a] Add  [e/Enter] Edit  [d] Delete  [↑/↓] Nav  [Tab] Next".into()
                }
                DetailFocus::Commands => {
                    "[a] Add  [e/Enter] Edit  [d] Delete  [↑/↓] Nav  [Tab] Next".into()
                }
            };
            format!(" {}  |  [Ctrl+S] Save  [Esc] Cancel", status)
        };
        render_status_bar(frame, area, theme, &text);
    }
```

- [ ] **验证**

```bash
cargo check
cargo test      # 165 pass
cargo run       # 检查：各焦点 + 内联编辑中
```

- [ ] **Commit**

```bash
git add src/ui/detail_screen/render.rs
git commit -m "fix: add up/down hints and resolve double-Esc in detail status bar"
```

---

### T3: Execution Screen 状态栏补齐

**文件：**
- Modify: `src/ui/execution_screen/render.rs:150-159`

- [ ] **替换 footer_text 逻辑**

```rust
        let footer_text = if self.focus_index.is_some() {
            if self.completed {
                "[←/→] Browse  [z] Follow  [q] Back"
            } else {
                "[←/→] Browse  [z] Follow  [s] Skip  [Ctrl+C] Interrupt  [q] Back"
            }
        } else if self.completed {
            if self.continue_from.is_some() {
                " [←/→] Browse  [n] Continue from next  [r] Re-execute  [q] Back"
            } else {
                " [←/→] Browse  [r] Re-execute  [q] Back"
            }
        } else {
            " [←/→] Browse  [s] Skip  [z] Auto-scroll  [Ctrl+C] Interrupt  [q] Back"
        };
```

- [ ] **验证**

```bash
cargo check
cargo test      # 165 pass
cargo run       # 检查：运行中/完成/聚焦各状态
```

- [ ] **Commit**

```bash
git add src/ui/execution_screen/render.rs
git commit -m "fix: add left/right browse hints to all execution status bar states"
```

---

### T4: Help 屏幕内容更新

**文件：**
- Modify: `src/ui/help_screen.rs:19-50`

- [ ] **替换 Help 快捷键列表**

```rust
    let lines = vec![
        Line::from(""),
        Line::from("  Global:").style(Style::default().fg(section_color)),
        Line::from("    ? / Ctrl+H    Show this help"),
        Line::from("    q             Quit / Go back"),
        Line::from(""),
        Line::from("  Main Screen:").style(Style::default().fg(section_color)),
        Line::from("    up/down / j/k Navigate list"),
        Line::from("    left/right    Switch between panels"),
        Line::from("    Enter         Execute selected command set"),
        Line::from("    e             Edit selected command set"),
        Line::from("    n             New command set"),
        Line::from("    d             Delete command set"),
        Line::from("    g             New group"),
        Line::from("    R             Rename group"),
        Line::from("    D             Delete group"),
        Line::from("    /             Search"),
        Line::from(""),
        Line::from("  Detail Screen:").style(Style::default().fg(section_color)),
        Line::from("    Tab/Shift+Tab  Switch focus region"),
        Line::from("    up/down        Navigate list (variables/commands)"),
        Line::from("    left/right     Cycle option (group/shell/mode)"),
        Line::from("    Enter / e      Edit selected item"),
        Line::from("    a              Add new item"),
        Line::from("    d              Delete selected item"),
        Line::from("    Ctrl+S         Save"),
        Line::from("    Esc            Cancel / Back"),
        Line::from(""),
        Line::from("  Execution Screen:").style(Style::default().fg(section_color)),
        Line::from("    left/right     Browse command output"),
        Line::from("    z              Toggle auto-scroll / Follow current"),
        Line::from("    s              Skip current command (running)"),
        Line::from("    Ctrl+C         Interrupt (running)"),
        Line::from("    n              Continue from next (after skip)"),
        Line::from("    r              Re-execute all (completed)"),
        Line::from("    q              Back to main"),
        Line::from(""),
        Line::from("  Press any key to close."),
    ];
```

- [ ] **验证**

```bash
cargo check
cargo test      # 165 pass
cargo run       # Main 界面按 ? → 检查全部三节内容
```

- [ ] **Commit**

```bash
git add src/ui/help_screen.rs
git commit -m "docs: update Help screen with all current key bindings"
```

---

## 验证清单

- [ ] `cargo test` — 165 pass
- [ ] `cargo clippy` — 1 warning（预存 too_many_arguments）
- [ ] `cargo run` — 手动验证全部状态栏文本和 Help 内容
