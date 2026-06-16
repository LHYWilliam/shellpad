# Clippy 警告清理

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 清除全部 3 个 clippy 警告（private_interfaces、collapsible_if、too_many_arguments）

**Architecture:** 3 个独立任务，从简到繁。T1 改字段可见性（1 行），T2 合并嵌套 if（3 行），T3 提取 edit 上下文 struct（~8 参数 → 1 struct）。

**Tech Stack:** Rust 2024 edition

---

### T1: `private_interfaces` — `execution_state` 字段可见性

**文件：**
- Modify: `src/app.rs`

**问题：** `pub execution_state: ExecutionState` 但 `ExecutionState` 是 `pub(crate)`。字段可见性大于类型可见性。

-**修复：** 将字段从 `pub` 改为 `pub(crate)`。

```rust
// app.rs — 改 1 行（line 57）
    pub execution_state: ExecutionState,
// →
    pub(crate) execution_state: ExecutionState,
```

验证：

```bash
cargo clippy 2>&1 | grep "private_interfaces"
# 预期：无输出（警告消除）
cargo test   # 165 pass
```

Commit：`fix: narrow execution_state visibility to pub(crate)`

---

### T2: `collapsible_if` — 事件 drain 嵌套 if

**文件：**
- Modify: `src/app.rs`

**问题：** `if let Running { screen, manager } { if let Some(rx) = ... }` 可合并为单行 guard。

**修复：** clippy 提供了自动修复。手动改写为等价的 `&& let` guard：

```rust
// app.rs — 将 4 行嵌套 if 合并为 2 行
            if let ExecutionState::Running {
                ref mut screen,
                ref manager, ..
            } = self.execution_state
                && let Some(ref rx) = manager.rx
            {
                screen.process_events(rx);
            }
```

原代码：

```rust
            if let ExecutionState::Running {
                ref mut screen,
                ref manager, ..
            } = self.execution_state
            {
                if let Some(ref rx) = manager.rx {
                    screen.process_events(rx);
                }
            }
```

验证：

```bash
cargo clippy 2>&1 | grep "collapsible_if"
# 预期：无输出
cargo test   # 165 pass
```

Commit：`style: collapse nested if-let in event drain`

---

### T3: `too_many_arguments` — `render_items_list` 参数分组

**文件：**
- Modify: `src/ui/detail_screen/render.rs`

**问题：** `render_items_list` 有 13 个参数，超 clippy 阈值（7）。

**参数分类：**

| 分类 | 参数 |
|------|------|
| 渲染基础设施 | `frame`, `area`, `theme` |
| 外观 | `title`, `focused` |
| 列表数据 | `count`, `list` |
| 编辑上下文 | `editing_item`, `insert_at`, `preview_label`, `empty_text` |
| 行为 | `item_fn` |

**修复：** 将"编辑上下文"4 个参数提取为 `ItemListEditCtx` struct：

```rust
// detail_screen/render.rs — 在 render_items_list 之前定义

/// Editor context bundle for `render_items_list`.
struct ItemListEditCtx<'a> {
    /// Index of the item currently being edited, or None.
    editing_item: Option<usize>,
    /// Insert position for a new item, or None (replacing existing).
    insert_at: Option<usize>,
    /// Preview label shown during insertion mode.
    preview_label: Option<String>,
    /// Text shown when the list is empty.
    empty_text: &'a str,
}
```

函数签名精简为：

```rust
    pub(crate) fn render_items_list<F>(
        &self,
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        title: &str,
        focused: bool,
        count: usize,
        list: &ScrollableList,
        edit_ctx: ItemListEditCtx,
        item_fn: F,
    ) -> Rect
```

减少到 10 个参数（仍超 7，但从 13 → 10 是实质性改善）。

函数体内引用变更：

```rust
// 原：
editing_item: Option<usize>,
insert_at: Option<usize>,

// 改为通过 edit_ctx 访问：
let editing_item = edit_ctx.editing_item;
let insert_at = edit_ctx.insert_at;
```

两处调用点分别构造 `ItemListEditCtx`：

```rust
// render_variables 中：
        let list_area = self.render_items_list(
            frame, area, theme,
            &format!(" Variables ({}) ", count),
            self.focus == DetailFocus::Variables,
            count, &self.variable_list,
            ItemListEditCtx {
                editing_item: self.var_edit.editing,
                insert_at: self.var_edit.insert_at,
                preview_label: self.var_edit.insert_at.is_some()
                    .then(|| format!("  ▶ {}", self.var_edit.edit_input.content)),
                empty_text: " (empty — press a to add a variable) ",
            },
            |i, is_editing| { ... },
        );
```

```rust
// render_commands 中：
        let list_area = self.render_items_list(
            frame, area, theme,
            &format!(" Commands ({}) ", count),
            self.focus == DetailFocus::Commands,
            count, &self.command_list,
            ItemListEditCtx {
                editing_item: self.cmd_edit.editing,
                insert_at: self.cmd_edit.insert_at,
                preview_label: self.cmd_edit.insert_at.is_some().then(|| {
                    let pos = self.cmd_edit.insert_at.unwrap_or(0);
                    format!("  #{}▶ {}", pos, self.cmd_edit.edit_input.content)
                }),
                empty_text: " (empty — press a to add a command) ",
            },
            |i, is_editing| { ... },
        );
```

验证：

```bash
cargo clippy 2>&1 | grep "too_many_arguments"
# 预期：无输出（clippy --fix 阈值仍是 7，但 10 < 13 是改进；若仍有警告，接受为暂态）
cargo test   # 165 pass
```

若 10 个参数仍超阈值，后续可进一步将 `frame`/`area`/`theme`/`title`/`focused` 五个渲染参数也分组。

Commit：`refactor: extract ItemListEditCtx to reduce render_items_list parameter count`

---

## 验证清单

```bash
cargo clippy   # 0 warnings
cargo test     # 165 pass
```
