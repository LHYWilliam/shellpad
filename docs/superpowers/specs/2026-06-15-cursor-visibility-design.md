# 输入光标可见性增强 — 设计文档

## 问题

应用在多个输入场景下缺少可见光标，用户无法感知当前输入位置。已有的光标定位代码（`TextInput::render()`）是死代码，从未被调用；详情屏名称编辑和内联编辑完全缺少光标。

## 目标

在所有输入场景中显示闪烁竖线光标（BlinkingBar），使用户能清晰感知输入位置。

## 范围

- 终端光标样式：全局设置为 `BlinkingBar`
- 终端恢复：重置为默认光标
- 详情屏名称编辑：添加光标
- 详情屏变量/命令内联编辑：添加光标
- 统一光标定位辅助函数，修复 `TextInput::render()` 死代码

## 设计

### 1. 终端光标样式（tui.rs）

`init_terminal()` 添加 `SetCursorStyle::BlinkingBar`，`restore_terminal()` 重置为默认。

### 2. 光标辅助函数（components.rs）

新增 `set_cursor_after_prefix()`，所有输入场景统一调用。

`TextInput::render()` 改为调用此辅助函数，不再是死代码。

### 3. 详情屏名称编辑光标（detail_screen.rs）

`render_metadata()` 中，当 `self.editing_name` 为 true 时，在 "Name: " 前缀后定位光标。

### 4. 详情屏内联编辑光标（detail_screen.rs）

`render_variables()` 和 `render_commands()` 中，当正在编辑变量/命令时，在 `▶` 前缀后定位光标。

### 涉及文件

| 文件 | 改动 |
|------|------|
| `src/tui.rs` | 添加 SetCursorStyle::BlinkingBar / DefaultUserShape |
| `src/ui/components.rs` | 新增 `set_cursor_after_prefix()`，修复 `TextInput::render()` |
| `src/ui/detail_screen.rs` | 名称编辑 + 内联编辑添加光标定位 |
| `src/ui/variable_screen.rs` | 改用辅助函数（现有光标逻辑保持不变） |
| `src/ui/main_screen.rs` | 重命名光标改为辅助函数 |

### 不受影响

- 执行屏、帮助屏、主屏列表 — 不需要输入光标
- 数据模型、存储、执行引擎 — 无变化
- 现有测试 — 全部通过

## 验收标准

- [ ] 终端打开时光标为闪烁竖线（BlinkingBar）
- [ ] 详情屏编辑名称时在文字末尾显示光标
- [ ] 详情屏编辑变量/命令时在内容后显示光标
- [ ] 退出应用后光标恢复为终端默认
- [ ] `cargo test` 全部通过
- [ ] `cargo clippy` 无新增警告
