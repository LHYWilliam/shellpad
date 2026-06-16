---
title: 项目代码模式一致性整合
date: 2026-06-17
status: draft
---

## 1. 背景

通过全代码库扫描，发现 19 处"同类行为不同实现"的模式不一致。这些不一致不是架构缺陷，而是开发过程中缺乏统一约定的自然积累。本规范将其分类为 5 个阶段，按影响从高到低依次修复。

## 2. 总览

| 优先级 | 数量 | 类别 |
|--------|------|------|
| P0 — 实际行为不一致 | 3 | auto_save 遗漏、空状态缺失、error 处理不统一 |
| P1 — 设计模式不一致 | 6 | handle_key 签名、return 风格、修饰键检查等 |
| P2 — 代码风格不一致 | 6 | 测试辅助函数重复、import 风格、doc 注释等 |
| P3 — 琐碎不一致 | 4 | 分隔符字符、toast severity、构造函数命名等 |
| **总计** | **19** | |

## 3. 实施阶段

### 阶段 1：行为修复

| # | 问题 | 操作 | 文件 | 预估 |
|---|------|------|------|------|
| 1 | DeleteVariable/Command 未 auto_save | 追加 `self.auto_save()` 调用 | `app/handler.rs` | 5m |
| 2 | Execution 界面无空状态提示 | 追加 `empty_hint` | `execution_screen/render.rs` | 10m |
| 3 | async_executor 中 send 错误处理不一致 | 统一为 `is_err() { return }` 模式 | `executor/async_executor.rs` | 10m |
| 4 | Toast severity 中 SaveSet 用 Success | 改为 Info（与其他创建/保存操作一致） | `app/handler.rs` | 2m |
| 5 | CLI "Error: " 前缀不一致 | 统一为 `eprintln!("Error: {}", e)` | `cli.rs` | 5m |

### 阶段 2：handle_key 统一

| # | 问题 | 操作 | 文件 | 预估 |
|---|------|------|------|------|
| 6 | handle_key 参数不统一 | 所有 screen 统一为 `pub fn handle_key(&mut self, key: KeyEvent)`；main_screen 改为从 `App` 传入 data | `main_screen/handler.rs`, `app/handler.rs` | 20m |
| 7 | return 风格不统一 | 所有 match arm 中去除 `return` 关键字，统一为表达式风格 | 3 个 handler 文件 | 15m |
| 8 | 修饰键检查不统一 | 全部改为 match guard `if key.modifiers.contains(...)` | 3 个 handler 文件 | 10m |
| 9 | Panel 枚举未 import | `main_screen/handler.rs` 中 import `Panel` 类型，去除 `crate::ui::main_screen::Panel::` 前缀 | `main_screen/handler.rs` | 5m |

### 阶段 3：render 模式统一

| # | 问题 | 操作 | 文件 | 预估 |
|---|------|------|------|------|
| 10 | 列表项构建两种风格 | main_screen 改用 `styled_list_item` | `main_screen/render.rs` | 10m |
| 11 | Execution 空状态缺失 | 追加 `empty_hint` 渲染 | `execution_screen/render.rs` | 5m |
| 12 | render 方法可见性不统一 | 统一为 `pub(crate)` | 3 个 render 文件 | 5m |

### 阶段 4：测试辅助函数统一

| # | 问题 | 操作 | 文件 | 预估 |
|---|------|------|------|------|
| 13 | `make_key` 4 处拷贝 | 提取到共享测试模块 | 4 个 handler 文件 | 15m |
| 14 | `make_app()` vs `test_app()` | 统一名字 | 2 个文件 | 5m |
| 15 | 测试 import 风格 | 统一为 `use super::*` | 多个文件 | 10m |

### 阶段 5：代码整洁

| # | 问题 | 操作 | 文件 | 预估 |
|---|------|------|------|------|
| 16 | AppData 用 `empty()` 而非 `new()` | 保留（不影响使用） | — | 不修复 |
| 17 | 分隔符字符不一致 | 统一为 `─` | 2 个 render 文件 | 5m |
| 18 | Doc comment 覆盖 | 为 widget struct 添加 doc | 4 个文件 | 10m |
| 19 | thiserror 变体风格 | 保留（风格偏好） | — | 不修复 |

## 4. 变更文件总览

| 阶段 | 文件 |
|------|------|
| S1 | `app/handler.rs`, `execution_screen/render.rs`, `executor/async_executor.rs`, `cli.rs` |
| S2 | `main_screen/handler.rs`, `detail_screen/handler.rs`, `execution_screen/mod.rs`, `app/handler.rs` |
| S3 | `main_screen/render.rs`, `execution_screen/render.rs` |
| S4 | 跨 6 个测试文件 |
| S5 | 2 个 render 文件 + 4 个 doc 文件 |

## 5. 验证

每阶段完成后：

```bash
cargo test   # 165 pass
cargo clippy # 无新增 warning
```
