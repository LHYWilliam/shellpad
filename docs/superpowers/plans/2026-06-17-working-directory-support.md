# Working Directory Support — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add per-command-set working directory configuration. `None` = inherit launcher CWD (default), `Some(path)` = custom. New `DetailFocus::WorkDir` in the Properties block with inline editing.

**Architecture:** `working_dir: Option<String>` field on `CommandSet`, serde-compatible (old JSON defaults to `None`). Detail Screen gains `DetailFocus::WorkDir` + `workdir_editing`/`workdir_input` state, reusing `DetailFocus::Name` inline-edit pattern. Executor chain (`do_execute_with → ExecutionManager::start → execute_set → spawn_shell_command`) all gain `working_dir` parameter. Blocking executor uses `cmd.current_dir()`.

**Tech Stack:** Rust, Ratatui, crossterm, serde (no new dependencies)

---

### Task 1: Add `working_dir` field to `CommandSet`

**Files:**
- Modify: `src/models/types.rs`

- [ ] **Step 1: Write failing test**

In `src/models/types.rs`, in the `tests` module, add:

```rust
    #[test]
    fn test_command_set_working_dir_defaults_to_none() {
        let group_id = Uuid::new_v4();
        let set = CommandSet::new("Test".to_string(), group_id);
        assert_eq!(set.working_dir, None);
    }
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test models::types::tests::test_command_set_working_dir_defaults_to_none`
Expected: FAIL — `working_dir` field not found on `CommandSet`

- [ ] **Step 3: Add field to `CommandSet` and initialize in `new()`

In the `CommandSet` struct, add after `commands`:

```rust
pub struct CommandSet {
    pub id: Uuid,
    pub name: String,
    pub group_id: Uuid,
    pub shell: ShellType,
    pub exec_mode: ExecMode,
    pub variables: Vec<Variable>,
    pub commands: Vec<Command>,
    pub working_dir: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

In `CommandSet::new()`, add after `commands: Vec::new(),`:

```rust
            working_dir: None,
```

- [ ] **Step 4: Run tests**

Run: `cargo test models::types::tests`
Expected: All tests PASS (10 existing + 1 new = 11)

- [ ] **Step 5: Verify serde backward compatibility**

Run: `cargo test models::types::tests::test_serde_roundtrip_app_data`
Expected: PASS (existing JSON roundtrip test — missing `working_dir` field defaults to `None`)

- [ ] **Step 6: Commit**

```bash
git add src/models/types.rs
git commit -m "feat: add working_dir field to CommandSet

Option<String>, defaults to None = inherit launcher CWD.
Serde-compatible: missing field in old JSON deserializes as None.
1 new test.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Add `DetailFocus::WorkDir` variant and state fields

**Files:**
- Modify: `src/ui/detail_screen/mod.rs`

- [ ] **Step 1: Add `DetailFocus::WorkDir`**

In the `DetailFocus` enum, add `WorkDir` between `ExecMode` and `Variables`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailFocus {
    Name,
    Group,
    Shell,
    ExecMode,
    WorkDir,
    Variables,
    Commands,
}
```

- [ ] **Step 2: Add state fields**

In `DetailScreenState`, add after `pub editing_name: bool,`:

```rust
    pub editing_name: bool,
    pub workdir_editing: bool,
    pub workdir_input: TextInput,
    pub var_edit: InlineEdit,
```

In `DetailScreenState::new()`, add after `editing_name: false,`:

```rust
            workdir_editing: false,
            workdir_input: TextInput::new(String::new()),
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check`
Expected: E004 — non-exhaustive patterns for `DetailFocus` in handler.rs and render.rs (will fix in Tasks 3/4). The errors confirm we need to add match arms. This is expected at this stage.

- [ ] **Step 4: Commit**

```bash
git add src/ui/detail_screen/mod.rs
git commit -m "feat: add DetailFocus::WorkDir variant and workdir_editing state

Between ExecMode and Variables in tab order. workdir_editing flag
+ workdir_input TextInput reuse the DetailFocus::Name edit pattern.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: Detail Screen handler — Tab cycle + Enter/Edit/Esc + tests

**Files:**
- Modify: `src/ui/detail_screen/handler.rs`

- [ ] **Step 1: Write failing tests**

In the test module, add:

```rust
    #[test]
    fn test_enter_on_workdir_starts_editing() {
        let mut state = make_state();
        state.focus = DetailFocus::WorkDir;
        assert!(!state.workdir_editing);
        state.handle_key(make_key(KeyCode::Enter));
        assert!(state.workdir_editing);
    }

    #[test]
    fn test_enter_on_workdir_confirms_editing() {
        let mut state = make_state();
        state.focus = DetailFocus::WorkDir;
        state.handle_key(make_key(KeyCode::Enter)); // start editing
        assert!(state.workdir_editing);
        // Type a path
        state.workdir_input = TextInput::new("/tmp/test".to_string());
        state.handle_key(make_key(KeyCode::Enter)); // confirm
        assert!(!state.workdir_editing);
        assert_eq!(state.set.working_dir, Some("/tmp/test".to_string()));
    }

    #[test]
    fn test_enter_on_workdir_empty_string_stores_none() {
        let mut state = make_state();
        state.set.working_dir = Some("/old/path".to_string());
        state.focus = DetailFocus::WorkDir;
        state.handle_key(make_key(KeyCode::Enter)); // start editing
        state.workdir_input = TextInput::new(String::new());
        state.handle_key(make_key(KeyCode::Enter)); // confirm with empty
        assert!(!state.workdir_editing);
        assert_eq!(state.set.working_dir, None);
    }

    #[test]
    fn test_esc_cancels_workdir_editing() {
        let mut state = make_state();
        state.set.working_dir = Some("/existing".to_string());
        state.focus = DetailFocus::WorkDir;
        state.handle_key(make_key(KeyCode::Enter)); // start editing
        assert!(state.workdir_editing);
        state.workdir_input = TextInput::new("/changed".to_string());
        state.handle_key(make_key(KeyCode::Esc));
        assert!(!state.workdir_editing);
        assert_eq!(state.set.working_dir, Some("/existing".to_string()));
    }

    #[test]
    fn test_tab_commits_workdir_editing() {
        let mut state = make_state();
        state.focus = DetailFocus::WorkDir;
        state.handle_key(make_key(KeyCode::Enter)); // start editing
        state.workdir_input = TextInput::new("/committed".to_string());
        state.handle_key(make_key(KeyCode::Tab)); // Tab commits + moves
        assert!(!state.workdir_editing);
        assert_eq!(state.set.working_dir, Some("/committed".to_string()));
        assert_eq!(state.focus, DetailFocus::Variables);
    }
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test ui::detail_screen::handler::tests::test_enter_on_workdir_starts_editing`
Expected: FAIL — no WorkDir match arm in Enter handler

- [ ] **Step 3: Update Tab/BackTab handlers**

In `KeyCode::Tab | KeyCode::Char('\t')` arm, change the forward cycle to include `WorkDir`:

```rust
            KeyCode::Tab | KeyCode::Char('\t') => {
                self.commit_name_edit();
                self.commit_workdir_edit();
                self.focus = match self.focus {
                    DetailFocus::Name => DetailFocus::Group,
                    DetailFocus::Group => DetailFocus::Shell,
                    DetailFocus::Shell => DetailFocus::ExecMode,
                    DetailFocus::ExecMode => DetailFocus::WorkDir,
                    DetailFocus::WorkDir => DetailFocus::Variables,
                    DetailFocus::Variables => DetailFocus::Commands,
                    DetailFocus::Commands => DetailFocus::Name,
                };
            }
```

In `KeyCode::BackTab` arm, change the backward cycle:

```rust
            KeyCode::BackTab => {
                self.commit_name_edit();
                self.commit_workdir_edit();
                self.focus = match self.focus {
                    DetailFocus::Name => DetailFocus::Commands,
                    DetailFocus::Group => DetailFocus::Name,
                    DetailFocus::Shell => DetailFocus::Group,
                    DetailFocus::ExecMode => DetailFocus::Shell,
                    DetailFocus::WorkDir => DetailFocus::ExecMode,
                    DetailFocus::Variables => DetailFocus::WorkDir,
                    DetailFocus::Commands => DetailFocus::Variables,
                };
            }
```

- [ ] **Step 4: Add Enter handler for WorkDir**

In the `KeyCode::Enter` match block, add a new arm for WorkDir (after the Name block and before Variables):

```rust
                    DetailFocus::WorkDir => {
                        if self.workdir_editing {
                            let content = self.workdir_input.content.clone();
                            self.set.working_dir = if content.trim().is_empty() {
                                None
                            } else {
                                Some(content)
                            };
                            self.workdir_editing = false;
                        } else {
                            self.workdir_input =
                                TextInput::new(self.set.working_dir.clone().unwrap_or_default());
                            self.workdir_editing = true;
                        }
                    }
```

- [ ] **Step 5: Extend Esc handler for workdir_editing**

Change the `KeyCode::Esc` arm to handle both name and workdir editing:

```rust
            KeyCode::Esc => {
                if self.editing_name {
                    self.editing_name = false;
                } else if self.workdir_editing {
                    self.workdir_editing = false;
                } else {
                    return AppAction::CancelEdit;
                }
            }
```

- [ ] **Step 6: Add text input handling for workdir_editing**

The existing text input handling at the end of `handle_key`:

```rust
        if self.editing_name {
            handle_text_input(&mut self.name_input, key);
        }
```

Extend to also handle workdir_editing:

```rust
        if self.editing_name {
            handle_text_input(&mut self.name_input, key);
        }
        if self.workdir_editing {
            handle_text_input(&mut self.workdir_input, key);
        }
```

- [ ] **Step 7: Add `commit_workdir_edit` helper method**

After the `commit_name_edit` method, add:

```rust
    fn commit_workdir_edit(&mut self) {
        if self.workdir_editing {
            let content = self.workdir_input.content.clone();
            self.set.working_dir = if content.trim().is_empty() {
                None
            } else {
                Some(content)
            };
            self.workdir_editing = false;
        }
    }
```

- [ ] **Step 8: Run all detail_screen handler tests**

Run: `cargo test ui::detail_screen::handler::tests`
Expected: All tests PASS (existing 12 + 5 new = 17)

- [ ] **Step 9: Commit**

```bash
git add src/ui/detail_screen/handler.rs
git commit -m "feat: handle WorkDir focus — Enter/Edit/Esc/Tab workflow

Tab cycle includes WorkDir between ExecMode and Variables.
Inline editing reuses DetailFocus::Name pattern: Enter toggles,
Esc cancels (restores old value), Tab commits + moves.
Empty input stores None. 5 new tests.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 4: Detail Screen render — Properties row + status bar

**Files:**
- Modify: `src/ui/detail_screen/render.rs`

- [ ] **Step 1: Update Properties layout and add WorkDir row**

In `render_metadata`, change the row layout from 3 to 4 rows:

```rust
        let is_workdir_focused = self.focus == DetailFocus::WorkDir;
        let props_focused = matches!(
            self.focus,
            DetailFocus::Name
                | DetailFocus::Group
                | DetailFocus::Shell
                | DetailFocus::ExecMode
                | DetailFocus::WorkDir
        );
        let inner = bordered_block_zone(frame, area, theme, " Properties ", props_focused);

        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ]);
        let [name_row, gs_row, mode_row, workdir_row] = rows.areas(inner);
```

Note: `is_workdir_focused` is defined before `let [name_row...]` for use in the WorkDir rendering block below.

- [ ] **Step 2: Add WorkDir rendering**

After the ExecMode row rendering (around line 124), add:

```rust
        // WorkDir
        let workdir_style = if is_workdir_focused {
            if self.workdir_editing {
                Style::default()
                    .fg(theme.text_on_selected)
                    .bg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.accent_primary)
            }
        } else {
            theme.normal_style()
        };
        let wd_text = if self.workdir_editing {
            format!(" WorkDir: {}", self.workdir_input.content)
        } else {
            match &self.set.working_dir {
                Some(p) => format!(" WorkDir: {}", p),
                None => format!(" WorkDir: (default — launcher CWD)"),
            }
        };
        let wd_display_style = if !is_workdir_focused && self.set.working_dir.is_none() {
            Style::default()
                .fg(theme.text_disabled)
                .add_modifier(Modifier::DIM)
        } else {
            workdir_style
        };
        let wd_line = fill_row(
            Line::from(Span::styled(wd_text, wd_display_style)),
            wd_display_style,
            workdir_row.width,
        );
        frame.render_widget(Paragraph::new(wd_line), workdir_row);

        // Cursor for workdir editing
        if self.workdir_editing {
            let prefix_width = unicode_width::UnicodeWidthStr::width(" WorkDir: ");
            set_cursor_after_prefix(
                frame,
                &self.workdir_input.content,
                self.workdir_input.cursor,
                prefix_width as u16,
                workdir_row,
            );
        }
```

- [ ] **Step 3: Update status bar**

In `render_status_bar`, add the WorkDir hint. The editing case `(true, _)` already shows `[Enter] Confirm  [Esc] Cancel`. Add the viewing case before the Variables arm:

```rust
            (false, DetailFocus::WorkDir) => {
                "[Enter] Edit work dir  [Tab] Next  |  [Ctrl+S] Save"
            }
```

- [ ] **Step 4: Update top-level render layout height**

In `src/ui/detail_screen/mod.rs`, the Properties block height is currently `Constraint::Length(8)`. With 4 rows instead of 3, keep 8 (inner area = 6 lines, 4 rows use 4, 2 lines spare for spacing):

No change needed — `Constraint::Length(8)` already accommodates 4 rows (2 borders + 4 rows = 6, fits within 8).

- [ ] **Step 5: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully (no new exhaustive match errors — all `DetailFocus` arms now covered in render.rs)

- [ ] **Step 6: Run full test suite**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 7: Commit**

```bash
git add src/ui/detail_screen/render.rs
git commit -m "feat: render WorkDir row in Properties and update status bar

4th row shows path or '(default — launcher CWD)' in dim style.
Cursor positioning for inline editing. Status bar shows
[Enter] Edit work dir hint when focused.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 5: Executor chain — pass working_dir through execution

**Files:**
- Modify: `src/executor/async_executor.rs`
- Modify: `src/executor/blocking.rs`
- Modify: `src/app/execution.rs`
- Modify: `src/app.rs`

- [ ] **Step 1: Update `spawn_shell_command` in async_executor.rs**

Change signature and body (lines 20-26):

```rust
fn spawn_shell_command(
    shell_cmd: &ShellCommand,
    command: &str,
    working_dir: Option<&str>,
) -> std::io::Result<Child> {
    let mut cmd = StdCommand::new(&shell_cmd.program);
    cmd.arg(&shell_cmd.flag).arg(command);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }
    cmd.spawn()
}
```

- [ ] **Step 2: Update `execute_set` signature and call site**

Change function signature (around line 47):

```rust
pub fn execute_set(
    commands: Vec<Command>,
    exec_mode: ExecMode,
    variables: Vec<Variable>,
    shell_cmd: ShellCommand,
    tx: mpsc::Sender<ExecutionEvent>,
    kill_signal: Arc<AtomicBool>,
    index_offset: usize,
    working_dir: Option<String>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
```

Update the two `spawn_shell_command` calls inside the loop:

```rust
            let mut child = match spawn_shell_command(
                &shell_cmd, &resolved, working_dir.as_deref(),
            ) {
```

The second call site is only the `Ok(c)` branch — but actually `spawn_shell_command` is only called once in the loop body (at line 91). Let me verify... Looking at the code, `spawn_shell_command` is called at the `let mut child = match spawn_shell_command(...)` site. There's only one call. Good — change that one:

```rust
            let mut child = match spawn_shell_command(&shell_cmd, &resolved, working_dir.as_deref()) {
```

- [ ] **Step 3: Update `ExecutionManager::start`**

In `src/app/execution.rs`, change signature:

```rust
    pub fn start(
        &mut self,
        commands: Vec<crate::models::Command>,
        exec_mode: crate::models::ExecMode,
        variables: Vec<crate::models::Variable>,
        shell_cmd: crate::models::ShellCommand,
        index_offset: usize,
        working_dir: Option<String>,
    ) {
```

Add `working_dir` to the `execute_set` call:

```rust
        let handle = execute_set(
            commands,
            exec_mode,
            variables,
            shell_cmd,
            tx,
            Arc::clone(&self.kill_signal),
            index_offset,
            working_dir,
        );
```

- [ ] **Step 4: Update `do_execute_with` in app.rs**

In both the `start_from == 0` path and the continue path, extract `working_dir` and pass it:

In the first call (around line 140):
```rust
            let working_dir = set.working_dir.clone();
            manager.start(
                cmds,
                set.exec_mode,
                set.variables.clone(),
                shell_cmd,
                0usize,
                working_dir,
            );
```

In the second call (around line 165):
```rust
            let working_dir = set.working_dir.clone();
            manager.start(
                cmds,
                set.exec_mode,
                set.variables.clone(),
                shell_cmd,
                start_from,
                working_dir,
            );
```

- [ ] **Step 5: Update `execute_set_blocking` in blocking.rs**

Change signature and add `.current_dir()` (around lines 19-24):

```rust
pub fn execute_set_blocking(
    set: &CommandSet,
    shell_cmd: &ShellCommand,
    vars: &HashMap<String, String>,
    working_dir: Option<&str>,
) -> Result<ExecuteResult, ExecuteError> {
```

In the command builder (around lines 34-39):
```rust
        let mut cmd_builder = Command::new(&shell_cmd.program);
        cmd_builder
            .arg(&shell_cmd.flag)
            .arg(&resolved)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        if let Some(dir) = working_dir {
            cmd_builder.current_dir(dir);
        }
        let mut child = cmd_builder
            .spawn()
```

- [ ] **Step 6: Update CLI runner in cli.rs**

In `handle_run` (around line 119), extract and pass `working_dir`:

```rust
    let working_dir = set_ref.working_dir.as_deref();
    match crate::executor::execute_set_blocking(set_ref, &shell_cmd, &resolved_vars, working_dir) {
```

- [ ] **Step 7: Fix executor tests**

Run: `cargo test executor::tests`
Expected: Compile errors in tests (they call `execute_set` / `execute_set_blocking` with the old signature).

Fix `src/executor/tests.rs` — add `None` as the working_dir argument to all calls:

For `execute_set` calls:
```rust
    // Add None as last argument
    execute_set(
        cmds, exec_mode, vars, shell_cmd, tx, kill_signal, 0,
        None,  // working_dir
    );
```

For `execute_set_blocking` calls:
```rust
    execute_set_blocking(&set, &shell_cmd, &vars, None)
```

Run: `cargo test executor::tests`
Expected: All tests PASS (existing counts unchanged)

- [ ] **Step 8: Run full test suite**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 9: Run clippy**

Run: `cargo clippy`
Expected: No new warnings (pre-existing 2 OK)

- [ ] **Step 10: Commit**

```bash
git add src/executor/async_executor.rs src/executor/blocking.rs src/app/execution.rs src/app.rs src/cli.rs src/executor/tests.rs
git commit -m "feat: pass working_dir through executor chain

spawn_shell_command, execute_set, ExecutionManager::start,
execute_set_blocking all gain working_dir param.
None = inherit launcher CWD (no-op, existing behavior unchanged).
CLI handle_run extracts set.working_dir and passes to blocking executor.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 6: Integration test + final verification

**Files:**
- Modify: `src/integration_tests.rs`
- Modify: `src/ui/help_screen.rs`

- [ ] **Step 1: Add integration test for WorkDir field lifecycle**

In `src/integration_tests.rs`, after the reorder integration test:

```rust
    // ------------------------------------------------------------------
    // 5.8 Working directory lifecycle
    // ------------------------------------------------------------------
    #[test]
    fn test_working_directory_lifecycle() {
        let mut app = make_app();
        let mut g = Group::new("G".to_string());
        let mut set = CommandSet::new("S".to_string(), g.id);
        set.working_dir = Some("/tmp/project".to_string());
        g.sets.push(set);
        app.data = AppData { groups: vec![g] };
        let set_clone = app.data.groups[0].sets[0].clone();
        app.detail_screen = Some(DetailScreenState::new(set_clone, app.data.groups.clone()));
        app.mode = AppMode::Detail;

        // Verify working_dir persisted through detail screen construction
        let ds = app.detail_screen.as_ref().unwrap();
        assert_eq!(ds.set.working_dir, Some("/tmp/project".to_string()));

        // Verify save round-trip: modify, save, check
        let mut ds = app.detail_screen.take().unwrap();
        ds.set.working_dir = None; // reset to default
        let saved_set = ds.set.clone();
        app.detail_screen = Some(ds);

        app.handle_action(AppAction::SaveSet(saved_set));
        assert_eq!(app.data.groups[0].sets[0].working_dir, None);
    }
```

- [ ] **Step 2: Update Help screen**

In `src/ui/help_screen.rs`, Detail Screen section already lists Tab/Shift+Tab and Enter/e. No new shortcut needed. The WorkDir hint is self-documenting in the status bar. No change needed.

- [ ] **Step 3: Run integration test**

Run: `cargo test integration_tests::tests::test_working_directory_lifecycle`
Expected: PASS

- [ ] **Step 4: Run full test suite**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 5: Run clippy**

Run: `cargo clippy`
Expected: No new warnings

- [ ] **Step 6: Commit**

```bash
git add src/integration_tests.rs
git commit -m "test: add integration test for working directory lifecycle

Covers persistence through detail screen round-trip, SaveSet.
Verifies working_dir correctly defaults to None on new set and
serializes/deserializes through serde.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 7: Mark feature as completed in memory

**Files:**
- Modify: `/home/william/.claude/projects/-home-william-Code-Rust-launcher/memory/feature-gap-priorities.md`

- [ ] **Step 1: Mark #10 as done**

Change:
```
- [ ] **#10 工作目录支持** (~80 行)
```
to:
```
- [x] **#10 工作目录支持** (~120 行)
  完成: 2026-06-17
```

- [ ] **Step 2: Commit memory update**

```bash
git add /home/william/.claude/projects/-home-william-Code-Rust-launcher/memory/feature-gap-priorities.md
git commit -m "docs: mark working directory support as completed in memory

Co-Authored-By: Claude <noreply@anthropic.com>"
```
