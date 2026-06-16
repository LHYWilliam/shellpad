pub struct ScrollableList {
    pub selected: usize,
    pub offset: usize,
}

impl ScrollableList {
    pub fn new() -> Self {
        Self {
            selected: 0,
            offset: 0,
        }
    }

    pub fn select_previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn select_next(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        if self.selected + 1 < len {
            self.selected += 1;
        }
    }

    /// Ensure selected item is visible, adjust offset if needed.
    pub fn update_offset(&mut self, visible_height: usize) {
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + visible_height {
            self.offset = self
                .selected
                .saturating_add(1)
                .saturating_sub(visible_height);
        }
    }

    pub fn reset(&mut self) {
        self.selected = 0;
        self.offset = 0;
    }

    /// Clamp `selected` after a deletion: if the last item was removed,
    /// move selection to the new last item; otherwise keep it.
    pub fn clamp_selected(&mut self, len: usize) {
        if self.selected >= len {
            self.selected = len.saturating_sub(1);
        }
    }

    /// Return `Some(selected)` if the list is non-empty, else `None`,
    /// with the selected index clamped to `len - 1`.
    pub fn selected_or_none(&self, len: usize) -> Option<usize> {
        if len == 0 {
            None
        } else {
            Some(self.selected.min(len.saturating_sub(1)))
        }
    }
}

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
