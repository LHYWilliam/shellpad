# Variable/Command Add & Edit Toast — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add toast notifications for variable/command add and edit operations (Enter commit), matching the existing delete toast pattern.

**Architecture:** Two new `AppAction` variants (`VariableSaved`, `CommandSaved`) are returned by editor.rs on Enter commit. handler.rs dispatches them with a toast. No auto_save — variable/command changes live in the detail screen's copy and are persisted via SaveSet (same as delete).

**Tech Stack:** Rust

---

### Task: Add VariableSaved/CommandSaved actions + editor return + handler toast

**Files:**
- Modify: `src/action.rs`
- Modify: `src/ui/detail_screen/editor.rs`
- Modify: `src/app/handler.rs`

- [ ] **Step 1: Add new AppAction variants**

In `src/action.rs`, after `DeleteCommand(usize)`:

```rust
    // === Variable/Command edited in Detail Screen ===
    VariableSaved,
    CommandSaved,
```

- [ ] **Step 2: Return new actions from editor.rs**

In `handle_variable_edit`, change the `Enter` commit closure to return `AppAction::VariableSaved` instead of `AppAction::None`:

```rust
    dispatch_inline_edit(edit, key,
        |e| {
            let input = e.edit_input.content.clone();
            if let Some(eq_pos) = input.find('=') {
                let name = input[..eq_pos].trim().to_string();
                let value = input[eq_pos + 1..].trim().to_string();
                e.commit(idx, variables, Variable { name, default_value: value }, list);
            } else if !input.is_empty() {
                e.commit(idx, variables, Variable { name: input.trim().to_string(), default_value: String::new() }, list);
            }
            AppAction::VariableSaved  // ← changed from AppAction::None
        },
```

In `handle_command_edit`, same change — commit closure returns `AppAction::CommandSaved`:

```rust
        |e| {
            let cmd = e.edit_input.content.clone();
            e.commit(idx, commands, Command { position: idx, command: cmd }, list);
            for (i, c) in commands.iter_mut().enumerate() {
                c.position = i;
            }
            AppAction::CommandSaved  // ← changed from AppAction::None
        },
```

- [ ] **Step 3: Handle new actions in handler.rs**

In `app/handler.rs`, add handler arms. Place after `DeleteCommand` handler:

```rust
            AppAction::VariableSaved => {
                self.toasts.add("Variable saved", ToastSeverity::Info);
            }
            AppAction::CommandSaved => {
                self.toasts.add("Command saved", ToastSeverity::Info);
            }
```

- [ ] **Step 4: Update editor tests**

Tests in `editor.rs` expect `AppAction::None` on commit. Change to `AppAction::VariableSaved` / `AppAction::CommandSaved`.

- [ ] **Step 5: Verify compilation and tests**

Run: `cargo check && cargo test`
Expected: All tests PASS

- [ ] **Step 6: Commit**

```bash
git add src/action.rs src/ui/detail_screen/editor.rs src/app/handler.rs
git commit -m "feat: add toast for variable/command add and edit operations

Enter commit in inline editor now returns VariableSaved/
CommandSaved actions. Handler toasts 'Variable saved' or
'Command saved' matching the delete toast pattern.

Co-Authored-By: Claude <noreply@anthropic.com>"
```
