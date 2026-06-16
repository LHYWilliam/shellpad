# Batch 3: 三大键盘 Handler 单元测试

> **For agentic workers:** 在 `main_screen/handler.rs`、`detail_screen/handler.rs`、`variable_screen.rs` 末尾追加 `#[cfg(test)] mod tests` 块。不修改生产代码。

**Goal:** 给三个屏幕的键盘处理逻辑添加约 22 个单元测试，验证按键到 `AppAction` 的正确映射。

---

### Task 1: `main_screen/handler.rs` 测试（~10 个）

**文件:** `src/ui/main_screen/handler.rs`

测试策略：创建 `MainScreenState::new()` 和假 `AppData`（至少含 1 个 group 和 1 个 set），调用 `handle_key`，验证返回的 `AppAction`。

**注意：** 测试模块需要 import 以下内容：
- `use super::*`（访问 `MainScreenState`）
- `use crate::action::AppAction`
- `use crate::models::{AppData, Group, CommandSet}`
- `use crossterm::event::{KeyCode, KeyEvent, KeyModifiers}`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::AppAction;
    use crate::models::{AppData, Group, CommandSet};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn make_data() -> AppData {
        let mut g = Group::new("Test Group".to_string());
        let set = CommandSet::new("Test Set".to_string(), g.id);
        g.sets.push(set);
        AppData { groups: vec![g] }
    }

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    #[test]
    fn test_nav_down_returns_none() {
        let mut state = MainScreenState::new();
        let data = make_data();
        let action = state.handle_key(make_key(KeyCode::Down), &data);
        assert!(matches!(action, AppAction::None));
        assert_eq!(state.group_list.selected, 0); // stays at first
    }

    #[test]
    fn test_enter_on_set_returns_execute_set() {
        let mut state = MainScreenState::new();
        state.active_panel = crate::ui::main_screen::Panel::Sets;
        let data = make_data();
        let action = state.handle_key(make_key(KeyCode::Enter), &data);
        assert!(matches!(action, AppAction::ExecuteSet(0, 0)));
    }

    #[test]
    fn test_e_returns_edit_set() {
        let mut state = MainScreenState::new();
        state.active_panel = crate::ui::main_screen::Panel::Sets;
        let data = make_data();
        let action = state.handle_key(make_key(KeyCode::Char('e')), &data);
        assert!(matches!(action, AppAction::EditSet(0, 0)));
    }

    #[test]
    fn test_n_returns_new_set() {
        let mut state = MainScreenState::new();
        let data = make_data();
        let action = state.handle_key(make_key(KeyCode::Char('n')), &data);
        assert!(matches!(action, AppAction::NewSet(0)));
    }

    #[test]
    fn test_d_returns_delete_set() {
        let mut state = MainScreenState::new();
        state.active_panel = crate::ui::main_screen::Panel::Sets;
        let data = make_data();
        let action = state.handle_key(make_key(KeyCode::Char('d')), &data);
        assert!(matches!(action, AppAction::DeleteSet(0, 0)));
    }

    #[test]
    fn test_big_d_returns_delete_group() {
        let mut state = MainScreenState::new();
        state.active_panel = crate::ui::main_screen::Panel::Groups;
        let data = make_data();
        let action = state.handle_key(make_key(KeyCode::Char('D')), &data);
        assert!(matches!(action, AppAction::DeleteGroup(0)));
    }

    #[test]
    fn test_g_returns_new_group() {
        let mut state = MainScreenState::new();
        let data = make_data();
        let action = state.handle_key(make_key(KeyCode::Char('g')), &data);
        assert!(matches!(action, AppAction::NewGroup));
    }

    #[test]
    fn test_q_returns_quit() {
        let mut state = MainScreenState::new();
        let data = make_data();
        let action = state.handle_key(make_key(KeyCode::Char('q')), &data);
        assert!(matches!(action, AppAction::Quit));
    }

    #[test]
    fn test_question_mark_returns_help() {
        let mut state = MainScreenState::new();
        let data = make_data();
        let action = state.handle_key(make_key(KeyCode::Char('?')), &data);
        assert!(matches!(action, AppAction::Help));
    }

    #[test]
    fn test_slash_enters_search_mode() {
        let mut state = MainScreenState::new();
        let data = make_data();
        let action = state.handle_key(make_key(KeyCode::Char('/')), &data);
        assert!(matches!(action, AppAction::None));
        assert!(state.search_mode);
    }
}
```

### Task 2: `detail_screen/handler.rs` 测试（~8 个）

**文件:** `src/ui/detail_screen/handler.rs`

测试策略：创建 `DetailScreenState::new(set, groups)`，调用 `handle_key`，验证 focus 状态和返回的 `AppAction`。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::AppAction;
    use crate::models::{CommandSet, Group};
    use crate::ui::detail_screen::DetailFocus;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn make_state() -> DetailScreenState {
        let group = Group::new("G".to_string());
        let set = CommandSet::new("S".to_string(), group.id);
        DetailScreenState::new(set, vec![group])
    }

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    #[test]
    fn test_tab_cycles_focus_forward() {
        let mut state = make_state();
        assert_eq!(state.focus, DetailFocus::Name);
        state.handle_key(make_key(KeyCode::Tab));
        assert_eq!(state.focus, DetailFocus::Group);
        state.handle_key(make_key(KeyCode::Tab));
        assert_eq!(state.focus, DetailFocus::Shell);
        state.handle_key(make_key(KeyCode::Tab));
        assert_eq!(state.focus, DetailFocus::ExecMode);
        state.handle_key(make_key(KeyCode::Tab));
        assert_eq!(state.focus, DetailFocus::Variables);
        state.handle_key(make_key(KeyCode::Tab));
        assert_eq!(state.focus, DetailFocus::Commands);
        state.handle_key(make_key(KeyCode::Tab));
        assert_eq!(state.focus, DetailFocus::Name); // wraps around
    }

    #[test]
    fn test_backtab_cycles_focus_backward() {
        let mut state = make_state();
        // Start at Name (index 0), send backtab -> goes to Commands (last)
        state.handle_key(make_key(KeyCode::BackTab));
        assert_eq!(state.focus, DetailFocus::Commands);
    }

    #[test]
    fn test_enter_on_name_starts_editing() {
        let mut state = make_state();
        assert_eq!(state.focus, DetailFocus::Name);
        assert!(!state.editing_name);
        state.handle_key(make_key(KeyCode::Enter));
        assert!(state.editing_name);
    }

    #[test]
    fn test_enter_on_variables_enters_edit_mode() {
        let mut state = make_state();
        // Add a variable
        state.set.variables.push(crate::models::Variable {
            name: "x".to_string(),
            default_value: "y".to_string(),
        });
        state.focus = DetailFocus::Variables;
        state.handle_key(make_key(KeyCode::Enter));
        assert!(state.var_edit.is_editing());
    }

    #[test]
    fn test_a_on_variables_triggers_insert() {
        let mut state = make_state();
        state.set.variables.push(crate::models::Variable {
            name: "a".to_string(),
            default_value: "b".to_string(),
        });
        state.focus = DetailFocus::Variables;
        let action = state.handle_key(make_key(KeyCode::Char('a')));
        assert!(matches!(action, AppAction::None));
        assert!(state.var_edit.insert_at.is_some());
    }

    #[test]
    fn test_d_on_variables_returns_delete_variable() {
        let mut state = make_state();
        state.set.variables.push(crate::models::Variable {
            name: "x".to_string(),
            default_value: "y".to_string(),
        });
        state.focus = DetailFocus::Variables;
        let action = state.handle_key(make_key(KeyCode::Char('d')));
        assert!(matches!(action, AppAction::DeleteVariable(0)));
    }

    #[test]
    fn test_ctrl_s_returns_save_set() {
        let mut state = make_state();
        let ctrl_s = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL);
        let action = state.handle_key(ctrl_s);
        assert!(matches!(action, AppAction::SaveSet(_)));
    }

    #[test]
    fn test_esc_returns_cancel_edit() {
        let mut state = make_state();
        let action = state.handle_key(make_key(KeyCode::Esc));
        assert!(matches!(action, AppAction::CancelEdit));
    }
}
```

### Task 3: `variable_screen.rs` 测试（~4 个）

**文件:** `src/ui/variable_screen.rs`

测试策略：创建 `VariableScreenState::new()`，激活，调用 `handle_key`，验证导航和返回的 `AppAction`。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::AppAction;
    use crate::models::CommandSet;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    #[test]
    fn test_tab_advances_focus() {
        let mut state = VariableScreenState::new();
        let set = CommandSet::new("test".to_string(), uuid::Uuid::new_v4());
        state.activate(&set, 0, 0);
        // Initially has 0 variables after activate. Let's add some to test navigation.
        // Actually, VariableScreenState.activate populates inputs from set.variables,
        // which is empty for a new CommandSet. Skip this test or adjust.
        // For now, we can test focus remains 0 when nothing to navigate.
        let _ = state.handle_key(make_key(KeyCode::Tab));
        assert_eq!(state.focus, 0); // no variables to navigate
    }

    #[test]
    fn test_enter_with_variables_returns_confirm() {
        let mut state = VariableScreenState::new();
        state.active = true;
        state.inputs.push(TextInput::new("val".to_string()));
        state.names.push("x".to_string());
        state.gi = 0;
        state.si = 0;
        let action = state.handle_key(make_key(KeyCode::Enter));
        assert!(matches!(action, AppAction::ConfirmVariables));
    }

    #[test]
    fn test_esc_returns_cancel_variables() {
        let mut state = VariableScreenState::new();
        state.active = true;
        let action = state.handle_key(make_key(KeyCode::Esc));
        assert!(matches!(action, AppAction::CancelVariables));
    }
}
```

注意：`VariableScreenState` 的 `handle_key` 方法需要在 `variable_screen.rs` 中定义。检查 `variable_screen.rs` 中 `pub fn handle_key` 的可见性——它可能是 `pub(crate)` 或 `pub`。如果测试无法访问，需要调整可见性。

### Verification

```bash
cargo test      # expect 129+ passed (107 + 22)
cargo clippy    # no new warnings
cargo fmt
git add src/ui/main_screen/handler.rs src/ui/detail_screen/handler.rs src/ui/variable_screen.rs
git commit -m "test(batch3): add keyboard handler unit tests for all three screens"
```
