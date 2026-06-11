# Launcher 架构图

## 1. 整体界面跳转图

```mermaid
stateDiagram-v2
    [*] --> Main : 启动

    Main --> Detail : e / n（编辑/新建命令集）
    Detail --> Main : Ctrl+S（保存）/ Esc（取消）

    Main --> Help : ? / Ctrl+H
    Help --> Main : 任意键

    Main --> Execution : Enter（执行命令集）
    Main --> VarOverlay : Enter（有变量的命令集）
    VarOverlay --> Execution : Enter（确认变量值）
    VarOverlay --> Main : Esc（取消）

    Execution --> Main : q（返回）
    Execution --> Execution : Ctrl+C / s（中断/跳过，停留在执行屏）
    Execution --> Execution : n（继续执行剩余命令）
    Execution --> Execution : r（重新执行）

    note right of Main
        主屏内还有两个子模式：
        - 搜索模式（/ 进入，Esc/Enter 退出）
        - 重命名模式（R 进入，Enter/Esc 退出）
    end note
```

## 2. Main 屏内部逻辑

```mermaid
stateDiagram-v2
    state "Main 屏" as Main {
        [*] --> NormalMode : 初始状态

        state "普通模式" as NormalMode {
            [*] --> GroupsPanel : 默认焦点在左侧

            GroupsPanel --> GroupsPanel : ↑/↓ 导航分组
            GroupsPanel --> SetsPanel : →（有命令集时）
            GroupsPanel --> GroupsPanel : D 删除分组 / g 新建分组

            SetsPanel --> SetsPanel : ↑/↓ 导航命令集
            SetsPanel --> GroupsPanel : ←
            SetsPanel --> SetsPanel : d 删除命令集 / n 新建
            SetsPanel --> SetsPanel : Enter 执行 / e 编辑
        end

        NormalMode --> SearchMode : /
        SearchMode --> NormalMode : Esc（取消）/ Enter（选中）

        NormalMode --> RenameMode : R
        RenameMode --> NormalMode : Enter（确认）/ Esc（取消）
    }

    state "搜索模式" as SearchMode {
        [*] --> Typing : 输入关键词
        Typing --> Filtered : 实时过滤
        Filtered --> Jump : Enter → 跳转到选中的命令集
        Jump --> [*]
        Filtered --> [*] : Esc → 取消
    }

    state "重命名模式" as RenameMode {
        [*] --> Editing : 输入新名称
        Editing --> [*] : Enter → 确认改名
        Editing --> [*] : Esc → 取消
    }
```

### Main 屏布局

```
┌──────────────────────────────────────────────────┐
│  Groups (左1/3)          │ Sets (右2/3)          │
│  ┌────────────────┐      │ ┌────────────────────┐│
│  │ ▶ Group1  (3)  │      │ │ 🛑 SetName [bash]  ││
│  │   Group2  (1)  │      │ │ ⏩ SetName  [zsh]  ││
│  │   Group3  (0)  │      │ │ ...                ││
│  │   (empty -     │      │ │                    ││
│  │    press g)    │      │ │                    ││
│  └────────────────┘      │ └────────────────────┘│
├──────────────────────────────────────────────────┤
│ [↑/↓] [←/→] Panel [Enter] Run [e] [n] [d] [/]   │
└──────────────────────────────────────────────────┘
```

## 3. Detail 屏内部逻辑

```mermaid
stateDiagram-v2
    state "Detail 屏" as Detail {
        [*] --> Navigation : 进入编辑

        state "导航模式" as Nav {
            [*] --> Name : Tab 循环 6 个焦点区域
            Name --> Group : Tab
            Group --> Shell : Tab
            Shell --> ExecMode : Tab
            ExecMode --> Variables : Tab
            Variables --> Commands : Tab
            Commands --> Name : Tab（循环）
        end

        Nav --> NameEditing : Enter（在 Name 上）
        NameEditing --> Nav : Enter（确认）/ Esc（取消）

        Nav --> VarEditing : Enter / e（在 Variable 上）
        VarEditing --> Nav : Enter（确认）/ Esc（取消）

        Nav --> CmdEditing : Enter / e（在 Command 上）
        CmdEditing --> Nav : Enter（确认）/ Esc（取消）

        Nav --> VarInsert : a（在 Variables 上）
        VarInsert --> Nav : Enter（确认插入）/ Esc（取消）

        Nav --> CmdInsert : a（在 Commands 上）
        CmdInsert --> Nav : Enter（确认插入）/ Esc（取消）
    }

    Note -- 6个焦点区域用Tab/BackTab循环切换
    Note -- 导航模式下 ←/→ 切换 Group/Shell/ExecMode
    Note -- d 删除选中的 Variable/Command
    Note -- Ctrl+S 保存全屏, Esc 取消返回
```

### Detail 屏布局

```
┌─── Edit: CommandSetName ──────────────────────────┐
│  Name:  [deploy backend              ]     ← Enter编辑 │
│  Group: Deploy                             ← ←/→切换   │
│  Shell: bash                              ← ←/→切换   │
│  Mode:  Stop on Error                     ← ←/→切换   │
│                                                    │
│  ┌─ Variables (2) ─────────────────────────┐       │
│  │  server = 192.168.1.100                │       │
│  │  branch = main                         │       │
│  │  ▶ new_var = value             ← 插入预览│       │
│  └─────────────────────────────────────────┘       │
│                                                    │
│  ┌─ Commands (3) ─────────────────────────┐       │
│  │  #0  ssh {{server}} 'git pull'        │       │
│  │  #1  ssh {{server}} 'docker up'       │       │
│  │  #2▶ docker-compose restart  ← 编辑中/插入预览  │
│  └─────────────────────────────────────────┘       │
│                                                    │
│  [a] Add  [e] Edit  [d] Delete  [Tab] Next  Ctrl+S │
└────────────────────────────────────────────────────┘
```

### 变量编辑按键保护

```
变量格式： name=value
            ^-- 不可删除/←移动到此左边
               ^-- 光标自由在此区域
```

| 按键 | 效果 |
|------|------|
| ← | 不能移过 = 号 |
| Backspace | 不能删掉 = 号及以前部分 |
| Delete | 不能删掉 = 号 |
| Home | 光标到开头（绕过保护，可进入 name 区） |

## 4. Execution 屏内部逻辑

```mermaid
stateDiagram-v2
    state "Execution 屏" as Exec {
        [*] --> Running : 开始执行

        state "运行中" as Running {
            [*] --> Cmd1 : 执行第1条
            Cmd1 --> Cmd2 : ✅ 成功 / ❌ 失败(ContinueOnError)
            Cmd1 --> Completed : ❌ 失败(StopOnError)
            Cmd2 --> Cmd3 : ...
            CmdN --> Completed : 全部完成
        }

        Running --> Interrupted : s（跳过当前） / Ctrl+C（中断）
        Interrupted --> Completed : 标记剩余为 ⏭ Skipped

        Completed --> Running : n（继续执行剩余）
        Completed --> Running : r（全部重跑）
        Completed --> Main : q（返回主屏）
    }

    state "命令状态" as States {
        ⏳ Pending → ▶ Running → ✅ Success
        ▶ Running → ❌ Failure
        ▶ Running → ⏭ Skipped (通过 s/Ctrl+C)
        ⏳ Pending → ⏭ Skipped (剩余命令)
    }
```

### Execution 屏布局

```
┌─── Executing: CommandSetName [Running...] ────────┐
│                                                    │
│  ✅ $ echo "Hello" (0.15s)                        │
│    Hello                                           │
│                                                    │
│  ▶ $ sleep 10                         ← 正在执行   │
│                                                    │
│  ⏳ $ docker-compose up -d            ← 等待中     │
│                                                    │
│  ⏳ $ curl -I {{server}}                           │
│                                                    │
│  3 / 4 completed, 2 succeeded, 1 failed            │
│                                                    │
│ [q] Back to main  [s] Skip current  [Ctrl+C]       │
└────────────────────────────────────────────────────┘
```

## 5. 变量输入覆盖层（VariableScreen）

```mermaid
stateDiagram-v2
    state "变量输入弹窗" as Var {
        [*] --> Input1 : 焦点在第1个变量

        Input1 --> Input2 : Tab / ↓
        Input2 --> Input3 : Tab / ↓
        InputN --> Input1 : Tab / ↓（循环）
        Input3 --> Input2 : ↑
        Input1 --> [*] : Esc（取消执行）

        InputN --> Execute : Enter
        Execute --> [*] : 执行命令集
    }
```

### 变量弹窗布局

```
┌─── Set Variables ───────────┐
│  host = 192.168.1.100     ← 黄色=当前焦点 │
│  port = 8080                  │
│  branch = main              │
│                              │
│ [Enter]  [Esc]  [Tab/Down]  │
└──────────────────────────────┘
```

## 6. CLI 模式（独立路径）

```mermaid
flowchart LR
    Start([启动]) --> Args{有CLI子命令?}
    Args -->|是| CLI[CLI模式]
    Args -->|否| TUI[TUI模式]

    CLI --> Run["run --id <uuid>"]
    CLI --> Search["search --set/--group"]
    Run --> Execute["execute_set_blocking()"]
    Execute --> Exit([exit])
    Search --> Print([print结果])

    TUI --> Init["init_terminal()"]
    Init --> App["app.run()"]
    App --> Loop["事件循环"]
    Loop --> Restore(["restore_terminal()"])
```

### CLI 命令一览

```bash
launcher                      # 进入 TUI（无参数）
launcher run --id <uuid>      # 按 UUID 执行
launcher run --group G --set S --var key=val  # 按名称执行 + 变量覆写
launcher run --id XXX --var default           # 使用默认变量值（不提示）
launcher search --set <query>  # 搜索命令集
launcher search --group <query> # 搜索分组
```
