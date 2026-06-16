---
title: 状态栏与 Help 内容修正
date: 2026-06-17
status: draft
---

## 1. 概述

修正全界面状态栏的 4 类问题 + Help 屏幕内容过时问题。

## 2. Main Screen 状态栏

### 问题
单条静态文本，不随 rename/search 子模式变化。

### 修复
`render_status_bar` 根据 `self.rename_mode` / `self.search_mode` 输出不同文本：

```rust
let text = if self.rename_mode {
    "[Enter] Confirm  [Esc] Cancel — renaming group"
} else if self.search_mode {
    "[Enter] Confirm  [Esc] Cancel  [↑/↓] Nav — searching"
} else {
    " [↑/↓] Nav  [←/→] Panel  [Enter] Run  [e] Edit  [n] New  [R] Rename group  [d] Del set  [D] Del group  [g] Group  [/] Search  [q] Quit"
};
```

变化：
- 修正 `R`（大写）和 `D`（大写）标注
- 移除 `[?] Help` / `[Shift+D]` 和 `[Ctrl+H]` — 用户可从 Help 屏幕获取这些信息，节省栏位

## 3. Detail Screen 状态栏

### 3.1 Variables/Commands 焦点缺 `[↑/↓]`

```rust
DetailFocus::Variables => " [a] Add  [e/Enter] Edit  [d] Delete  [↑/↓] Nav  [Tab] Next".into(),
DetailFocus::Commands => " [a] Add  [e/Enter] Edit  [d] Delete  [↑/↓] Nav  [Tab] Next".into(),
```

### 3.2 内联编辑中两个 `Esc` 冲突

当前：`[Enter] Confirm  [Esc] Cancel  |  [Ctrl+S] Save  [Esc] Cancel`

两个 `Esc` 含义不同（第一个取消内联编辑，第二个退出 Detail 屏幕）。修复：内联编辑状态下不显示第二段：

```rust
let text = if is_editing {
    format!(" {}  |  [Ctrl+S] Save  [Esc] Cancel", status)
} else {
    format!(" {}  |  [Ctrl+S] Save  [Esc] Cancel", status)
};
// 两段相同 — 保留结构但消除 is_editing 分支中的重复 Esc 提示
```

实际修复：内联编辑中省略尾部 `[Esc] Cancel`（因为当前正在编辑中，按 Esc 取消编辑而非退出屏幕）：

```rust
let text = if is_editing {
    format!(" [Enter] Confirm  [Esc] Cancel")
} else {
    format!(" {}  |  [Ctrl+S] Save  [Esc] Cancel", status)
};
```

## 4. Execution Screen 状态栏

### 4.1 所有模式补齐 `[←/→]`
←/→ 浏览在所有模式下都可用，应在状态栏体现：

| 状态 | 修复后 |
|------|--------|
| 运行中 + 聚焦 | `[←/→] Browse  [z] Follow  [s] Skip  [Ctrl+C] Interrupt  [q] Back` |
| 运行中 (正常) | `[←/→] Browse  [s] Skip  [z] Auto-scroll  [Ctrl+C] Interrupt  [q] Back` |
| 已完成 + 可继续 | `[←/→] Browse  [n] Continue from next  [r] Re-execute  [q] Back` |
| 已完成 (纯) | `[←/→] Browse  [r] Re-execute  [q] Back` |

```rust
let footer = if self.focus_index.is_some() {
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

## 5. Help 屏幕内容更新

就地更新 `help_screen.rs` 中的静态快捷键列表：

### Main Screen 节
```
    ↑/↓ / j/k     Navigate list
    ←/→           Switch between panels
    Enter         Execute selected command set
    e             Edit selected command set
    n             New command set
    d             Delete command set
    g             New group
    R             Rename group
    D             Delete group
    /             Search
```

### Detail Screen 节
```
    Tab/Shift+Tab  Switch focus region
    ↑/↓            Navigate list (variables/commands)
    ←/→            Cycle option (group/shell/mode)
    Enter / e      Edit selected item
    a              Add new item
    d              Delete selected item
    Ctrl+S         Save
    Esc            Cancel / Back
```

### Execution Screen 节
```
    ←/→            Browse command output
    z              Toggle auto-scroll / Follow current
    s              Skip current command (running)
    Ctrl+C         Interrupt (running)
    n              Continue from next (after skip, completed)
    r              Re-execute all (completed)
    q              Back to main
```

## 6. 变更文件

| 文件 | 操作 |
|------|------|
| `src/ui/main_screen/render.rs` | Main 状态栏改为动态 |
| `src/ui/detail_screen/render.rs` | Detail 状态栏补 ↑/↓ + 修 Esc 冲突 |
| `src/ui/execution_screen/render.rs` | Execution 状态栏补 ←/→ 到所有模式 |
| `src/ui/help_screen.rs` | Help 内容整体更新 |

## 7. 验证

```bash
cargo test   # 165 pass
cargo clippy
cargo run    # 检查各界面各状态的状态栏文本
```
