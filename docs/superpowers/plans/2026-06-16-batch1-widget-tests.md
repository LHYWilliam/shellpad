# Batch 1: Widget 三件套单元测试

> **For agentic workers:** 在 3 个 widget 文件末尾追加 `#[cfg(test)] mod tests` 块。不修改任何生产代码。每完成一个文件后运行 `cargo test` 确认通过。

**Goal:** 给 `TextInput`、`ScrollableList`、`InlineEdit` 三个 widget 添加合计 ~26 个单元测试。

**方法:** 所有测试为 `#[cfg(test)] mod tests` 块，直接追加在文件末尾。测试纯函数逻辑，不涉及 terminal/UI。

---

### Task 1: `text_input.rs` 测试（~12 个）

**文件:** `src/ui/widget/text_input.rs` — 在文件末尾追加

```rust
#[cfg(test)]
mod tests {
    use super::TextInput;

    #[test]
    fn test_new_empty() {
        let input = TextInput::new(String::new());
        assert!(input.content.is_empty());
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn test_new_with_content() {
        let input = TextInput::new("hello".to_string());
        assert_eq!(input.content, "hello");
        assert_eq!(input.cursor, 5);
    }

    #[test]
    fn test_insert_char_middle() {
        let mut input = TextInput::new("ab".to_string());
        input.cursor = 1; // between 'a' and 'b'
        input.insert_char('x');
        assert_eq!(input.content, "axb");
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn test_insert_char_at_end() {
        let mut input = TextInput::new("abc".to_string());
        input.insert_char('d');
        assert_eq!(input.content, "abcd");
        assert_eq!(input.cursor, 4);
    }

    #[test]
    fn test_insert_char_empty() {
        let mut input = TextInput::new(String::new());
        input.insert_char('a');
        assert_eq!(input.content, "a");
        assert_eq!(input.cursor, 1);
    }

    #[test]
    fn test_delete_before_middle() {
        let mut input = TextInput::new("abcd".to_string());
        input.cursor = 3; // after 'c', before 'd'
        input.delete_before();
        assert_eq!(input.content, "abd");
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn test_delete_before_at_start() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor = 0;
        input.delete_before();
        assert_eq!(input.content, "abc"); // unchanged
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn test_delete_at_middle() {
        let mut input = TextInput::new("abcd".to_string());
        input.cursor = 2; // at 'c'
        input.delete_at();
        assert_eq!(input.content, "abd");
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn test_delete_at_end() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor = 3; // past end
        input.delete_at();
        assert_eq!(input.content, "abc"); // unchanged
        assert_eq!(input.cursor, 3);
    }

    #[test]
    fn test_move_cursor_left() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor = 2;
        input.move_cursor_left();
        assert_eq!(input.cursor, 1);
    }

    #[test]
    fn test_move_cursor_left_at_start() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor = 0;
        input.move_cursor_left();
        assert_eq!(input.cursor, 0); // clamped
    }

    #[test]
    fn test_move_cursor_right() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor = 1;
        input.move_cursor_right();
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn test_move_cursor_right_at_end() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor = 3;
        input.move_cursor_right();
        assert_eq!(input.cursor, 3); // clamped
    }

    #[test]
    fn test_move_cursor_to_start() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor = 2;
        input.move_cursor_to_start();
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn test_move_cursor_to_end() {
        let mut input = TextInput::new("abc".to_string());
        input.cursor = 0;
        input.move_cursor_to_end();
        assert_eq!(input.cursor, 3);
    }

    #[test]
    fn test_clear() {
        let mut input = TextInput::new("hello".to_string());
        input.cursor = 3;
        input.clear();
        assert!(input.content.is_empty());
        assert_eq!(input.cursor, 0);
    }
}
```

### Task 2: `scrollable_list.rs` 测试（~8 个）

**文件:** `src/ui/widget/scrollable_list.rs` — 在文件末尾追加

```rust
#[cfg(test)]
mod tests {
    use super::ScrollableList;

    #[test]
    fn test_new() {
        let list = ScrollableList::new();
        assert_eq!(list.selected, 0);
        assert_eq!(list.offset, 0);
    }

    #[test]
    fn test_select_next_within_bounds() {
        let mut list = ScrollableList::new();
        list.select_next(5);
        assert_eq!(list.selected, 1);
    }

    #[test]
    fn test_select_next_at_end() {
        let mut list = ScrollableList::new();
        list.selected = 4;
        list.select_next(5);
        assert_eq!(list.selected, 4); // clamped at last
    }

    #[test]
    fn test_select_next_empty() {
        let mut list = ScrollableList::new();
        list.select_next(0);
        assert_eq!(list.selected, 0); // no change
    }

    #[test]
    fn test_select_previous() {
        let mut list = ScrollableList::new();
        list.selected = 3;
        list.select_previous();
        assert_eq!(list.selected, 2);
    }

    #[test]
    fn test_select_previous_at_start() {
        let mut list = ScrollableList::new();
        list.selected = 0;
        list.select_previous();
        assert_eq!(list.selected, 0); // clamped at 0
    }

    #[test]
    fn test_update_offset_scroll_down() {
        let mut list = ScrollableList::new();
        list.selected = 10;
        list.update_offset(5); // visible_height = 5
        // selected (10) >= offset (0) + 5 => offset = 10 + 1 - 5 = 6
        assert_eq!(list.offset, 6);
    }

    #[test]
    fn test_update_offset_scroll_up() {
        let mut list = ScrollableList::new();
        list.offset = 8;
        list.selected = 5;
        list.update_offset(5);
        // selected (5) < offset (8) => offset = selected = 5
        assert_eq!(list.offset, 5);
    }

    #[test]
    fn test_update_offset_no_scroll_needed() {
        let mut list = ScrollableList::new();
        list.offset = 3;
        list.selected = 5;
        list.update_offset(5);
        // selected (5) within [offset(3), offset+vis(8))
        assert_eq!(list.offset, 3);
    }

    #[test]
    fn test_clamp_selected_after_deletion() {
        let mut list = ScrollableList::new();
        list.selected = 4;
        list.clamp_selected(4); // len=4, indices 0-3
        assert_eq!(list.selected, 3); // clamped to last
    }

    #[test]
    fn test_selected_or_none_empty() {
        let list = ScrollableList::new();
        assert_eq!(list.selected_or_none(0), None);
    }

    #[test]
    fn test_selected_or_none_non_empty() {
        let list = ScrollableList::new();
        assert_eq!(list.selected_or_none(5), Some(0));
    }

    #[test]
    fn test_reset() {
        let mut list = ScrollableList::new();
        list.selected = 10;
        list.offset = 5;
        list.reset();
        assert_eq!(list.selected, 0);
        assert_eq!(list.offset, 0);
    }
}
```

### Task 3: `inline_edit.rs` 测试（~6 个）

**文件:** `src/ui/widget/inline_edit.rs` — 在文件末尾追加

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widget::scrollable_list::ScrollableList;

    #[test]
    fn test_new() {
        let edit = InlineEdit::new();
        assert!(edit.editing.is_none());
        assert!(edit.insert_at.is_none());
        assert!(edit.edit_input.content.is_empty());
    }

    #[test]
    fn test_is_editing() {
        let mut edit = InlineEdit::new();
        assert!(!edit.is_editing());
        edit.editing = Some(0);
        assert!(edit.is_editing());
    }

    #[test]
    fn test_commit_replace() {
        let mut edit = InlineEdit::new();
        edit.editing = Some(1);
        let mut items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let mut list = ScrollableList::new();
        edit.commit(1, &mut items, "x".to_string(), &mut list);
        assert_eq!(items, vec!["a", "x", "c"]);
        assert_eq!(list.selected, 1);
        assert!(edit.editing.is_none());
    }

    #[test]
    fn test_commit_insert() {
        let mut edit = InlineEdit::new();
        edit.editing = Some(3);
        edit.insert_at = Some(1);
        let mut items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let mut list = ScrollableList::new();
        edit.commit(3, &mut items, "x".to_string(), &mut list);
        assert_eq!(items, vec!["a", "x", "b", "c"]);
        assert_eq!(list.selected, 1);
        assert!(edit.editing.is_none());
    }

    #[test]
    fn test_cancel() {
        let mut edit = InlineEdit::new();
        edit.editing = Some(0);
        edit.insert_at = Some(1);
        edit.cancel();
        assert!(edit.editing.is_none());
        assert!(edit.insert_at.is_none());
    }

    #[test]
    fn test_handle_key_protected_blocks_backspace() {
        let mut edit = InlineEdit::new();
        edit.edit_input = TextInput::new("ab=cd".to_string());
        edit.edit_input.cursor = 4; // after 'c', before 'd'
        // protect pos 3 (the '='), cursor at 4 > 3, delete_before works
        // First move cursor to pos 3
        edit.edit_input.cursor = 3; // at '='
        let key = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Backspace,
            crossterm::event::KeyModifiers::empty(),
        );
        edit.handle_key_protected(key, Some(3));
        // cursor at protect boundary -> no deletion
        assert_eq!(edit.edit_input.content, "ab=cd");
    }
}
```

### Verification

每完成一个文件的测试后运行：

```bash
cargo test 2>&1 | tail -10
```

全部完成后：
```bash
cargo test      # expect 86+ passed (60 original + 26 new)
cargo clippy    # expect no new warnings
```

### Commit

```bash
git add src/ui/widget/
git commit -m "test(batch1): add TextInput, ScrollableList, InlineEdit unit tests"
```
