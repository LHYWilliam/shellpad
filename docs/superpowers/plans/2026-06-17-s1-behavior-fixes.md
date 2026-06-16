# S1 — 行为修复

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复 5 处实际行为不一致：auto_save 遗漏、empty_hint 缺失、send error 处理、toast severity、CLI error 前缀

**Architecture:** 5 个独立任务，按文件分组。T1+T4 同文件（handler.rs）合并执行。

**Tech Stack:** Rust 2024 edition

---

## 文件变更

| 文件 | 涉及任务 |
|------|---------|
| `src/app/handler.rs` | T1, T4 |
| `src/execution_screen/render.rs` | T2 |
| `src/executor/async_executor.rs` | T3 |
| `src/cli.rs` | T5 |

---

### T1 + T4: auto_save 补救 + toast severity 统一

**文件：**
- Modify: `src/app/handler.rs`

- [ ] **DeleteVariable 追加 auto_save**（line 175 之前）

```rust
// 在 DeleteVariable arm 中，toast 之前追加：
                    self.auto_save();
                    self.toasts.add("Variable deleted", ToastSeverity::Info);
```

- [ ] **DeleteCommand 追加 auto_save**（line 190 之前）

```rust
// 在 DeleteCommand arm 中，toast 之前追加：
                    self.auto_save();
                    self.toasts.add("Command deleted", ToastSeverity::Info);
```

- [ ] **SaveSet toast 改为 Info**（line 159）

```rust
            self.toasts.add("Command set saved", ToastSeverity::Info);
```

- [ ] **验证**

```bash
cargo test      # 165 pass
cargo clippy    # 1 warning (pre-existing)
```

- [ ] **Commit**

```bash
git add src/app/handler.rs
git commit -m "fix: add auto_save for DeleteVariable/DeleteCommand, normalize SaveSet toast to Info"
```

---

### T2: Execution 界面空状态提示

**文件：**
- Modify: `src/ui/execution_screen/render.rs`

- [ ] **在 items 构建末尾追加 empty_hint**

在 `for (i, state) in self.cmd_states.iter().enumerate() { ... }` 循环之后、summary 之前，追加：

```rust
        // Append empty-state hint if no commands
        if self.cmd_states.is_empty() {
            let empty = empty_hint(theme, " (no commands to display — press q to go back) ");
            items.insert(0, empty);
        }
```

- [ ] **追加 import**

```rust
// render.rs 顶部 — 追加 empty_hint
use crate::ui::render::{list_scrollbar_areas, render_scrollbar, render_status_bar, empty_hint};
```

- [ ] **验证**

```bash
cargo test      # 165 pass
```

- [ ] **Commit**

```bash
git add src/ui/execution_screen/render.rs
git commit -m "fix: add empty-state hint in execution screen output list"
```

---

### T3: async_executor send 错误处理统一

**文件：**
- Modify: `src/executor/async_executor.rs`

**当前状态：**

| 行 | 方式 | 上下文 |
|----|------|--------|
| 85 | `if tx.send(...).is_err() { return; }` | Starting 事件 |
| 94 | `let _ = tx.send(...)` (静默忽略) | spawn 失败 stderr |
| 98 | `let _ = tx.send(...)` (静默忽略) | spawn 失败 Finished |
| 136 | `if tx.send(...).is_err() { return; }` | Finished 事件 |
| 156 | `let _ = tx.send(...)` (静默忽略) | CompletedAll |

- [ ] **统一为 early return 模式**

```rust
// 将行 94 改为：
92:             Err(e) => {
93:                 // Spawn failure — notify via channel if possible
94:                 if tx.send(ExecutionEvent::StderrLine {
95:                     index: actual_index,
96:                     line: format!("Failed to spawn command: {}", e),
97:                 }).is_err() { return; }
98:                 if tx.send(ExecutionEvent::Finished {
99:                     index: actual_index,
100:                    success: false,
101:                    duration_ms: cmd_start.elapsed().as_millis(),
102:                }).is_err() { return; }
103:                failed += 1;
104:                if matches!(exec_mode, ExecMode::StopOnError) {
105:                    break;
106:                }
107:                continue;
108:            }
```

```rust
// 将行 156 改为：
155:        if tx.send(ExecutionEvent::CompletedAll {
156:            total,
157:            succeeded,
158:            failed,
159:            total_duration_ms: start.elapsed().as_millis(),
160:        }).is_err() { return; }
```

- [ ] **验证**

```bash
cargo test      # 165 pass (executor tests must still pass)
cargo clippy
```

- [ ] **Commit**

```bash
git add src/executor/async_executor.rs
git commit -m "fix: unify channel send error handling to early-return pattern"
```

---

### T5: CLI error 前缀统一

**文件：**
- Modify: `src/cli.rs`

**当前：**
- line 59: `eprintln!("Error: {}", e)` ✅ 有前缀
- line 91: `eprintln!("{}", e)` ❌ 无前缀
- line 112: `eprintln!("{}", e)` ❌ 无前缀
- line 132: `eprintln!("Execution error: {}", e)` 有前缀但用词不同

- [ ] **统一为 `Error:` 前缀**

```rust
// line 91:
            eprintln!("Error: {}", e);

// line 112:
            eprintln!("Error: {}", e);

// line 132:
            eprintln!("Error: {}", e);
```

- [ ] **验证**

```bash
cargo test      # 165 pass
```

- [ ] **Commit**

```bash
git add src/cli.rs
git commit -m "fix: unify CLI error messages with Error: prefix"
```

---

## 验证清单

- [ ] `cargo test` — 165 pass
- [ ] `cargo clippy` — 1 warning（pre-existing）
