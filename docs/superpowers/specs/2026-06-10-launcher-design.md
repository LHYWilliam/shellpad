# Launcher — Rust TUI 命令集管理工具 设计文档

> 日期: 2026-06-10
> 状态: Draft

## 1. 概述

Launcher 是一个基于 Rust + Ratatui 构建的终端 TUI 工具。用户可以在其中创建**命令集（Command Set）**，每个命令集包含一组按顺序执行的 shell 命令，支持一键执行、变量注入、分组管理等功能。

### 核心需求

- 用户可创建/编辑/删除分组（用于组织命令集）
- 用户可创建/编辑/删除命令集
- 每条命令集中的命令可增删改排序
- 命令集支持变量（`{{var}}` 模板），执行前可修改变量值
- 支持多种 shell（bash/zsh/fish/自定义）
- 支持两种执行模式：遇错停止 / 遇错继续
- 全屏执行视图展示实时命令输出
- 全局搜索命令集

## 2. 架构方案

### 选型方案: JSON 存储 + 状态机导航

| 维度 | 选择 |
|------|------|
| **数据格式** | JSON 文件 (`~/.config/launcher/sets.json`) |
| **UI 导航** | 模式切换（Mode-based）: 主列表屏 → 详情/编辑屏 → 执行屏 |
| **持久化** | `serde_json` 序列化/反序列化，启动时加载，变更时保存 |
| **组织方式** | TUI 原生实现分组 + 全局搜索 |

**选型理由：**
- `serde` + `serde_json` 是 Rust 最成熟的组合，零学习成本
- 模式切换最契合需求——每个屏有清晰的职责
- 数百个命令集全量 JSON 加载内存操作完全够用
- 用户可编辑、可备份、可版本控制

## 3. UI 导航设计

### 模式定义

```rust
enum AppMode {
    Main,      // 主列表屏
    Detail,    // 详情/编辑屏
    Execution, // 全屏执行屏
    Help,      // 快捷键帮助
}
```

同一时间只显示一个屏，切换时全屏替换。

### 3.1 屏 1: 主列表屏（Main）

左右双栏布局：
- **左侧** — 分组树（Group List），支持折叠展开，选中切换右侧内容
- **右侧** — 当前分组下的命令集列表，支持上下选择
- **底部** — 状态栏快捷键提示

交互：
| 按键 | 功能 |
|------|------|
| `↑` `↓` | 导航 |
| `←` `→` | 展开/折叠分组 |
| `Enter` | 执行选中命令集（有变量先进入变量输入） |
| `e` | 编辑选中命令集 |
| `n` | 当前分组下新建命令集 |
| `d` | 删除命令集（带确认） |
| `/` | 全局搜索（即时筛选，匹配高亮） |
| `Ctrl+H` / `?` | 显示帮助 |

### 3.2 屏 2: 详情/编辑屏（Detail）

全屏编辑表单，从上到下依次为：

1. **元数据区** — 命令集名称（文本输入）、分组选择（下拉）、Shell 选择、执行模式选择
2. **变量区** — 表格：变量名 ↔ 默认值，支持增删
3. **命令区** — 命令列表，带序号，支持增删改排序

交互：
| 按键 | 功能 |
|------|------|
| `Tab` / `Shift+Tab` | 区域间切换焦点 |
| `↑` `↓` | 区域内导航 |
| `a` | 追加命令/变量 |
| `e` | 编辑选中项 |
| `d` | 删除选中项（带确认） |
| `Ctrl+S` | 保存并返回 |
| `Esc` | 取消返回（有确认提示） |

### 3.3 屏 3: 全屏执行屏（Execution）

全屏终端风格日志视图。

流程：
1. 如有变量 → 先弹变量输入屏（用户可覆盖默认值）
2. 进入执行屏，自动开始执行
3. 逐条展示命令的 stdout/stderr（边执行边渲染）
4. 已完成的标记 ✅ 或 ❌，正在执行的有 spinner

交互：
| 按键 | 功能 |
|------|------|
| `q` | 返回主界面（当前命令继续后台完成） |
| `Ctrl+C` | 发送 SIGINT 给当前子进程 |
| `r` | 重新执行 |
| `s` | 跳过当前命令 |

### 3.4 屏 4: 帮助屏（Help）

覆盖层（Overlay）展示所有快捷键绑定。

### 模式切换总图

```
                    ┌──────────┐
                    │  Main    │
                    └────┬─────┘
                         │
          ┌──────────────┼──────────────┐
          │ Enter        │ e            │ n
          ▼              ▼              │
   ┌──────────┐   ┌──────────┐         │
   │ Variable │   │ Detail   │ ◄───────┘
   │ 变量输入  │   │ 详情编辑  │
   └────┬─────┘   └──────────┘
        │ Enter              │ Ctrl+S / Esc
        ▼                    │
   ┌──────────┐              │
   │Execution │             │
   │ 全屏执行  │             │
   └────┬─────┘             │
        │ q                 │
        ▼                   ▼
   ┌──────────┐         ┌──────────┐
   │  Main    │ ◄───────│  Main    │
   └──────────┘         └──────────┘
```

## 4. 数据模型

### Rust 数据结构

```rust
struct AppData {
    groups: Vec<Group>,
}

struct Group {
    id: Uuid,
    name: String,
    sets: Vec<CommandSet>,
}

struct CommandSet {
    id: Uuid,
    name: String,
    group_id: Uuid,
    shell: ShellType,
    exec_mode: ExecMode,
    variables: Vec<Variable>,
    commands: Vec<Command>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

struct Variable {
    name: String,
    default_value: String,
}

struct Command {
    position: usize,
    command: String,
}

enum ShellType {
    SystemDefault,
    Bash,
    Zsh,
    Fish,
    Custom(String),
}

enum ExecMode {
    StopOnError,
    ContinueOnError,
}
```

### JSON 存储格式

路径: `~/.config/launcher/sets.json`

```json
{
  "groups": [
    {
      "id": "a1b2c3d4-...",
      "name": "部署相关",
      "sets": [
        {
          "id": "e5f6g7h8-...",
          "name": "部署到生产环境",
          "shell": "bash",
          "exec_mode": "stop_on_error",
          "variables": [
            { "name": "server", "default_value": "192.168.1.100" },
            { "name": "branch", "default_value": "main" }
          ],
          "commands": [
            { "position": 0, "command": "ssh {{server}} 'git pull origin {{branch}}'" },
            { "position": 1, "command": "ssh {{server}} 'docker-compose up -d'" }
          ],
          "created_at": "2026-06-10T10:00:00Z",
          "updated_at": "2026-06-10T10:30:00Z"
        }
      ]
    }
  ]
}
```

### 数据层约定
- 分组和命令集使用 UUID v4 作为唯一标识
- 命令排序使用 `position` 字段（0-based）
- 原子写入: 先写 `.tmp` 再 `rename`，避免文件损坏

## 5. 项目结构

```
src/
├── main.rs                # 入口：初始化终端、启动 App
├── app.rs                 # App 核心：模式管理、事件循环
├── mode.rs                # AppMode 枚举
├── models.rs              # 数据模型（Group, CommandSet, Command, Variable 等）
├── storage.rs             # JSON 文件加载/保存
├── executor.rs            # Shell 命令执行器（std::process::Command）
├── tui.rs                 # 终端初始化/恢复（crossterm raw mode）
├── config.rs              # 配置路径等常量
└── ui/
    ├── mod.rs             # ui 模块入口
    ├── main_screen.rs     # 主列表屏渲染 + 交互处理
    ├── detail_screen.rs   # 详情编辑屏渲染 + 交互处理
    ├── execution_screen.rs # 全屏执行屏渲染 + 交互处理
    ├── help_screen.rs     # 帮助屏渲染
    └── components.rs      # 复用组件（文本输入框、确认对话框等）
```

## 6. 依赖管理

```toml
[dependencies]
ratatui            = "0.29"
crossterm          = "0.28"
serde              = { version = "1", features = ["derive"] }
serde_json         = "1"
uuid               = { version = "1", features = ["v4"] }
chrono             = { version = "0.4", features = ["serde"] }
directories        = "6"
```

不使用 `tokio`/`rusqlite`/`toml` 等重量级依赖。

## 7. 执行引擎

### 执行流程

```
[用户按 Enter]
    │
    ├── 有变量 → 变量输入屏（用户可修改默认值）
    │
    └── 进入 Execution Mode
         │
         1. 变量替换 ("{{server}}" → "实际值")
         2. 按 position 排序
         3. 逐条执行:
            std::process::Command::new(shell)
                .arg("-c")
                .arg(替换后的命令)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
         4. 读取 stdout/stderr，实时渲染
         5. 检查退出码 → 根据 ExecMode 决定是否继续
         6. 全部完成 → 展示汇总
```

### 中断处理
- `Ctrl+C` → 发送 SIGINT 给当前子进程（不杀 launcher）
- `q` → 标记停止，当前命令后台完成，回到主屏

## 8. 错误处理

| 场景 | 处理方式 |
|------|----------|
| JSON 文件损坏/不存在 | 不存在则创建默认空结构；损坏则备份为 `.bak`，创建新文件 |
| Shell 不可执行 | 报错并回退到 `SystemDefault` |
| 命令执行失败 | 根据执行模式处理，失败标记 ❌ |
| 变量未填 | 有默认值则自动填充，无则标记必填 |
| 终端尺寸过小 | < 80×24 时提示用户放大终端 |
| 并发写入 | 不处理（单用户工具）。原子 `.tmp` + `rename` 防写入中断 |

## 9. 测试策略

- **单元测试** — 数据模型序列化/反序列化、变量替换逻辑、执行模式判断（`#[test]` 内嵌）
- **集成测试** — 存储模块读写、命令执行器模拟（`tests/` 目录）
- **手动测试** — TUI 交互流：创建分组 → 命令集 → 编辑命令 → 执行

重点覆盖: 变量替换逻辑（`"ssh {{server}}"` → `"ssh 192.168.1.100"`）
