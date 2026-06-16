# Working Directory Support — Design Spec

**Date:** 2026-06-17
**Status:** Approved
**Scope:** Per-command-set working directory configuration with launcher CWD as default

## Problem

All commands execute from the launcher's working directory (CWD). Users working
on multiple projects must `cd` manually or embed `cd /path/to/project` as a
command prefix. This is error-prone and inconvenient.

## Solution

Add `working_dir: Option<String>` to `CommandSet`. A new `DetailFocus::WorkDir`
focus region in the Properties block enables inline editing. If `None` (the
default), child processes inherit launcher's CWD — no behavioral change.

## Data Model

### `src/models/types.rs` — `CommandSet` field addition

```rust
pub struct CommandSet {
    // ...existing fields...
    pub working_dir: Option<String>,  // None = use launcher's CWD
    // ...existing fields...
}
```

`CommandSet::new()` initializes `working_dir: None`.

**Serde backward compatibility:** `Option<String>` with no serde attribute
defaults to `None` on missing field. Old JSON files load without error.

## Detail Focus UI

### New variant

```rust
pub enum DetailFocus {
    Name,
    Group,
    Shell,
    ExecMode,
    WorkDir,       // new — between ExecMode and Variables
    Variables,
    Commands,
}
```

### Tab cycle

Forward: `Name → Group → Shell → ExecMode → WorkDir → Variables → Commands → Name`
Backward: `Name → Commands → Variables → WorkDir → ExecMode → Shell → Group → Name`

### Editing behavior

Same pattern as `DetailFocus::Name` (inline editing, Enter commit, Esc cancel):

| Action | Not editing | Editing |
|--------|-------------|---------|
| Enter | Start editing, `workdir_input = self.set.working_dir.clone().unwrap_or("")` | Confirm: `self.set.working_dir = Some(workdir_input.content)`; if empty → `None` |
| Esc | — | Cancel: restore previous value |
| Tab | Move to next focus (commit if editing) | Commit + move focus |
| Character input | — | Append to `workdir_input` |

When `self.set.working_dir` is `None`, entering edit mode starts with an empty
TextInput. Confirming an empty string stores `None`.

### Display

In the Properties block, add a 4th row below the Mode row:

```
 WorkDir: /home/user/project
```

When `working_dir` is `None`:
```
 WorkDir: (default — launcher CWD)
```
Rendered in `theme.text_disabled` (dim style) to distinguish from an explicit path.

## State additions

`DetailScreenState` gains two fields:

```rust
pub struct DetailScreenState {
    // ...existing...
    pub workdir_editing: bool,      // tracks edit mode
    pub workdir_input: TextInput,   // buffer during editing
}
```

## Executor Changes

### `src/executor/async_executor.rs`

`spawn_shell_command` gains `working_dir: Option<&str>` parameter:

```rust
fn spawn_shell_command(shell_cmd: &ShellCommand, command: &str, working_dir: Option<&str>)
    -> std::io::Result<Child>
{
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

`execute_set` function signature adds `working_dir: Option<String>`, passes it
to `spawn_shell_command` for each command. Callers unaffected by default None
value (existing test behavior preserved).

### `src/executor/blocking.rs`

`execute_set_blocking` adds `.current_dir()` to the `Command` builder when
`working_dir` is set:

```rust
let mut child = Command::new(&shell_cmd.program)
    .arg(&shell_cmd.flag)
    .arg(&resolved)
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit());
if let Some(ref dir) = working_dir {
    child.current_dir(dir);
}
let mut child = child.spawn()...;
```

### `src/app/execution.rs`

`ExecutionManager::start` adds `working_dir: Option<String>` parameter, passes
to `execute_set`.

### `src/app.rs`

`do_execute_with` extracts `set.working_dir.clone()` and passes it through the
chain. No change to the execution flow beyond this parameter.

## Status Bar

### WorkDir focus (viewing):

```
[Enter] Edit work dir  [Tab] Next  |  [Ctrl+S] Save
```

### WorkDir focus (editing):

```
[Enter] Confirm  [Esc] Cancel — editing work directory
```

## Error Handling

No new error paths. If the path doesn't exist, the OS returns a spawn error
which is already handled by the existing `SpawnFailed` error path.

## Testing

| Module | Tests | Count |
|--------|-------|-------|
| `models/types.rs` | working_dir defaults to None in `CommandSet::new()` | 1 |
| `ui/detail_screen/handler.rs` | Enter starts editing, Enter commits, Esc cancels, Tab commits | 4 |
| `app/handler.rs` | ExecuteSet with working_dir passes it through | 1 |

Estimated total: ~120 lines production code, ~30 lines tests.

## Files Affected

| File | Change |
|------|--------|
| `src/models/types.rs` | Add `working_dir` field to `CommandSet` |
| `src/ui/detail_screen/mod.rs` | Add `DetailFocus::WorkDir` variant + state fields |
| `src/ui/detail_screen/handler.rs` | Tab cycle, Enter/Edit/Esc handler |
| `src/ui/detail_screen/render.rs` | Properties row + status bar |
| `src/executor/async_executor.rs` | `spawn_shell_command` + `execute_set` working_dir |
| `src/executor/blocking.rs` | `execute_set_blocking` working_dir |
| `src/app/execution.rs` | `ExecutionManager::start` working_dir |
| `src/app.rs` | Pass `set.working_dir` to execution chain |
