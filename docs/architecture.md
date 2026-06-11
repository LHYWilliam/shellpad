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
    direction LR
    [*] --> GroupsPanel : 初始（默认）
    GroupsPanel --> GroupsPanel : ↑/↓ 导航
    GroupsPanel --> SetsPanel : →（有命令集）
    SetsPanel --> SetsPanel : ↑/↓ 导航
    SetsPanel --> GroupsPanel : ←
    GroupsPanel --> SearchMode : /
    SetsPanel --> SearchMode : /
    SearchMode --> GroupsPanel : Esc 取消
    SearchMode --> SetsPanel : Enter 选中结果
    GroupsPanel --> RenameMode : R 重命名
    RenameMode --> GroupsPanel : Enter 确认
    RenameMode --> GroupsPanel : Esc 取消
```
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
    direction LR
    [*] --> NavName : 进入编辑

    state "Tab 循环 6 焦点" as Nav {
        NavName --> NavGroup : Tab
        NavGroup --> NavShell : Tab
        NavShell --> NavMode : Tab
        NavMode --> NavVar : Tab
        NavVar --> NavCmd : Tab
        NavCmd --> NavName : Tab
    }

    NavName --> NameEdit : Enter
    NameEdit --> NavName : Enter / Esc

    NavVar --> VarEdit : Enter/e
    VarEdit --> NavVar : Enter / Esc
    NavVar --> VarInsert : a
    VarInsert --> NavVar : Enter / Esc

    NavCmd --> CmdEdit : Enter/e
    CmdEdit --> NavCmd : Enter / Esc
    NavCmd --> CmdInsert : a
    CmdInsert --> NavCmd : Enter / Esc
```

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
    direction LR
    [*] --> Cmd1 : 开始执行
    Cmd1 --> Cmd2 : 成功
    Cmd1 --> Cmd2 : 失败/ContinueOnError
    Cmd1 --> Completed : 失败/StopOnError
    Cmd2 --> Cmd3 : ...
    CmdN --> Completed : 全部完成
    Cmd1 --> Completed : s跳过/Ctrl+C中断
    Cmd2 --> Completed : s跳过/Ctrl+C中断
    Completed --> CmdN : n继续执行剩余
    Completed --> Cmd1 : r重新执行全部
    Completed --> BackMain : q
```

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
