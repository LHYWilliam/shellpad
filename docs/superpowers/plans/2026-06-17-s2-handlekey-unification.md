# S2 — handle_key 模式统一

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 统一 handler 文件中的 return 风格、修饰键检查模式、Panel import 方式

**Architecture:** 3 个可执行任务。Task 6（handle_key 参数统一）因 main_screen 深度依赖 `&AppData` 暂时标记为不可行——需要引入生命周期标注或缓存复制，超出本次一致性修复范围。

**Tech Stack:** Rust 2024 edition

---

## 前置说明

### Task 6 延期

main_screen 的 `handle_key(&mut self, key: KeyEvent, data: &AppData)` 与其他 screen 不同的原因是 main_screen 是"数据视图"——需要实时查询 `visible_sets`、`selected_group_idx`、`filter_sets` 等。其他 screen（Detail/Execution/Variable）各自拥有独立的数据副本，不需要外部引用。

统一参数签名需要以下方案之一：

| 方案 | 代价 |
|------|------|
| MainScreenState 存储 `&AppData` 引用 | 引入生命周期参数，波及 App struct 定义 |
| MainScreenState 存储 `AppData` 快照 | 每次按键深层克隆 |
| App 层预查询后传结构化数据 | 大幅重写 keyboard dispatch |

三者收益均为"统一了 4 个 handle_key 的签名"——不改变任何行为。建议后续在设计层面统一考虑，当前跳过。

---

### T7: return 风格统一

**受影响的 return 语句：**

- `main_screen/handler.rs:15-29` — `return match ...` 控制流（rename 模式早期退出）
- `main_screen/handler.rs:34-69` — `return match ...` 控制流（search 模式早期退出）
- `main_screen/handler.rs:130,138,152,161,188` — `return AppAction::X` 从 match 臂内提前返回
- `detail_screen/handler.rs:163,170,179,185` — `return AppAction::X` 从 match 臂内提前返回

**处理策略**：main_screen 的前两类 `return match` 是早期退出控制流，不应修改。只处理简单 `return AppAction::X`：

**文件：**
- Modify: `src/ui/main_screen/handler.rs`（5 处）
- Modify: `src/ui/detail_screen/handler.rs`（4 处）

- [ ] **main_screen/handler.rs — 转换 5 处 return 为表达式**

线 130（ExecuteSet）：
```rust
            KeyCode::Enter => {
                if self.active_panel == crate::ui::main_screen::Panel::Sets
                    && let Some((gi, si)) = self.selected_set_idx(data)
                {
                    return AppAction::ExecuteSet(gi, si);
                }
                AppAction::None
            }
// 改为——利用 Rust2024 let-chain 在 arm guard 中直接匹配：
            KeyCode::Enter
                if self.active_panel == crate::ui::main_screen::Panel::Sets
                    && let Some((gi, si)) = self.selected_set_idx(data) =>
            {
                AppAction::ExecuteSet(gi, si)
            }
            KeyCode::Enter => AppAction::None,
```

不行——Enter 需要两个臂，因为它在 Sets panel 时才触发 ExecuteSet，否则返回 None。这个设计用 `return` 无法避免。保留。

实际上分析一下：`return AppAction::X` 都是因为条件嵌套——外层 if-activated 条件检查，有条件时返回 action，否则返回 None。每个这样的臂是 2:1 的 if-else 模式（一个 action，一个 fallthrough）。改写为表达式风格需要为每个条件创建独立臂（会导致臂的数量翻倍）。

**结论：main_screen 的 return 保留——不是风格问题，是必要的控制流。**

**detail_screen 的 4 处才应修复。**

- [ ] **detail_screen/handler.rs — Escape arm 转换**

```rust
// 原（lines 181-187）：
            KeyCode::Esc => {
                if self.editing_name {
                    self.editing_name = false;
                } else {
                    return AppAction::CancelEdit;
                }
            }
// 改为：
            KeyCode::Esc if self.editing_name => {
                self.editing_name = false;
            }
            KeyCode::Esc => return AppAction::CancelEdit,
```

但这样引入了一个新的 `return`... 不是改进。真正的问题是 Esc 行为分叉（editing_name 时取消编辑，否则退出 screen）。这个分叉本身就是早期返回的合理用例。保留。

Line 163, 170（DeleteVariable/DeleteCommand）：
```rust
            KeyCode::Char('d' | 'D') => match self.focus {
                DetailFocus::Variables if !self.set.variables.is_empty() => {
                    let idx = ...;
                    return AppAction::DeleteVariable(idx);
                }
```
这里 `return` 之所以需要是因为 match 处于嵌套之中——`match self.focus` 内。可以改为将外层 match arm 拆分：

```rust
            KeyCode::Char('d') if self.focus == DetailFocus::Variables
                && !self.set.variables.is_empty() =>
            {
                let idx = self.variable_list.selected.min(self.set.variables.len().saturating_sub(1));
                AppAction::DeleteVariable(idx)
            }
            KeyCode::Char('d') if self.focus == DetailFocus::Commands
                && !self.set.commands.is_empty() =>
            {
                let idx = self.command_list.selected.min(self.set.commands.len().saturating_sub(1));
                AppAction::DeleteCommand(idx)
            }
            KeyCode::Char('d') => AppAction::None,
```

然后用 guard 模式替代 Ctrl+S 的 body 检查（line 174-180）：

```rust
            KeyCode::Char('s')
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                AppAction::SaveSet(self.set.clone())
            }
```

这同时解决了 **Task 8 modifier 风格** 在 detail_screen 中的差异——将 body 检查移回 match guard。

```rust
            KeyCode::Char('d') if self.focus == DetailFocus::Variables
                && !self.set.variables.is_empty() =>
            {
                let idx = self.variable_list.selected.min(self.set.variables.len().saturating_sub(1));
                AppAction::DeleteVariable(idx)
            }
            KeyCode::Char('d') if self.focus == DetailFocus::Commands
                && !self.set.commands.is_empty() =>
            {
                let idx = self.command_list.selected.min(self.set.commands.len().saturating_sub(1));
                AppAction::DeleteCommand(idx)
            }
            KeyCode::Char('d') => AppAction::None,
```

Line 179:

```rust
            KeyCode::Char('s')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                AppAction::SaveSet(self.set.clone())
            }
```

OK 我现在意识到计划的 return 风格统一在 detail_screen 中范围有限。只改'd'臂。并且已经包含了 Task 8。直接合并。

- [ ] **在 detail_screen/handler.rs 中替换'd'臂**

从：
```rust
            KeyCode::Char('d' | 'D') => match self.focus {
                DetailFocus::Variables if !self.set.variables.is_empty() => {
                    let idx = self
                        .variable_list
                        .selected
                        .min(self.set.variables.len().saturating_sub(1));
                    return AppAction::DeleteVariable(idx);
                }
                DetailFocus::Commands if !self.set.commands.is_empty() => {
                    let idx = self
                        .command_list
                        .selected
                        .min(self.set.commands.len().saturating_sub(1));
                    return AppAction::DeleteCommand(idx);
                }
                _ => {}
            },
```

改为：
```rust
            KeyCode::Char('d') if self.focus == DetailFocus::Variables
                && !self.set.variables.is_empty() =>
            {
                let idx = self.variable_list.selected.min(self.set.variables.len().saturating_sub(1));
                AppAction::DeleteVariable(idx)
            }
            KeyCode::Char('d') if self.focus == DetailFocus::Commands
                && !self.set.commands.is_empty() =>
            {
                let idx = self.command_list.selected.min(self.set.commands.len().saturating_sub(1));
                AppAction::DeleteCommand(idx)
            }
            KeyCode::Char('d') => AppAction::None,
```

注意：去掉 `| 'D'` 联合——大写 D 将沿用 default arm（有同样效果）。

- [ ] **判断**：是否还应修改 `KeyCode::Char('a' | 'A') => match self.focus { ... }`（添加变量/命令）臂？这些使用表达式风格，无 return。保留不必修改。

- [ ] **验证**

```bash
cargo check
cargo test      # 165 pass
```

- [ ] **Commit**

```bash
git add src/ui/detail_screen/handler.rs
git commit -m "refactor: remove return from detail d key handler, use guard style"
```

---

### T8: main_screen modifier 键检查改为 guard 风格

**文件：**
- Modify: `src/ui/main_screen/handler.rs`（L183-189）

- [ ] **Ctrl+H 臂替换**

从：
```rust
            KeyCode::Char('h') | KeyCode::Char('H') => {
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
                {
                    return AppAction::Help;
                }
                AppAction::None
            }
```

改为：
```rust
            KeyCode::Char('h') | KeyCode::Char('H')
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                AppAction::Help
            }
            KeyCode::Char('h') | KeyCode::Char('H') => AppAction::None,
```

- [ ] **验证**

```bash
cargo check
cargo test      # 165 pass
```

- [ ] **Commit**

```bash
git add src/ui/main_screen/handler.rs
git commit -m "refactor: use guard style for Ctrl+H check in main_screen handler"
```

---

### T9: Panel 枚举 import

**文件：**
- Modify: `src/ui/main_screen/handler.rs`

- [ ] **添加 import + 替换引用**

在顶部追加：
```rust
use super::Panel;
```

替换函数体内所有 `crate::ui::main_screen::Panel::Groups` → `Panel::Groups`，共 10 处，所有在 handle_key 函数体内。

- [ ] **验证**

```bash
cargo check
cargo test      # 165 pass
```

- [ ] **Commit**

```bash
git add src/ui/main_screen/handler.rs
git commit -m "refactor: import Panel enum in main_screen handler"
```

---

## 验证清单

- [ ] `cargo test` — 165 pass
- [ ] `cargo clippy` — 无新增 warning
