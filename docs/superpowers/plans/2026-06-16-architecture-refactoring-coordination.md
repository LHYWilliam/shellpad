# Launcher 架构重构 — 协调计划

> **For agentic workers:** REQUIRED SUB-SKILL: 使用 `superpowers:subagent-driven-development` 逐阶段执行。此计划为顶层协调计划，每个阶段在其开始时生成详细实施计划。步骤使用 checkbox (`- [ ]`) 语法追踪进度。

**Goal:** 将 Launcher 项目从 23 个单片文件重组为约 34 个模块化文件，每文件 ≤~300 行，依赖单向清晰，纯重构不改变行为。

**Architecture:** 大文件按"模块根声明子模块 → 提取渲染/处理/事件到子目录"的模式拆分。引入统一 `AppAction` 枚举替代 4 个分散的 Action 枚举，动作处理集中到 `app/handler.rs`。

**Tech Stack:** Rust 2024 edition, Ratatui 0.30, crossterm 0.29, serde 1.0, uuid 1.23, chrono 0.4, unicode-width 0.2, clap 4.5, directories 6.0

---

## 0. 前置阅读

实施前必须通读的设计规范：
- `docs/superpowers/specs/2026-06-16-architecture-refactoring.md`

关键章节：
- **§2 总体模块结构** — 最终目标文件树
- **§2.1 文件行数预估** — 各文件拆分的预期容量
- **§5 数据流与依赖关系** — 模块间依赖方向和禁止的反向依赖
- **§8 风险与注意事项** — import 路径变更、`cargo check` 时机等

---

## 1. 阶段总览

```
阶段 1: ui/render.rs + widget/ 提取
  产出: components.rs 消失，拆为 ui/render.rs + widget/ 目录 (3 文件)
  验证: cargo check + cargo test + cargo clippy 全绿

阶段 2: action.rs + app/ 目录重构
  产出: action.rs + app/ 目录 (4 文件)，app.rs 从 517 → ~80 行
  验证: cargo check + cargo test + cargo clippy 全绿

阶段 3: models/ 拆分
  产出: models.rs 变薄，拆出 models/types.rs + models/queries.rs
  验证: cargo check + cargo test + cargo clippy 全绿

阶段 4: 三大 Screen 拆分
  4a: main_screen/ 拆分 (629→200 行)
  4b: detail_screen/ 拆分 (582→200 行)
  4c: execution_screen/ 拆分 (357→120 行)
  验证: cargo check + cargo test + cargo clippy + cargo run (肉眼确认)
```

### 阶段依赖图

```
阶段 1 (ui/widget + render)
   ↓
阶段 2 (action.rs + app/)    ← 需要阶段 1 的 widget/render 就位
   ↓
阶段 3 (models/)              ← 独立于阶段 1/2，但需要阶段 2 的 action.rs 引用 model 类型
   ↓
阶段 4 (screens/拆分)          ← 需要阶段 2 的 AppAction 定义 + 阶段 3 的 model 路径
```

### 阶段间依赖说明

| 阶段 | 前置条件 | 本阶段产出的新模块 | 被后续阶段依赖 |
|------|---------|-----------------|--------------|
| 1 | 无 | `ui::render`, `ui::widget::*` | 阶段 2 的 `app/render.rs` 会使用 `ui::render` |
| 2 | 阶段 1 | `action::AppAction`, `app::*` | 阶段 4 的 screens 需要返回 `AppAction` |
| 3 | 阶段 2 (action.rs 的 path) | `models::types`, `models::queries` | 阶段 4 的 screens 导入 model 路径 |
| 4 | 阶段 1+2+3 | `main_screen::*`, `detail_screen::*`, `execution_screen::*` | 无 (最终态) |

---

## 2. 全局风险与统一策略

### 2.1 import 路径变更模式

最频繁的操作：`use crate::ui::components::{X, Y, Z}` → 拆分为多个 `use crate::ui::widget::*` + `use crate::ui::render::*`。

**策略：** 每个阶段完成后立即 `cargo check`，不累积 import 错误。

### 2.2 模块可见性

子模块在父文件中声明时必须指定可见性：

```rust
// main_screen.rs (模块根)
pub(crate) mod render;
pub(crate) mod handler;
pub(crate) mod search;
```

父文件通过 `use self::render::render_something` 引用子模块的公共函数。子模块通过 `use super::MainScreenState` 引用父文件定义的 struct。

### 2.3 impl 块分布在多个文件

不同文件中写 `impl MyStruct` 是 Rust 的标准实践：

```rust
// main_screen.rs
pub struct MainScreenState { ... }

// main_screen/render.rs
use super::MainScreenState;
impl MainScreenState {
    pub fn render_panels(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) { ... }
}
```

不需要 `pub use` 转发——子模块直接 `use super::TypeName`。

### 2.4 测试迁移

- 测试函数标记为 `#[cfg(test)]` + `pub(crate)`（或 `pub`）以确保被同模块可见
- 序列化/构造测试随类型定义走（留在 `types.rs`）
- 搜索测试迁入 `search.rs`
- 迁移后运行 `cargo test --nocapture` 确认所有测试路径正确

### 2.5 git 提交规范

```
refactor(phase1): 拆分 components.rs → ui/render.rs + widget/
refactor(phase2): 提取 action.rs + app/ 目录
refactor(phase3): 拆分 models.rs → models/types.rs + queries.rs
refactor(phase4a): 拆分 main_screen/ → render + handler + search
refactor(phase4b): 拆分 detail_screen/ → render + handler
refactor(phase4c): 拆分 execution_screen/ → render + events
```

每阶段一个独立 commit，可在任何一点 `git revert`。

### 2.6 一致性操作

每阶段提交前统一执行：
```bash
cargo check
cargo test
cargo clippy
cargo fmt
```

---

## 3. 各阶段实施顺序

### 阶段 1: `ui/render.rs` + `ui/widget/` 提取

**产出清单：**

| 操作 | 文件 | 说明 |
|------|------|------|
| 创建 | `ui/widget/mod.rs` | 声明并 re-export 子模块 |
| 创建 | `ui/widget/text_input.rs` | `TextInput` + `handle_text_input` |
| 创建 | `ui/widget/scrollable_list.rs` | `ScrollableList` |
| 创建 | `ui/widget/inline_edit.rs` | `InlineEdit` |
| 创建 | `ui/render.rs` | 9 个纯渲染辅助函数 |
| 删除 | `components.rs` | 拆分完成 |
| 更新 | 所有引用 `components` 的文件 | 改为 `widget/` 或 `render/` |

### 阶段 2: `action.rs` + `app/` 目录重构

**产出清单：**

| 操作 | 文件 | 说明 |
|------|------|------|
| 创建 | `action.rs` | `AppAction` 枚举 + 辅助方法 |
| 创建 | `app/toast.rs` | `ToastManager` |
| 创建 | `app/execution.rs` | `ExecutionManager` |
| 创建 | `app/render.rs` | `impl App { render() }` |
| 创建 | `app/handler.rs` | `impl App { handle_action() }` |
| 精简 | `app.rs` | struct + `new()` + `run()` + 子模块声明 |
| 更新 | 所有 screen | `handle_key()` 返回 `AppAction` |

### 阶段 3: `models/` 拆分

**产出清单：**

| 操作 | 文件 | 说明 |
|------|------|------|
| 创建 | `models/types.rs` | 全部数据 struct + label 方法 |
| 创建 | `models/queries.rs` | `impl AppData { all_sets_iter, find_set_by_id, filter_sets }` |
| 更新 | `models.rs` | 添加子模块声明 + re-export，原始定义迁出 |

### 阶段 4: 三大 Screen 拆分

**产出清单：**

| 操作 | 文件 | 说明 |
|------|------|------|
| 创建 | `main_screen/render.rs` | `impl MainScreenState { render() }` 提取 |
| 创建 | `main_screen/handler.rs` | `impl MainScreenState { handle_key() }` 提取 |
| 创建 | `main_screen/search.rs` | 搜索模式 + `find_matches_case_insensitive` + 测试 |
| 创建 | `detail_screen/render.rs` | `impl DetailScreenState { render_*() }` 提取 |
| 创建 | `detail_screen/handler.rs` | `impl DetailScreenState { handle_key() }` 提取 |
| 创建 | `execution_screen/render.rs` | `impl ExecutionScreenState { render() }` 提取 |
| 创建 | `execution_screen/events.rs` | `process_events()` + `CmdState` 转换 |
| 更新 | 各屏幕父文件 | 添加子模块声明，保留 struct + 分派逻辑 |

---

## 4. 验证清单

每个阶段完成后验证：

- [ ] `cargo check` — 无编译错误
- [ ] `cargo test` — 全部测试通过（每次 ≥30）
- [ ] `cargo clippy` — 无新增 warning（与重构前一致）
- [ ] `cargo fmt` — 代码格式一致

阶段 4 完成后额外验证：

- [ ] `cargo run` — TUI 正常启动、模式切换、列表显示
- [ ] 快捷键响应正确（`q`/`Tab`/`Enter`/`Esc` 等）
- [ ] 编辑/保存/执行功能正常

---

## 5. 快速参考：最终模块树

```
src/
├── main.rs
├── tui.rs                        (34 行)
├── config.rs                     (59 行)
├── mode.rs                       (12 行)
├── action.rs                     (~60 行)  ← 新
├── cli.rs                        (386 行)
├── storage.rs                    (193 行)
│
├── app.rs                        (~80 行)  ← 精简
├── app/
│   ├── handler.rs                (~220 行) ← 新
│   ├── render.rs                 (~120 行) ← 新
│   ├── toast.rs                  (~40 行)  ← 新
│   └── execution.rs              (~80 行)  ← 新
│
├── models.rs                     (模块根)
├── models/
│   ├── types.rs                  (~250 行) ← 新
│   └── queries.rs                (~80 行)  ← 新
│
├── ui/
│   ├── mod.rs
│   ├── render.rs                 (~120 行) ← 新
│   ├── widget/
│   │   ├── mod.rs                          ← 新
│   │   ├── text_input.rs         (~100 行) ← 新
│   │   ├── scrollable_list.rs   (~50 行)  ← 新
│   │   └── inline_edit.rs       (~90 行)  ← 新
│   ├── theme.rs                  (118 行)
│   ├── notification.rs           (~20 行)
│   ├── main_screen.rs            (~200 行) ← 精简
│   ├── main_screen/
│   │   ├── render.rs             (~200 行) ← 新
│   │   ├── handler.rs            (~150 行) ← 新
│   │   └── search.rs             (~80 行)  ← 新
│   ├── detail_screen.rs          (~200 行) ← 精简
│   ├── detail_screen/
│   │   ├── render.rs             (~200 行) ← 新
│   │   └── handler.rs            (~150 行) ← 新
│   ├── detail_editor.rs          (75 行)
│   ├── execution_screen.rs       (~120 行) ← 精简
│   ├── execution_screen/
│   │   ├── render.rs             (~120 行) ← 新
│   │   └── events.rs             (~100 行) ← 新
│   ├── variable_screen.rs        (138 行)
│   └── help_screen.rs            (56 行)
│
└── executor/                     (5 文件, 580 行)
```

## 6. 各阶段详细计划入口

- [ ] **Phase 1 执行**：阶段 1 详细计划在开始前生成
- [ ] **Phase 2 执行**：阶段 2 详细计划在 Phase 1 完成后生成
- [ ] **Phase 3 执行**：阶段 3 详细计划在 Phase 2 完成后生成
- [ ] **Phase 4 执行**：阶段 4 详细计划在 Phase 3 完成后生成
