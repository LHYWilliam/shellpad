# 状态栏文本分派模式统一

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 Detail 和 Execution 的状态栏文本分派改为与 Main 一致的扁平 `match` 模式

**Architecture:** 2 个独立任务，纯文本替换，无行为变更。Detail 用 `match (bool, enum)` 元组，Execution 用 `match (Option, bool, Option)` 元组。Main 保留现有 `if-else`（仅 3 条件）。

**Tech Stack:** Rust 2024 edition

---

### T1: Detail Screen — 嵌套 if-else-match → 单一 match

**文件：**
- Modify: `src/ui/detail_screen/render.rs:293-309`

- [ ] **替换完整函数体**

```rust
    pub(crate) fn render_status_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let is_editing = self.var_edit.is_editing() || self.cmd_edit.is_editing();
        let text = match (is_editing, self.focus) {
            (true, _) => "[Enter] Confirm  [Esc] Cancel",
            (false, DetailFocus::Name) => "[Enter] Edit name  [Tab] Next  |  [Ctrl+S] Save",
            (false, DetailFocus::Group) => "[←/→] Change group  [Tab] Next  |  [Ctrl+S] Save",
            (false, DetailFocus::Shell) => "[←/→] Change shell  [Tab] Next  |  [Ctrl+S] Save",
            (false, DetailFocus::ExecMode) => "[←/→] Change mode  [Tab] Next  |  [Ctrl+S] Save",
            (false, DetailFocus::Variables) => {
                "[a] Add  [e/Enter] Edit  [d] Delete  [↑/↓] Nav  [Tab] Next  |  [Ctrl+S] Save"
            }
            (false, DetailFocus::Commands) => {
                "[a] Add  [e/Enter] Edit  [d] Delete  [↑/↓] Nav  [Tab] Next  |  [Ctrl+S] Save"
            }
        };
        render_status_bar(frame, area, theme, text);
    }
```

- [ ] **验证**

```bash
cargo check
cargo test      # 165 pass
```

- [ ] **Commit**

```bash
git add src/ui/detail_screen/render.rs
git commit -m "refactor: unify detail status bar with flat match tuple pattern"
```

---

### T2: Execution Screen — 嵌套 if-else → 单一 match

**文件：**
- Modify: `src/ui/execution_screen/render.rs:150-159`

- [ ] **替换 footer_text 块**

```rust
        let footer_text = match (self.focus_index, self.completed, self.continue_from) {
            (Some(_), _, _) => "[←/→] Browse  [z] Follow  [q] Back",
            (None, true, None) => " [←/→] Browse  [r] Re-execute  [q] Back",
            (None, true, Some(_)) => " [←/→] Browse  [n] Continue from next  [r] Re-execute  [q] Back",
            (None, false, _) => " [←/→] Browse  [s] Skip  [z] Auto-scroll  [Ctrl+C] Interrupt  [q] Back",
        };
```

- [ ] **验证**

```bash
cargo check
cargo test      # 165 pass
```

- [ ] **Commit**

```bash
git add src/ui/execution_screen/render.rs
git commit -m "refactor: unify execution status bar with flat match tuple pattern"
```

---

## 验证清单

- [ ] `cargo test` — 165 pass
- [ ] `cargo clippy` — 1 warning（预存 too_many_arguments）
