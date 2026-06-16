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
