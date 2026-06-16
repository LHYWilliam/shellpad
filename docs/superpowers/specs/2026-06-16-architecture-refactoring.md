---
title: "Launcher 架构重构设计文档"
date: 2026-06-16
status: draft
---

## 1. 动机与目标

### 1.1 现状

项目当前 23 个源文件、4606 行，但 7 个文件占据了 82% 的代码量：

| 文件 | 行数 | 问题 |
|------|------|------|
| `main_screen.rs` | 629 | 渲染 + 键盘处理 + 搜索逻辑高度耦合 |
| `detail_screen.rs` | 582 | 渲染 + 6 个焦点区域的键盘处理混合 |
| `app.rs` | 517 | 状态机 + 事件循环 + 所有动作处理归于一文件 |
| `models.rs` | 413 | 类型定义与业务查询方法混合 |
| `components.rs` | 394 | 共享组件（Widget）+ 渲染辅助函数 + 事件处理混杂 |
| `cli.rs` | 386 | 解析 + 执行 + 测试扁平放置，但结构尚可接受 |
| `execution_screen.rs` | 357 | 事件消费 + 渲染 + 状态管理耦合 |

核心问题：
- 大文件含多个职责，无法"一眼看清这个文件做什么"
- Action 系统割裂：每个 Screen 有自己的 Action 枚举，`app.rs` 中 4 个 `on_*_action` 方法处理逻辑重复
- 组件与渲染辅助函数混在同一个文件中，不利于独立演化和测试

### 1.2 目标

- 每个文件 ≤~300 行（大致目标，非教条）
- 每个文件单一可描述的职责
- 模块依赖方向清晰，无循环引用
- 不改变任何运行时行为（纯重构）
- 保持现有测试全部通过，测试覆盖率不降级

### 1.3 非目标

- 不引入异步运行时（保持 `std::thread` + `mpsc`）
- 不改动 `executor/` 目录结构（已良好分离）
- 不修改外部 API（CLI 参数、数据格式、快捷键）
- 不改变 UI 行为或视觉呈现

---

## 2. 总体模块结构

```
src/
├── main.rs                           ← 入口点（不变）
├── tui.rs                            ← 终端初始化/恢复（不变，34 行）
├── config.rs                         ← XDG 路径 / 最小终端尺寸（不变，59 行）
│
├── app.rs                            ← 模块根：声明子模块 + App struct + new() + run()
├── app/
│   ├── handler.rs                    ← handle_action(AppAction)
│   ├── render.rs                     ← render() + overlay 渲染
│   ├── toast.rs                      ← ToastManager
│   └── execution.rs                  ← ExecutionManager
│
├── models.rs                         ← 模块根：声明子模块 + re-export
├── models/
│   ├── types.rs                      ← 所有数据 struct 定义 + label 方法
│   └── queries.rs                    ← AppData 查询方法
│
├── action.rs                         ← AppAction 统一枚举
│
├── mode.rs                           ← AppMode 枚举（不变，12 行）
├── cli.rs                            ← clap 解析 + 阻塞执行（结构不变，386 行）
├── storage.rs                        ← 原子 JSON 持久化（不变，193 行）
│
├── ui/
│   ├── mod.rs                        ← re-export
│   ├── render.rs                     ← 纯渲染辅助函数
│   ├── widget/
│   │   ├── mod.rs
│   │   ├── text_input.rs             ← TextInput + handle_text_input
│   │   ├── scrollable_list.rs        ← ScrollableList
│   │   └── inline_edit.rs            ← InlineEdit
│   ├── theme.rs                      ← Theme（不变，118 行）
│   ├── notification.rs               ← Toast/ToastSeverity 类型（精简）
│   ├── main_screen.rs                ← 模块根：声明子模块 + MainScreenState + 分派
│   ├── main_screen/
│   │   ├── render.rs                 ← 双面板渲染
│   │   ├── handler.rs                ← 键盘事件处理 + Action 构建
│   │   └── search.rs                 ← 搜索模式状态 + 高亮逻辑
│   ├── detail_screen.rs              ← 模块根：声明子模块 + DetailScreenState + 分派
│   ├── detail_screen/
│   │   ├── render.rs                 ← render_metadata / render_variables / render_commands
│   │   └── handler.rs                ← 焦点导航 + 编辑触发 + Action 构建
│   ├── detail_editor.rs              ← 变量/命令内联编辑处理（不变，75 行）
│   ├── execution_screen.rs           ← 模块根：声明子模块 + ExecutionScreenState + 分派
│   ├── execution_screen/
│   │   ├── render.rs                 ← 进度条、输出渲染、状态图标
│   │   └── events.rs                 ← process_events() + CmdState 转换
│   ├── variable_screen.rs            ← 变量输入覆盖（不变，138 行）
│   └── help_screen.rs                ← 帮助覆盖（不变，56 行）
│
└── executor/                         ← 保持不变（11 + 37 + 169 + 89 + 274 行）
    ├── mod.rs
    ├── events.rs
    ├── async_executor.rs
    ├── blocking.rs
    └── tests.rs
```

### 2.1 文件行数预估

| 文件 | 当前 | 重构后 | 变化 |
|------|------|--------|------|
| `app.rs` | 517 | ~80 | -437 |
| `app/handler.rs` | — | ~220 | +220 |
| `app/render.rs` | — | ~120 | +120 |
| `app/toast.rs` | — | ~40 | +40 |
| `app/execution.rs` | — | ~80 | +80 |
| `action.rs` | — | ~60 | +60 |
| `models.rs` | 413 | 0 (删) | -413 |
| `models/types.rs` | — | ~250 | +250 |
| `models/queries.rs` | — | ~80 | +80 |
| `components.rs` | 394 | 0 (删) | -394 |
| `ui/render.rs` | — | ~120 | +120 |
| `ui/widget/text_input.rs` | — | ~100 | +100 |
| `ui/widget/scrollable_list.rs` | — | ~50 | +50 |
| `ui/widget/inline_edit.rs` | — | ~90 | +90 |
| `ui/notification.rs` | 25 | ~20 | -5 |
| `main_screen.rs` | 629 | ~200 | -429 |
| `main_screen/render.rs` | — | ~200 | +200 |
| `main_screen/handler.rs` | — | ~150 | +150 |
| `main_screen/search.rs` | — | ~80 | +80 |
| `detail_screen.rs` | 582 | ~200 | -382 |
| `detail_screen/render.rs` | — | ~200 | +200 |
| `detail_screen/handler.rs` | — | ~150 | +150 |
| `execution_screen.rs` | 357 | ~120 | -237 |
| `execution_screen/render.rs` | — | ~120 | +120 |
| `execution_screen/events.rs` | — | ~100 | +100 |

总计约 **34 个文件**（含 mod.rs），核心文件均在 80-250 行范围。

---

## 3. 统一 Action 系统

### 3.1 全局枚举

所有 Screen 返回 `AppAction`，`app/handler.rs` 集中处理：

```rust
// action.rs
pub enum AppAction {
    None,
    Quit,

    // === Main screen ===
    EditSet(CommandSet, Vec<Group>),         // → Detail mode
    ExecuteSet(usize, usize),                // → Execution mode（可能先走 Variable overlay）
    DeleteSet(usize, usize),
    DeleteGroup(usize),
    RenameGroup(usize, String),          // (group_index, new_name)
    NewSet,
    NewGroup,
    ToggleSearch,

    // === Detail screen ===
    SaveSet(CommandSet),                     // → 保存并返回 Main
    CancelEdit,                              // → 不保存返回 Main
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
    ConfirmVariables(Vec<(String, String)>), // → 开始执行
    CancelVariables,                         // → 返回 Main
}
```

### 3.2 处理流程

```
User KeyEvent
  → Screen::handle_key(key) → AppAction
  → App::handle_action(action) → mutate state + auto_save() + mode transitions
  → next render cycle
```

每个 Screen 的 `handle_key()` 返回 `AppAction`（不再返回各自独立的 Action 枚举）。`app/handler.rs` 中的 `handle_action()` 统一处理所有情况。

### 3.3 消除的模式

当前：
```rust
// app.rs
fn on_main_action(&mut self, action: MainScreenAction) { ... }
fn on_detail_action(&mut self, action: DetailScreenAction) { ... }
fn on_exec_action(&mut self, action: ExecScreenAction) { ... }
fn handle_variable_action(&mut self, action: VariableAction) { ... }
```

重构后：
```rust
// app/handler.rs
impl App {
    pub fn handle_action(&mut self, action: AppAction) {
        match action {
            AppAction::EditSet(set, groups) => self.set_detail_mode(set, groups),
            AppAction::ExecuteSet(gi, si) => self.start_execution(gi, si),
            AppAction::SaveSet(set) => self.save_set_and_go_back(set),
            // ...
        }
    }
}
```

---

## 4. 各模块详述

### 4.1 `app.rs` + `app/` 目录

**`app.rs`** — 精简后的 App 结构体和生命周期：

```rust
pub struct App {
    pub running: bool,
    pub data: AppData,
    pub mode: AppMode,
    pub main_screen: MainScreenState,
    pub detail_screen: Option<DetailScreenState>,
    pub exec_screen: ExecutionScreenState,
    pub variable_screen: VariableScreenState,
    pub pending_set: Option<(usize, usize)>,
    pub execution: ExecutionManager,      // 从 app.rs 提取
    pub toasts: ToastManager,              // 从 app.rs 提取
    pub theme: Theme,
}
```

`App::new()` 从存储加载数据，初始化所有状态。`App::run()` 维持当前简洁的事件循环骨架（~25 行）。

**`app/toast.rs`** — Toast 数据管理：
- `ToastManager { toasts: Vec<Toast> }`
- `add(message, severity)` / `clean_expired()` — 只管理数据，不渲染
- Toast 和 ToastSeverity 类型保留在 `ui/notification.rs`，`ToastManager` 引用这些类型
- 现有 `push_toast()` 调用改为 `toasts.add()`
- Toast 的渲染在 `app/render.rs` 中完成（遍历 `toast_manager.toasts` 绘制覆盖层）

**`app/execution.rs`** — 执行生命周期：
- `ExecutionManager { rx, handle, kill_signal }`
- `start(set, gi, si)` / `kill()` / `poll_events()`
- 当前 `app.rs` 中的 `execution_rx`、`execution_handle`、`kill_signal` 字段迁入
- `pending_set` 保留在 `App` 结构体上（服务于变量覆盖层→执行的过渡）

**`app/render.rs`** — 渲染逻辑：
- `impl App` 中 render 方法的完整实现
- 标题栏、模式分派、help/variable/toast overlay 渲染
- `render_status_bar` 已在 `ui/render.rs` 中（从 components.rs 提取的纯函数）

**`app/handler.rs`** — 动作处理：
- `impl App { fn handle_action(&mut self, action: AppAction) }`
- 集中处理所有模式转换和数据变更
- `auto_save()` 在每次变更后调用

### 4.2 `models/` 目录

**`models/types.rs`** — 纯类型定义：
```rust
pub struct AppData { pub groups: Vec<Group> }
pub struct Group { pub id: Uuid, pub name: String, pub sets: Vec<CommandSet> }
pub struct CommandSet { pub id: Uuid, /* ...所有字段 */ }
pub struct Variable { pub name: String, pub default_value: String }
pub struct Command { pub position: usize, pub command: String }
pub enum ShellType { /* 6 个变体 */ }
pub enum ExecMode { StopOnError, ContinueOnError }
```
保留：`ShellType::label()`、`ExecMode::label()`、`ShellType::builtin_variants()`、`ShellType::resolve_command()`、`CommandSet::new()`、`Group::new()`——这些是类型自带的行为。

**`models/queries.rs`** — `AppData` 上的查询方法：
```rust
impl AppData {
    pub fn all_sets_iter(&self) -> impl Iterator<Item = (usize, usize, &CommandSet)> { .. }
    pub fn find_set_by_id(&self, id: Uuid) -> Option<(usize, usize, &CommandSet)> { .. }
    pub fn filter_sets(&self, query: &str) -> Vec<(usize, usize, &CommandSet)> { .. }
}
```
这些提取自 `models.rs`，便于独立测试。

### 4.3 `action.rs`

`AppAction` 枚举 + 辅助方法（如 `is_exit()` 等），独立文件以便被 `app/` 和所有 `ui/*_screen.rs` 引用。

### 4.4 `ui/` 目录

**`ui/render.rs`** — 从 `components.rs` 提取的纯渲染函数：
- `bordered_block()`、`bordered_block_info()`
- `centered_rect()`
- `render_scrollbar()`
- `render_inline_cursor()`
- `set_cursor_after_prefix()`
- `empty_hint()`
- `fill_row()`
- `list_scrollbar_areas()`
- `render_status_bar()`

所有这些函数只依赖 `ratatui` 和 `Theme`，不涉及任何内部可变状态。

**`ui/widget/` 目录**：

- `text_input.rs` — `TextInput` struct + 所有方法（insert/delete/cursor 操作）+ `handle_text_input` 函数（从 `components.rs` 提取）
- `scrollable_list.rs` — `ScrollableList` struct + `select_next/previous/update_offset/selected_or_none`
- `inline_edit.rs` — `InlineEdit` struct + `commit/cancel/handle_key_protected/is_editing`

**`ui/notification.rs`** — 精简为纯类型定义：`Toast { message: String, severity: ToastSeverity, created_at: Instant }` 和 `ToastSeverity { Success, Error, Info }`。不包含管理逻辑。

**`app/toast.rs`** — 管理逻辑：`ToastManager { toasts: Vec<Toast> }`，提供 `add()`、`clean_expired()` 方法。渲染在 `app/render.rs` 中完成。`app/handler.rs` 通过 `self.toasts.add(...)` 触发通知。

**剩余 Screen 文件** 按 "薄的顶级文件 + 子目录" 模式：
- `main_screen.rs` — struct + `render()` 分派 + `handle_key()` 分派（~200 行）
- `main_screen/render.rs` — 渲染 Group 面板 + Set 面板（~200 行）
- `main_screen/handler.rs` — 键盘事件到 AppAction 的映射（~150 行）
- `main_screen/search.rs` — 搜索模式 TextInput + `find_matches_case_insensitive` 逻辑 + 现有测试（~80 行）
- `detail_screen.rs`、`detail_screen/render.rs`、`detail_screen/handler.rs` — 同上模式
- `execution_screen.rs`、`execution_screen/render.rs`、`execution_screen/events.rs` — 同上模式

### 4.5 `storage.rs`、`cli.rs`、`executor/`

不拆分，仅：
- `cli.rs` + `executor/blocking.rs` 之间的接口不做改动
- `storage.rs` 的原子写入模式保持

---

## 5. 数据流与依赖关系

### 5.1 依赖方向

```
main.rs
  ├── tui.rs
  ├── config.rs
  ├── app.rs → app/ (handler, render, toast, execution)
  │     ├── mode.rs
  │     ├── action.rs
  │     ├── models/ (types, queries)
  │     ├── storage.rs
  │     ├── ui/
  │     │     ├── render.rs
  │     │     ├── widget/ (text_input, scrollable_list, inline_edit)
  │     │     ├── theme.rs
  │     │     ├── notification.rs
  │     │     ├── *screen.rs → */ (handler, render, ...)
  │     │     └── *editor.rs
  │     └── executor/ (events, async_executor)
  │
  └── cli.rs ──→ executor/ (blocking, events)  ← cli 独立运行，不依赖 app/ui
```

### 5.2 关键原则

- `models/` 不可依赖 `ui/`、`app/`、`executor/` — 纯数据层
- `action.rs` 可以依赖 `models/`（因为 `AppAction` 枚举携带了 `CommandSet`、`Group` 等类型），不可依赖 `app/`、`ui/`
- `ui/` 可以依赖 `models/`、`action.rs` — 但不能反向依赖 `app/` 或 `executor/`
- `app/` 可以依赖所有模块
- `cli.rs` 不依赖 `app/` 或 `ui/`（目前已经是这样，保持）

---

## 6. 测试策略

### 6.1 现有测试迁移

| 当前位置 | 测试数量 | 目标位置 |
|---------|---------|---------|
| `models.rs` | 12 | `models/queries.rs`（类型构造/序列化测试保留在 `types.rs`） |
| `main_screen.rs` | 7 | `main_screen/search.rs` |
| `executor/tests.rs` | 18 | 保持不变 |
| `cli.rs` | 14 | 保持不变 |
| `storage.rs` | 5 | 保持不变 |
| `config.rs` | 3 | 保持不变 |

### 6.2 新增测试机会

保持纯重构原则，本次不要求新增测试覆盖，但在拆分过程中若发现现有逻辑有显然可测的纯函数（如 `main_screen/handler.rs` 中的键盘映射逻辑），可在迁移时同步添加。不过这是 bonus，不构成必须交付项。

### 6.3 验证方式

```
cargo check       # 重构过程中保持无编译错误
cargo test        # 重构后全部 30 个测试仍通过
cargo clippy      # 无新增 warning
cargo run         # 肉眼验证 TUI 行为未变
```

---

## 7. 实施阶段

整个重构划分为 4 个独立阶段，每个阶段后 `cargo check + cargo test` 通过：

### 阶段 1：`ui/render.rs` + `ui/widget/` 提取

从 `components.rs` 拆分出 4 个文件：
- `ui/render.rs`（纯渲染函数）
- `ui/widget/text_input.rs`（TextInput + handle_text_input）
- `ui/widget/scrollable_list.rs`（ScrollableList）
- `ui/widget/inline_edit.rs`（InlineEdit）

**步骤：**
1. 创建 `ui/widget/mod.rs`
2. 将 `TextInput`（+ `handle_text_input`）→ `ui/widget/text_input.rs`
3. 将 `ScrollableList` → `ui/widget/scrollable_list.rs`
4. 将 `InlineEdit` → `ui/widget/inline_edit.rs`
5. 将渲染辅助函数 → `ui/render.rs`
6. 将 `Toast`、`ToastSeverity` 保留在 `ui/notification.rs`（精简）
7. 删除 `components.rs`
8. 更新所有 `use` 导入

### 阶段 2：`action.rs` + `app/` 目录重构

**步骤：**
1. 创建 `action.rs`，定义 `AppAction` 枚举
2. 更新所有 Screen 的 `handle_key()` 返回类型为 `AppAction`
3. 创建 `app/toast.rs`：提取 ToastManager（`app.rs` 作为模块根声明子模块）
4. 创建 `app/execution.rs`：提取 ExecutionManager
5. 创建 `app/render.rs`：从 `app.rs` 提取 render 逻辑
6. 创建 `app/handler.rs`：从 `app.rs` 提取 handle_action
7. 精简 `app.rs` 为 struct + new() + run()

### 阶段 3：`models/` 拆分

**步骤：**
1. 创建 `models/types.rs`：类型定义 + label 方法（`models.rs` 作为模块根声明子模块）
2. 创建 `models/queries.rs`：AppData 查询方法
3. 在 `models.rs` 头部补充 `pub(crate) mod types; pub(crate) mod queries;` + re-export
4. 删除 `models.rs` 中的原定义（已迁至子文件）
5. 更新所有 `use` 导入

### 阶段 4：三大 Screen 拆分

按文件线数从多到少依次处理：

**子阶段 4a：`main_screen/` 拆分**
1. 在 `main_screen.rs` 头部补充 `pub(crate) mod render; pub(crate) mod handler; pub(crate) mod search;`
2. 创建 `main_screen/render.rs`：提取渲染逻辑
3. 创建 `main_screen/handler.rs`：提取键盘处理
4. 创建 `main_screen/search.rs`：提取搜索逻辑（含测试迁移）

**子阶段 4b：`detail_screen/` 拆分**
1. 在 `detail_screen.rs` 头部补充 `pub(crate) mod render; pub(crate) mod handler;`
2. 创建 `detail_screen/render.rs`：提取渲染逻辑
3. 创建 `detail_screen/handler.rs`：提取键盘处理

**子阶段 4c：`execution_screen/` 拆分**
1. 在 `execution_screen.rs` 头部补充 `pub(crate) mod render; pub(crate) mod events;`
2. 创建 `execution_screen/render.rs`：提取渲染逻辑
3. 创建 `execution_screen/events.rs`：提取事件处理

---

## 8. 风险与注意事项

1. **模块路径变更影响**：拆分后 `use` 路径会变。建议阶段 1 和阶段 2 先做（影响面最大），一个阶段完成后立即 `cargo check`，避免累积编译错误难以定位。
2. **测试可见性**：`search.rs` 中的测试函数可以保持 `pub(crate)` 或 `pub`，确保 `#[cfg(test)]` 模块能直接访问。
3. **`detail_editor.rs`** 中的 `handle_variable_edit` 和 `handle_command_edit` 函数目前接受 `&mut InlineEdit` 等参数，接口合理，不需要变化。
4. **无 rustfmt 配置**：项目使用默认 rustfmt 格式化，各阶段完成后 `cargo fmt` 确保风格一致。
5. **git 管理**：每个阶段独立 commit，方便回滚。commit message 遵循 `refactor: 拆分 X → Y/Z` 格式。
6. **阶段顺序验证**：1→2→3→4 的顺序已验证合理。阶段 2 在源文件上修改 `handle_key` 签名，阶段 4 再将已更新代码转移到子目录。每行最多被触及 2 次（签名变更 + 移动），无重复劳动。

---

*设计文档版本 v1 — 2026-06-16*
