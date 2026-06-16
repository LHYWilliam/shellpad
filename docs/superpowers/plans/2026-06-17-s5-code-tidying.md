# S5 — 代码整洁（分隔符统一 + Doc comment 覆盖）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 统一 1 处分隔符字符差异，为 2 个缺少文档的 widget struct 添加 doc comment。T16 和 T19 已确认跳过。

**Architecture:** 两个独立任务：T17 修改 execution_screen/render.rs 中一个字符；T18 修改 scrollable_list.rs 和 text_input.rs 的 struct 声明前追加 `///` 行。T16（AppData::empty）和 T19（thiserror 风格）按规范不做修改。

**Tech Stack:** Rust 2024 edition

---

## 前置说明

### T16 — 跳过

`AppData::empty()` 和 construct-new 模式的差异不影响使用，保留现状。

### T19 — 跳过

thiserror 变体风格（tuple vs. struct variant）属于主观偏好，不修复。

---

## 文件变更

| 文件 | 涉及任务 |
|------|---------|
| `src/ui/execution_screen/render.rs` | T17 |
| `src/ui/widget/scrollable_list.rs` | T18 |
| `src/ui/widget/text_input.rs` | T18 |

---

### Task 17: 分隔符字符统一

**文件：**
- Modify: `src/ui/execution_screen/render.rs`

**当前状态：** 代码库中仅 2 处使用 repeat-based 分隔符：

| 文件 | 行 | 字符 | 用途 |
|------|-----|------|------|
| `src/ui/render.rs` | 90 | `"─"` (U+2500 BOX LIGHT HORIZONTAL) | status bar 顶部线 ✅ |
| `src/ui/execution_screen/render.rs` | 117 | `"╌"` (U+254C BOX DOUBLE DASH HORIZONTAL) | 命令间分隔线 ❌ |

`render.rs` 使用正确的 `─`，`execution_screen/render.rs` 使用不同的 `╌`。统一为 `─`。

- [ ] **Step 1: 替换分隔符字符**

`src/ui/execution_screen/render.rs` L117：

```rust
// 原：
                let separator = "╌".repeat(sep_width);

// 改为：
                let separator = "─".repeat(sep_width);
```

完整上下文（L115-123）变更后：

```rust
            // Separator between commands
            if i + 1 < self.cmd_states.len() {
                let sep_width = area.width.saturating_sub(6) as usize;
                let separator = "─".repeat(sep_width);
                items.push(ListItem::new(Line::from(Span::styled(
                    separator,
                    Style::default()
                        .fg(theme.text_disabled)
                        .add_modifier(Modifier::DIM),
                ))));
            }
```

- [ ] **Step 2: 验证**

```bash
cargo check
cargo test      # 165 pass
cargo clippy    # 2 pre-existing warnings
```

- [ ] **Step 3: Commit**

```bash
git add src/ui/execution_screen/render.rs
git commit -m "style: unify separator character to U+2500 box light horizontal"
```

---

### Task 18: Widget struct doc comment 覆盖

**文件：**
- Modify: `src/ui/widget/scrollable_list.rs`
- Modify: `src/ui/widget/text_input.rs`

`InlineEdit` 已有 doc comment (`/// Generic inline text-edit state for a list.`)，无需修改。

- [ ] **Step 1: 为 `ScrollableList` 添加 doc**

`src/ui/widget/scrollable_list.rs` L1 当前：

```rust
pub struct ScrollableList {
    pub selected: usize,
    pub offset: usize,
}
```

改为：

```rust
/// Scrollable list selection state — tracks the selected index and visible
/// offset. Call [`update_offset`](ScrollableList::update_offset) each render
/// frame to keep the selection in view.
pub struct ScrollableList {
    pub selected: usize,
    pub offset: usize,
}
```

- [ ] **Step 2: 为 `TextInput` 添加 doc**

`src/ui/widget/text_input.rs` L9-13 当前：

```rust
#[derive(Clone)]
pub struct TextInput {
    pub content: String,
    pub cursor: usize,
}
```

改为：

```rust
/// Single-line text input with cursor tracking. Supports insert, delete,
/// and cursor movement. Use [`handle_text_input`] for key event processing.
#[derive(Clone)]
pub struct TextInput {
    pub content: String,
    pub cursor: usize,
}
```

- [ ] **Step 3: 验证**

```bash
cargo check
cargo doc --no-deps 2>&1 | grep -i "warning\|error"   # ensure no doc warnings
cargo test      # 165 pass
cargo clippy    # 2 pre-existing warnings
```

- [ ] **Step 4: Commit**

```bash
git add src/ui/widget/scrollable_list.rs src/ui/widget/text_input.rs
git commit -m "docs: add doc comments for ScrollableList and TextInput widget structs"
```

---

## 验证清单

- [ ] `cargo check` — 编译通过
- [ ] `cargo test` — 165 pass
- [ ] `cargo clippy` — 仅 2 pre-existing warnings，无新增
- [ ] `cargo doc --no-deps` — 无 broken intra-doc link warning
