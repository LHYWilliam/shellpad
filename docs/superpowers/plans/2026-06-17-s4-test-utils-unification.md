# S4 — 测试辅助函数统一

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 4 处完全相同的 `make_key` 辅助函数提取到共享测试模块，统一 `make_app`/`test_app` 命名，规范化测试 import 风格。

**Architecture:** 在 `lib.rs` 中声明 `#[cfg(test)] mod test_utils;`，新建 `src/test_utils.rs` 存放共享测试辅助函数。4 处 `make_key` 定义删除并改为 `use crate::test_utils::make_key`；`integration_tests.rs` 的 `test_app()` 改为使用 `test_utils::make_app()`。

**Tech Stack:** Rust 2024 edition, crossterm

---

## 现有状态

### `make_key` — 4 处完全相同

所有 4 处实现完全一致（仅参数/返回类型），函数体相同：

```rust
fn make_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}
```

| 文件 | 行号 |
|------|------|
| `src/ui/variable_screen.rs` | 148 |
| `src/ui/detail_screen/handler.rs` | 242 |
| `src/ui/detail_screen/editor.rs` | 88 |
| `src/ui/main_screen/handler.rs` | 216 |

每个文件的测试模块都同时 import 了 `KeyCode, KeyEvent, KeyModifiers`。

### `make_app()` vs `test_app()` — 2 处几乎相同

`src/app/handler.rs:286`:
```rust
fn make_app() -> App {
    App {
        data: AppData::empty(),
        mode: AppMode::Main,
        running: true,
        main_screen: MainScreenState::new(),
        detail_screen: None,
        execution_state: ExecutionState::Idle { pending_set: None },
        prev_mode: None,
        variable_screen: VariableScreenState::new(),
        theme: Theme::default_dark(),
        toasts: ToastManager::new(),
    }
}
```

`src/integration_tests.rs:17`:
```rust
fn test_app() -> App {
    use crate::app::ExecutionState;
    App {
        data: AppData::empty(),
        mode: AppMode::Main,
        running: true,
        main_screen: MainScreenState::new(),
        detail_screen: None,
        execution_state: ExecutionState::Idle { pending_set: None },
        prev_mode: None,
        variable_screen: crate::ui::variable_screen::VariableScreenState::new(),
        theme: Theme::default_dark(),
        toasts: ToastManager::new(),
    }
}
```

差异：`test_app` 有 `use crate::app::ExecutionState`（函数内 import），`VariableScreenState` 使用完整路径。字段值完全相同。统一为 `make_app` 放到 `test_utils.rs`。

### Test import 风格

| 文件 | 当前 | 目标 |
|------|------|------|
| `main_screen/handler.rs` | `use super::*` ✅ | — |
| `detail_screen/handler.rs` | `use super::*` ✅ | — |
| `detail_screen/editor.rs` | `use super::*` ✅ | — |
| `variable_screen.rs` | `use super::*` ✅ | — |
| `execution_screen/events.rs` | `use super::*` ✅ | — |
| `app/handler.rs` | `use super::{App, ExecutionState}` ❌ | `use super::*` |
| `integration_tests.rs` | `use crate::*` (integration test) | 保持不变 |

仅 `app/handler.rs` 需要修改。`integration_tests.rs` 是集成测试（位于 `src/` 根目录，无 `super`），import 风格不变。

---

## 文件变更

| 文件 | 操作 | 涉及任务 |
|------|------|---------|
| `src/test_utils.rs` | **Create** | T13, T14 |
| `src/lib.rs` | Modify（添加模块声明） | T13 |
| `src/ui/variable_screen.rs` | Modify（删除 `make_key`，import 共享版） | T13 |
| `src/ui/detail_screen/handler.rs` | Modify（删除 `make_key`，import 共享版） | T13 |
| `src/ui/detail_screen/editor.rs` | Modify（删除 `make_key`，import 共享版） | T13 |
| `src/ui/main_screen/handler.rs` | Modify（删除 `make_key`，import 共享版） | T13 |
| `src/integration_tests.rs` | Modify（删除 `test_app`，使用共享 `make_app`） | T14 |
| `src/app/handler.rs` | Modify（import 改为 `use super::*`，删除 `make_app`，改用共享版） | T14, T15 |

---

### Task 13: 提取 `make_key` 到共享模块

**文件：**
- Create: `src/test_utils.rs`
- Modify: `src/lib.rs`（添加 `#[cfg(test)] mod test_utils;`）
- Modify: `src/ui/variable_screen.rs`
- Modify: `src/ui/detail_screen/handler.rs`
- Modify: `src/ui/detail_screen/editor.rs`
- Modify: `src/ui/main_screen/handler.rs`

- [ ] **Step 1: 创建 `src/test_utils.rs`**

```rust
//! Shared test helpers for unit and integration tests.
//!
//! This module is `#[cfg(test)]` — only compiled during `cargo test`.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Shorthand for creating a key event with no modifiers.
pub(crate) fn make_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}
```

- [ ] **Step 2: 在 `src/lib.rs` 中声明 test_utils 模块**

先读取 `src/lib.rs` 确认当前内容：

```bash
head -20 src/lib.rs
```

然后在中追加 `#[cfg(test)] mod test_utils;`。位置：放在现有 `pub mod` 声明之后（通常在所有 `pub mod` 之后，`use` 语句之后）。

用 Read 工具查看 `lib.rs` 末尾，定位到最后一个 `mod` 声明后插入。

- [ ] **Step 3: 修改 4 个文件 — 删除私有 `make_key`，改用共享版**

每个文件的修改分两步：

**3a. 顶部 import 块追加：**

```rust
use crate::test_utils::make_key;
```

**3b. 删除 `fn make_key(...) -> KeyEvent { ... }` 函数定义。**

具体如下：

**文件 1: `src/ui/variable_screen.rs`**

当前 L146：
```rust
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
```
追加一行：
```rust
    use crate::test_utils::make_key;
```

当前 L148-150：
```rust
    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }
```
删除这 3 行。`KeyModifiers` 可能不再需要（如果只有 `make_key` 使用它），但 `KeyCode` 和 `KeyEvent` 可能仍在测试函数中被引用。保守处理：移除 `KeyModifiers` 如果没其他使用，保留 `KeyCode` 和 `KeyEvent`。

**文件 2: `src/ui/detail_screen/handler.rs`**

当前 L234：
```rust
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
```
追加：
```rust
    use crate::test_utils::make_key;
```

当前 L242-244：
```rust
    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }
```
删除。文件中仍有 `make_key(KeyCode::Tab)` 等调用——由共享 import 覆盖。

**文件 3: `src/ui/detail_screen/editor.rs`**

当前 L86：
```rust
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
```
追加：
```rust
    use crate::test_utils::make_key;
```

当前 L88-90：
```rust
    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }
```
删除。

**文件 4: `src/ui/main_screen/handler.rs`**

当前 L207：
```rust
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
```
追加：
```rust
    use crate::test_utils::make_key;
```

当前 L216-218：
```rust
    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }
```
删除。

- [ ] **Step 4: 检查未使用的 import 并清理**

T13 修改后，某些文件中 `KeyModifiers` 可能不再被测试代码直接使用（仅 `make_key` 内部用到）。编译时 Rust 会报告 unused import。逐个处理：

```bash
cargo test 2>&1 | grep -E "unused import|warning"
```

对于每个 warning：
- 如果仅 `KeyModifiers` 未使用：从 import 中移除 `KeyModifiers`
- 如果 `KeyCode` 未使用（不太可能，测试中需要构造 key code）：也应移除

注意：`detail_screen/handler.rs` 中有一处使用 `KeyModifiers::CONTROL`（`test_ctrl_s_returns_save_set` 测试 L323），所以 `KeyModifiers` 需要保留。

- [ ] **Step 5: 验证**

```bash
cargo test      # 165 pass
cargo clippy    # 2 pre-existing warnings
```

- [ ] **Step 6: Commit**

```bash
git add src/test_utils.rs src/lib.rs src/ui/variable_screen.rs src/ui/detail_screen/handler.rs src/ui/detail_screen/editor.rs src/ui/main_screen/handler.rs
git commit -m "refactor: extract duplicate make_key to shared test_utils module"
```

---

### Task 14: 统一 `make_app` / `test_app`

**文件：**
- Modify: `src/test_utils.rs`（追加 `make_app`）
- Modify: `src/integration_tests.rs`（删除 `test_app`，改用 `make_app`）
- Modify: `src/app/handler.rs`（删除 `make_app`，改用共享版）

- [ ] **Step 1: 在 `test_utils.rs` 中追加 `make_app`**

```rust
use crate::app::App;
use crate::app::toast::ToastManager;
use crate::mode::AppMode;
use crate::models::AppData;
use crate::ui::main_screen::MainScreenState;
use crate::ui::theme::Theme;

/// Create a minimal App for testing, with empty data and Main mode.
pub(crate) fn make_app() -> App {
    use crate::app::ExecutionState;
    App {
        data: AppData::empty(),
        mode: AppMode::Main,
        running: true,
        main_screen: MainScreenState::new(),
        detail_screen: None,
        execution_state: ExecutionState::Idle { pending_set: None },
        prev_mode: None,
        variable_screen: crate::ui::variable_screen::VariableScreenState::new(),
        theme: Theme::default_dark(),
        toasts: ToastManager::new(),
    }
}
```

- [ ] **Step 2: 修改 `src/integration_tests.rs`**

**2a.** 删除 L14-31（`test_app` 函数及其上方注释）。

**2b.** 在测试模块 import 中追加：
```rust
use crate::test_utils::make_app;
```

**2c.** 将 `test_app_crud_cycle` 中的 `test_app()` 调用改为 `make_app()`：
```rust
// 原 L74:
let mut app = test_app();
// 改为：
let mut app = make_app();
```

检查 `integration_tests.rs` 中其他使用 `test_app()` 的地方（通过 rg 搜索确认仅 `test_app_crud_cycle` 使用）。

```bash
rg "test_app\(\)" src/integration_tests.rs
```

**2d.** `test_app()` 函数的 import `use crate::app::ExecutionState` 在函数内部，删除函数即删除该 import，不需要单独处理。

- [ ] **Step 3: 修改 `src/app/handler.rs`**

**3a.** 删除 L286-299（`fn make_app() -> App { ... }`）。

**3b.** 在测试模块 import 中追加：
```rust
use crate::test_utils::make_app;
```

稍后在 T15 中会同时把 `use super::{App, ExecutionState}` 改为 `use super::*`。

但注意：删除 `make_app` 后，`App` 和 `ExecutionState` 可能不再被测试模块直接引用——仅 `make_app` 内部用到它们。如果测试中不再直接使用 `App` 类型（所有测试通过 `make_app()` 构造），则可以移除 `use super::{App, ExecutionState}`。但某些测试（如 `test_handler_back_to_main`）直接构造 `ExecutionState::Running { ... }`，所以 `ExecutionState` 仍需保留。

**3c.** 执行 `cargo check`，根据 warning 移除不再需要的 import。`App` 类型如果仅在已删除的 `make_app` 中使用，可以安全移除。

- [ ] **Step 4: 验证**

```bash
cargo test      # 165 pass
cargo clippy    # 2 pre-existing warnings
```

- [ ] **Step 5: Commit**

```bash
git add src/test_utils.rs src/integration_tests.rs src/app/handler.rs
git commit -m "refactor: unify make_app/test_app into shared test_utils::make_app"
```

---

### Task 15: 统一 `app/handler.rs` 测试 import 风格

**文件：**
- Modify: `src/app/handler.rs`

- [ ] **Step 1: 修改 `app/handler.rs` 测试模块 import**

将 `use super::{App, ExecutionState};` 改为 `use super::*`：

```rust
// 原（L274-285）：
use super::{App, ExecutionState};
use crate::action::AppAction;
use crate::app::execution::ExecutionManager;
use crate::app::toast::ToastManager;
use crate::mode::AppMode;
use crate::models::{AppData, CommandSet, Group};
use crate::ui::detail_screen::DetailScreenState;
use crate::ui::main_screen::{MainScreenState, Panel};
use crate::ui::theme::Theme;
use crate::ui::variable_screen::VariableScreenState;

// 改为：
use super::*;
use crate::action::AppAction;
use crate::app::execution::ExecutionManager;
use crate::app::toast::ToastManager;
use crate::mode::AppMode;
use crate::models::{AppData, CommandSet, Group};
use crate::ui::detail_screen::DetailScreenState;
use crate::ui::main_screen::{MainScreenState, Panel};
use crate::ui::theme::Theme;
use crate::ui::variable_screen::VariableScreenState;
use crate::test_utils::make_app;
```

仅第一行改变：`use super::{App, ExecutionState}` → `use super::*`。

`use super::*` 从父模块 `app` 导入 `App`（`pub`）和 `ExecutionState`（`pub(crate)`）。不会导入 `ExecutionManager` / `ToastManager`（它们在子模块中），所以这两个的显式 import 保留。

- [ ] **Step 2: 验证**

```bash
cargo test      # 165 pass
cargo clippy    # 2 pre-existing warnings
```

- [ ] **Step 3: Commit**

```bash
git add src/app/handler.rs
git commit -m "refactor: use super::* import style in app/handler tests"
```

---

## 注意事项

### 并行执行安全

T13 → T14 之间有依赖（T14 在 `test_utils.rs` 中追加 `make_app`，依赖 T13 已创建该文件）。T15 对 `app/handler.rs` 的修改与 T14 Step 3 修改同一文件，建议 T14 和 T15 合并执行（单次 commit）以避免冲突。

**推荐执行顺序：** T13 → T14+T15（合并）

### `integration_tests.rs` import 风格

`integration_tests.rs` 不使用 `super::*`——这是正确的，因为它是集成测试文件（`src/` 根级别），无 `super` 模块可用。保持 `use crate::*` 风格不动。

---

## 验证清单

- [ ] `cargo test` — 165 pass
- [ ] `cargo clippy` — 仅 2 pre-existing warnings，无新增
- [ ] `rg "fn make_key" src/` — 只在 `test_utils.rs` 中有定义
- [ ] `rg "fn make_app\b" src/` — 只在 `test_utils.rs` 中有定义
- [ ] `rg "fn test_app\b" src/` — 无结果
