---
title: "项目文档补全设计"
date: 2026-06-17
status: draft
---

## 动机

项目经过架构重构和测试覆盖后，文档未同步更新。CLAUDE.md 中的模块描述、架构指引已过期，缺少面向用户的 README，关键模块缺少顶部说明。

## 范围

### 1. README.md

在项目根目录新建 `README.md`，包含以下章节：

- **项目简介** — Launcher 是什么（TUI 命令集管理 + 执行）
- **快速安装** — `cargo install --path .`
- **快速使用** — 创建组和执行集的例子
- **CLI 模式** — `launcher run --id <uuid>` 等命令
- **键盘快捷键** — 各模式快捷键汇总
- **数据文件** — `~/.config/launcher/sets.json`
- **构建** — 依赖和构建命令
- **截图占位** — 预留（当前无截图）

### 2. 关键模块 `//!` 文档

为以下 7 个模块顶部的 `mod.rs` 或主文件添加 `//!` 模块级文档：

| 模块 | 文件 | 文档内容 |
|------|------|---------|
| `action` | `src/action.rs` | 统一 Action 系统概述 |
| `app` | `src/app.rs` | App 结构体职责、子模块说明 |
| `config` | `src/config.rs` | 配置路径、最小终端尺寸 |
| `error` | `src/error.rs` | 错误类型层次 |
| `executor` | `src/executor/mod.rs` | 双路径执行架构（async/blocking） |
| `storage` | `src/storage.rs` | 原子写入机制、EXDEV 回退 |
| `ui` | `src/ui/mod.rs` | UI 模块结构概览 |

每个 `//!` 文档约 3-8 行，说明该模块的职责和关键概念。

### 3. CLAUDE.md 同步更新

过期的内容：
- 模块清单（重构后结构变化）
- 文件行数统计
- 架构描述（匹配当前模块树）

## 不变项

- 不添加逐函数 `///` 文档（已自文档化）
- 不创建用户手册（README 即用户手册）
- 不改动代码逻辑

## 变更清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `README.md` | 新建 | 项目介绍、安装、使用 |
| `src/action.rs` | 修改 | +`//!` |
| `src/app.rs` | 修改 | +`//!` |
| `src/config.rs` | 修改 | +`//!` |
| `src/error.rs` | 修改 | +`//!` |
| `src/executor/mod.rs` | 修改 | +`//!` |
| `src/storage.rs` | 修改 | +`//!` |
| `src/ui/mod.rs` | 修改 | +`//!` |
| `CLAUDE.md` | 修改 | 更新模块结构 |

## 验证

```bash
cargo doc --no-deps --open   # 查看 rustdoc 渲染效果
cargo build                   # 编译无影响
```
