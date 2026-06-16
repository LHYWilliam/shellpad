use crate::ui::widget::scrollable_list::ScrollableList;
use crate::ui::widget::text_input::{TextInput, handle_text_input};

/// Generic inline text-edit state for a list.
#[derive(Clone)]
pub struct InlineEdit {
    pub editing: Option<usize>, // index of item being edited (or None)
    pub edit_input: TextInput,
    pub insert_at: Option<usize>, // Some(pos) = inserting new item at pos
}

impl InlineEdit {
    pub fn new() -> Self {
        Self {
            editing: None,
            edit_input: TextInput::new(String::new()),
            insert_at: None,
        }
    }

    pub fn is_editing(&self) -> bool {
        self.editing.is_some()
    }

    /// Commit the edit, either inserting at `insert_at` position or replacing at `idx`.
    pub fn commit<T>(
        &mut self,
        idx: usize,
        items: &mut Vec<T>,
        new_item: T,
        list: &mut ScrollableList,
    ) {
        if let Some(insert_pos) = self.insert_at.take() {
            items.insert(insert_pos, new_item);
            list.selected = insert_pos;
        } else {
            items[idx] = new_item;
            list.selected = idx;
        }
    }

    /// Cancel the current edit.
    pub fn cancel(&mut self) {
        self.insert_at = None;
        self.editing = None;
    }

    /// Handle a plain text key event.
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        handle_text_input(&mut self.edit_input, key);
    }

    /// Handle a key event with an optional prefix-protection byte position.
    /// If `protect` is Some(pos), Backspace/Delete/Left are blocked when
    /// the cursor is at or before `pos`.
    pub fn handle_key_protected(
        &mut self,
        key: crossterm::event::KeyEvent,
        protect: Option<usize>,
    ) {
        use crossterm::event::KeyCode;
        let guard = protect.unwrap_or(0);
        match key.code {
            KeyCode::Backspace => {
                if self.edit_input.cursor > guard {
                    self.edit_input.delete_before();
                }
            }
            KeyCode::Delete => {
                if self.edit_input.cursor > guard {
                    self.edit_input.delete_at();
                }
            }
            KeyCode::Left => {
                if self.edit_input.cursor > guard {
                    self.edit_input.move_cursor_left();
                }
            }
            KeyCode::Right => self.edit_input.move_cursor_right(),
            KeyCode::Home => self.edit_input.move_cursor_to_start(),
            KeyCode::End => self.edit_input.move_cursor_to_end(),
            _ => {
                handle_text_input(&mut self.edit_input, key);
            }
        }
    }
}

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
        // commit() does NOT clear editing — caller does that
        assert!(edit.editing.is_some());
        assert!(edit.insert_at.is_none());
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
        // commit() does NOT clear editing — caller does that
        assert!(edit.editing.is_some());
        assert!(edit.insert_at.is_none()); // insert_at IS cleared by commit
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
