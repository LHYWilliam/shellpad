# Phase 4: 三大 Screen 拆分 — 实施计划

> **For agentic workers:** 此计划包含 3 个独立子阶段（4a/4b/4c），每个互不依赖。每个子阶段后运行 `cargo check` 确保编译通过。3 个子阶段可并行执行，但建议顺序执行以减少冲突排查成本。

**Goal:** 将 `main_screen.rs`(655行)、`detail_screen.rs`(643行)、`execution_screen.rs`(371行) 按"模块根 + 子目录"模式拆分。

**Architecture:** 每个 Screen 的父文件保留 struct 定义 + `new()` + `render()` 分派 + `handle_key()` 分派 + 子模块声明。渲染方法提取到 `*/render.rs`，键盘处理提取到 `*/handler.rs`。主屏幕额外提取搜索逻辑到 `*/search.rs`。

---

### 子阶段 4a：`main_screen/` 拆分

**边界：**

| 保留在 `main_screen.rs` | 迁到 `main_screen/render.rs` | 迁到 `main_screen/handler.rs` | 迁到 `main_screen/search.rs` |
|------------------------|------------------------------|------------------------------|------------------------------|
| imports (精简) | `render_group_panel()` | `handle_key()` 完整方法体 | `find_matches_case_insensitive()` |
| `Panel` enum (52-56) | `render_set_panel()` | | `#[cfg(test)] mod tests` 全部 7 个搜索测试 |
| `MainScreenState` struct + `new()` (58-79) | `render_status_bar()` | | |
| `selected_group_idx()` (81-88) | | | |
| `selected_set_idx()` (90-98) | | | |
| `visible_sets()` (100-117) | | | |
| `render()` 分派 (119-141) | | | |
| `handle_key()` 签名 (第 388 行) | | | |
| 新增: `pub(crate) mod render; pub(crate) mod handler; pub(crate) mod search;` | | | |

**步骤：**

1. 创建 `main_screen/render.rs`，提取渲染方法：

```rust
use crate::models::AppData;
use crate::ui::render::{
    bordered_block, empty_hint, fill_row, list_scrollbar_areas, render_inline_cursor,
    render_scrollbar, render_status_bar, set_cursor_after_prefix,
};
use crate::ui::theme::Theme;
use crate::ui::main_screen::{MainScreenState, Panel};
use crate::ui::main_screen::search::find_matches_case_insensitive;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};

impl MainScreenState {
    pub fn render_group_panel(&mut self, frame: &mut Frame, area: Rect, data: &AppData, theme: &Theme) {
        // 原 main_screen.rs 第 143-217 行内容
        let block = bordered_block(theme, " Groups ", self.active_panel == Panel::Groups);
        let inner = block.inner(area);
        frame.render_widget(&block, area);
        let (list_area, scrollbar_area) = list_scrollbar_areas(inner);
        let avail = list_area.width as usize;
        let mut items: Vec<ListItem> = data
            .groups
            .iter()
            .enumerate()
            .map(|(i, g)| {
                let marker = if i == self.group_list.selected { "▶ " } else { "  " };
                let display_name = if self.rename_mode && i == self.group_list.selected {
                    &self.rename_input.content
                } else {
                    &g.name
                };
                let name = format!("{}{}", marker, display_name);
                let count = format!("({})", g.sets.len());
                let name_width = unicode_width::UnicodeWidthStr::width(name.as_str());
                let pad = avail.saturating_sub(name_width + count.len());
                let label = format!("{}{:>pad$}{}", name, "", count, pad = pad);
                let style = if i == self.group_list.selected {
                    theme.selected_style(theme.selection_bg_primary)
                } else {
                    theme.normal_style()
                };
                let line = fill_row(Line::from(Span::styled(label, style)), style, list_area.width);
                ListItem::new(line)
            })
            .collect();
        if data.groups.is_empty() {
            items.push(empty_hint(theme, " (empty — press g to add) "));
        }
        let mut list_state = ratatui::widgets::ListState::default()
            .with_selected(self.group_list.selected_or_none(data.groups.len()));
        let list = List::new(items).highlight_style(theme.selected_style(theme.selection_bg_primary));
        frame.render_stateful_widget(list, list_area, &mut list_state);
        render_scrollbar(frame, scrollbar_area, theme, data.groups.len(), self.group_list.selected);
        if self.rename_mode && !data.groups.is_empty() {
            render_inline_cursor(frame, list_area, self.group_list.offset, self.group_list.selected,
                &self.rename_input, unicode_width::UnicodeWidthStr::width("▶ ") as u16);
        }
    }

    pub fn render_set_panel(
        &self, frame: &mut Frame, area: Rect, data: &AppData,
        sets: &[(usize, usize, &crate::models::CommandSet)], theme: &Theme,
    ) {
        // 原 main_screen.rs 第 219-376 行 — 完整复制
        let title = if self.search_mode {
            " Search ".to_string()
        } else {
            let group_name: &str = self.selected_group_idx(data)
                .map(|gi| data.groups[gi].name.as_str()).unwrap_or("Commands");
            format!(" {} ", group_name)
        };
        let block = bordered_block(theme, &title, self.active_panel == Panel::Sets);
        let inner = block.inner(area);
        frame.render_widget(&block, area);
        let (list_area, scrollbar_area) = if self.search_mode {
            let search_layout = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]);
            let [search_line, remaining] = search_layout.areas(inner);
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    format!(" Search: {} ", self.search_input.content),
                    Style::default().fg(theme.text_primary),
                ))), search_line,
            );
            let prefix_width = unicode_width::UnicodeWidthStr::width(" Search: ");
            set_cursor_after_prefix(frame, &self.search_input.content,
                self.search_input.cursor, prefix_width as u16, search_line);
            list_scrollbar_areas(remaining)
        } else {
            list_scrollbar_areas(inner)
        };
        let mut items: Vec<ListItem> = sets.iter().enumerate().map(|(i, &(gi, _, set))| {
            let shell_label = set.shell.label();
            let mode_label = match set.exec_mode {
                crate::models::ExecMode::StopOnError => "🛑",
                crate::models::ExecMode::ContinueOnError => "⏩",
            };
            let cmd_count = set.commands.len();
            let is_selected = i == self.set_list.selected && self.active_panel == Panel::Sets;
            let text_style = if is_selected {
                theme.selected_style(theme.selection_bg_secondary)
            } else { theme.normal_style() };
            let prefix = format!(" {}  ", mode_label);
            let suffix = format!("  [{}] ({} cmd)", shell_label, cmd_count);
            let name_part: Vec<Span> = if self.search_mode && !self.search_input.content.is_empty() && !is_selected {
                let matches = find_matches_case_insensitive(&set.name, &self.search_input.content);
                if matches.is_empty() {
                    vec![Span::styled(set.name.clone(), text_style)]
                } else {
                    let mut spans: Vec<Span> = Vec::new();
                    let mut last_end = 0usize;
                    for (match_start, match_end) in &matches {
                        if *match_start > last_end {
                            spans.push(Span::styled(&set.name[last_end..*match_start], text_style));
                        }
                        spans.push(Span::styled(&set.name[*match_start..*match_end],
                            Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD)));
                        last_end = *match_end;
                    }
                    if last_end < set.name.len() {
                        spans.push(Span::styled(&set.name[last_end..], text_style));
                    }
                    spans
                }
            } else {
                vec![Span::styled(set.name.clone(), text_style)]
            };
            let mut parts = vec![Span::styled(prefix, text_style)];
            parts.extend(name_part);
            parts.push(Span::styled(suffix, text_style));
            if self.search_mode {
                let gname = data.groups.get(gi).map(|g| g.name.as_str()).unwrap_or("?");
                let text_width: usize = parts.iter().map(|s| unicode_width::UnicodeWidthStr::width(s.content.as_ref())).sum();
                let pad = list_area.width as usize;
                let padding = pad.saturating_sub(text_width + gname.len() + 1);
                if padding > 0 { parts.push(Span::styled(" ".repeat(padding), text_style)); }
                parts.push(Span::styled(gname, text_style));
            }
            let set_line = fill_row(Line::from(parts), text_style, list_area.width);
            ListItem::new(set_line)
        }).collect();
        if sets.is_empty() { items.push(empty_hint(theme, " (empty — press n to add a set) ")); }
        let selected = if self.active_panel == Panel::Sets { self.set_list.selected_or_none(sets.len()) } else { None };
        let mut list_state = ratatui::widgets::ListState::default().with_selected(selected);
        let list = List::new(items).highlight_style(theme.selected_style(theme.selection_bg_secondary));
        frame.render_stateful_widget(list, list_area, &mut list_state);
        render_scrollbar(frame, scrollbar_area, theme, sets.len(), selected.unwrap_or(0));
    }

    pub fn render_status_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        render_status_bar(frame, area, theme,
            " [↑/↓] Nav  [←/→] Panel  [Enter] Run  [e] Edit  [n] New  [d] Del set  [Shift+D] Del group  [g] Group  [/] Search  [?] Help  [q] Quit");
    }
}
```

创建文件后，将 main_screen.rs 中的 `render_group_panel`, `render_set_panel`, `render_status_bar`, `find_matches_case_insensitive`, 和测试块删除。

2. 创建 `main_screen/handler.rs`：

```rust
use crate::action::AppAction;
use crate::models::AppData;
use crate::ui::main_screen::MainScreenState;
use crate::ui::widget::text_input::handle_text_input;
use crate::ui::widget::TextInput;
use crossterm::event::KeyEvent;

impl MainScreenState {
    pub fn handle_key(&mut self, key: KeyEvent, data: &AppData) -> AppAction {
        // 完整复制 main_screen.rs 第 388-576 行内容
        use crossterm::event::KeyCode;
        if self.rename_mode {
            return match key.code {
                KeyCode::Enter => {
                    let name = self.rename_input.content.clone();
                    let gi = self.group_list.selected;
                    self.rename_mode = false;
                    AppAction::RenameGroup(gi, name)
                }
                KeyCode::Esc => { self.rename_mode = false; AppAction::None }
                _ => { handle_text_input(&mut self.rename_input, key); AppAction::None }
            };
        }
        if self.search_mode {
            return match key.code {
                KeyCode::Esc => {
                    self.search_mode = false;
                    self.search_input = TextInput::new(String::new());
                    self.set_list.reset();
                    self.active_panel = crate::ui::main_screen::Panel::Groups;
                    AppAction::None
                }
                KeyCode::Enter => {
                    let results = data.filter_sets(&self.search_input.content);
                    if let Some((gi, si, _)) = results.get(self.set_list.selected) {
                        self.group_list.selected = *gi;
                        self.set_list.selected = *si;
                        self.active_panel = crate::ui::main_screen::Panel::Sets;
                    }
                    self.search_mode = false;
                    self.search_input = TextInput::new(String::new());
                    AppAction::None
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    self.set_list.select_previous(); AppAction::None
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    let n = data.filter_sets(&self.search_input.content).len();
                    self.set_list.select_next(n); AppAction::None
                }
                _ => {
                    handle_text_input(&mut self.search_input, key);
                    self.active_panel = crate::ui::main_screen::Panel::Sets;
                    self.set_list.reset();
                    AppAction::None
                }
            };
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                match self.active_panel {
                    crate::ui::main_screen::Panel::Groups => self.group_list.select_previous(),
                    crate::ui::main_screen::Panel::Sets => {
                        if self.visible_sets(data).is_empty() {
                            self.active_panel = crate::ui::main_screen::Panel::Groups;
                        } else { self.set_list.select_previous(); }
                    }
                }
                AppAction::None
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                match self.active_panel {
                    crate::ui::main_screen::Panel::Groups => self.group_list.select_next(data.groups.len()),
                    crate::ui::main_screen::Panel::Sets => {
                        let n = self.visible_sets(data).len();
                        if n == 0 { self.active_panel = crate::ui::main_screen::Panel::Groups; }
                        else { self.set_list.select_next(n); }
                    }
                }
                AppAction::None
            }
            KeyCode::Left => { match self.active_panel {
                crate::ui::main_screen::Panel::Sets => self.active_panel = crate::ui::main_screen::Panel::Groups,
                _ => {}
            } AppAction::None }
            KeyCode::Right => { match self.active_panel {
                crate::ui::main_screen::Panel::Groups => {
                    let has_sets = self.selected_group_idx(data).map(|gi| !data.groups[gi].sets.is_empty()).unwrap_or(false);
                    if has_sets { self.active_panel = crate::ui::main_screen::Panel::Sets; }
                }
                _ => {}
            } AppAction::None }
            KeyCode::Enter => {
                if self.active_panel == crate::ui::main_screen::Panel::Sets
                    && let Some((gi, si)) = self.selected_set_idx(data)
                { return AppAction::ExecuteSet(gi, si); }
                AppAction::None
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                if self.active_panel == crate::ui::main_screen::Panel::Sets
                    && let Some((gi, si)) = self.selected_set_idx(data)
                { return AppAction::EditSet(gi, si); }
                AppAction::None
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                if let Some(gi) = self.selected_group_idx(data) { AppAction::NewSet(gi) }
                else { AppAction::None }
            }
            KeyCode::Char('d') => {
                if self.active_panel == crate::ui::main_screen::Panel::Sets
                    && let Some((gi, si)) = self.selected_set_idx(data)
                { return AppAction::DeleteSet(gi, si); }
                AppAction::None
            }
            KeyCode::Char('D') => {
                if self.active_panel == crate::ui::main_screen::Panel::Groups
                    && let Some(gi) = self.selected_group_idx(data)
                { return AppAction::DeleteGroup(gi); }
                AppAction::None
            }
            KeyCode::Char('g') => AppAction::NewGroup,
            KeyCode::Char('R') => {
                if self.active_panel == crate::ui::main_screen::Panel::Groups
                    && let Some(gi) = self.selected_group_idx(data)
                {
                    self.rename_mode = true;
                    self.rename_input = TextInput::new(data.groups[gi].name.clone());
                }
                AppAction::None
            }
            KeyCode::Char('/') => {
                self.search_mode = true;
                self.search_input.content.clear();
                self.set_list.reset();
                self.active_panel = crate::ui::main_screen::Panel::Sets;
                AppAction::None
            }
            KeyCode::Char('?') => AppAction::Help,
            KeyCode::Char('h') | KeyCode::Char('H') => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) { return AppAction::Help; }
                AppAction::None
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                if key.code == KeyCode::Esc { AppAction::None } else { AppAction::Quit }
            }
            _ => AppAction::None,
        }
    }
}
```

注意：`Panel` 枚举在 `main_screen.rs` 中，子模块通过 `crate::ui::main_screen::Panel` 访问。

3. 创建 `main_screen/search.rs`：

```rust
/// Find case-insensitive matches of `query` in `text`, returning byte-offset pairs
/// into `text` that are guaranteed valid for slicing.
pub fn find_matches_case_insensitive<'a>(text: &'a str, query: &str) -> Vec<(usize, usize)> {
    if query.is_empty() { return Vec::new(); }
    let text_chars: Vec<(usize, char)> = text.char_indices().collect();
    let query_lower: Vec<char> = query.chars().flat_map(|c| c.to_lowercase()).collect();
    let text_lower: Vec<char> = text.chars().map(|c| c.to_lowercase().next().unwrap_or(c)).collect();
    let text_len = text_chars.len();
    let q_len = query_lower.len();
    let mut matches = Vec::new();
    let mut i = 0;
    while i + q_len <= text_len {
        if text_lower[i..i + q_len] == query_lower[..] {
            let byte_start = text_chars[i].0;
            let byte_end = if i + q_len < text_len { text_chars[i + q_len].0 } else { text.len() };
            matches.push((byte_start, byte_end));
            i += q_len;
        } else { i += 1; }
    }
    matches
}

#[cfg(test)]
mod tests {
    use super::find_matches_case_insensitive;

    #[test] fn test_find_matches_ascii() {
        let m = find_matches_case_insensitive("deploy backend", "deploy");
        assert_eq!(m, vec![(0, 6)]);
    }
    #[test] fn test_find_matches_case_insensitive_ascii() {
        let m = find_matches_case_insensitive("Deploy Backend", "deploy");
        assert_eq!(m, vec![(0, 6)]);
    }
    #[test] fn test_find_matches_no_match() {
        let m = find_matches_case_insensitive("hello world", "xyz");
        assert!(m.is_empty());
    }
    #[test] fn test_find_matches_empty_query() {
        let m = find_matches_case_insensitive("hello", "");
        assert!(m.is_empty());
    }
    #[test] fn test_find_matches_multiple() {
        let m = find_matches_case_insensitive("test test test", "test");
        assert_eq!(m.len(), 3);
        assert_eq!(m[0], (0, 4)); assert_eq!(m[1], (5, 9)); assert_eq!(m[2], (10, 14));
    }
    #[test] fn test_find_matches_partial_word() {
        let m = find_matches_case_insensitive("deployment", "ploy");
        assert_eq!(m, vec![(2, 6)]);
    }
    #[test] fn test_find_matches_unicode_safe() {
        let m = find_matches_case_insensitive("Café", "café");
        assert_eq!(m, vec![(0, 5)]);
    }
    #[test] fn test_find_matches_eszett_roundtrip() {
        let text = "STRAẞE";
        let m = find_matches_case_insensitive(text, "e");
        assert_eq!(m.len(), 1);
        assert_eq!(&text[m[0].0..m[0].1], "E");
    }
    #[test] fn test_find_matches_eszett_inside() {
        let text = "STRAẞE";
        let m = find_matches_case_insensitive(text, "e");
        assert!(!m.is_empty());
        for (start, end) in &m { let _ = &text[*start..*end]; }
    }
}
```

4. 精简 `main_screen.rs`：

保留的内容：
```rust
use crate::action::AppAction;
use crate::models::AppData;
use crate::ui::main_screen::render;
use crate::ui::main_screen::handler;
pub(crate) mod render;
pub(crate) mod handler;
pub(crate) mod search;

use crate::ui::theme::Theme;
use crate::ui::widget::{ScrollableList, TextInput};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::Paragraph;
use ratatui::style::Style;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel { Groups, Sets }

pub struct MainScreenState {
    pub group_list: ScrollableList,
    pub set_list: ScrollableList,
    pub active_panel: Panel,
    pub search_mode: bool,
    pub search_input: TextInput,
    pub rename_mode: bool,
    pub rename_input: TextInput,
}

impl MainScreenState {
    pub fn new() -> Self { /* 同前 */ }

    pub fn selected_group_idx(&self, data: &AppData) -> Option<usize> { /* 同前 */ }
    pub fn selected_set_idx(&self, data: &AppData) -> Option<(usize, usize)> { /* 同前 */ }
    pub fn visible_sets<'a>(&'a self, data: &'a AppData) -> Vec<(usize, usize, &'a crate::models::CommandSet)> { /* 同前 */ }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, data: &AppData, theme: &Theme) {
        let vertical = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]);
        let [main_area, status_area] = vertical.areas(area);
        let horizontal = Layout::horizontal([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)]);
        let [left_area, right_area] = horizontal.areas(main_area);
        let left_vis = left_area.height.saturating_sub(2) as usize;
        let right_vis = right_area.height.saturating_sub(2) as usize;
        self.group_list.update_offset(left_vis);
        self.set_list.update_offset(right_vis);
        self.render_group_panel(frame, left_area, data, theme);
        let sets = self.visible_sets(data);
        self.render_set_panel(frame, right_area, data, &sets, theme);
        self.render_status_bar(frame, status_area, theme);
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent, data: &AppData) -> AppAction {
        // 完整体在 main_screen/handler.rs 中
        handler::handle_key(self, key, data)
    }
}
```

注意：由于 `impl MainScreenState` 块分布在多个文件中，`render()` 方法会调用子模块的方法（Rust 自动找到所有 impl 块中的方法）。但 `handle_key()` 在子模块中，在父文件中需要方法签名。实际上最简单的做法是让 `main_screen.rs` 保留方法分派：

```rust
pub fn render(&mut self, frame: &mut Frame, area: Rect, data: &AppData, theme: &Theme) {
    // render methods are in render.rs — called directly as self.render_*()
    // ... the code above
}
pub fn handle_key(&mut self, key: ..., data: ...) -> AppAction {
    // handler.rs has the full impl
    handler::handle_key(self, key, data)
}
```

Wait, that's not right. The render methods like `render_group_panel` are called as `self.render_group_panel(...)` and they're in `render.rs` as `impl MainScreenState`. So when the parent `main_screen.rs` calls `self.render_group_panel(...)`, Rust's method resolution finds it in `render.rs`. No forwarding needed!

Similarly, `handle_key` is in `handler.rs`. The parent's `render()` method can call `self.render_group_panel(...)` without any explicit forwarding. The method is resolved across impl blocks.

So the parent `main_screen.rs` just needs to remove those methods and rely on Rust's cross-file method resolution. No forwarding functions needed.

But there's a subtlety: `render()` is defined in the parent AND calls `self.render_group_panel()` which is in `render.rs`. This works because Rust resolves methods across all `impl MainScreenState` blocks in the same crate. ✓

5. 创建 `main_screen/` 目录，将 `render.rs`, `handler.rs`, `search.rs` 放入。

6. 删除 `main_screen.rs` 中的：
   - `render_group_panel`, `render_set_panel`, `render_status_bar` 方法
   - `find_matches_case_insensitive` 函数
   - `#[cfg(test)] mod tests { ... }` 块
   - 不再需要的 imports（`handle_text_input`, `render::*` 等部分）
   imports 保留的最小集合在精简版 `main_screen.rs` 顶部。

---

### 子阶段 4b：`detail_screen/` 拆分

**边界：**

| 保留在 `detail_screen.rs` | 迁到 `detail_screen/render.rs` | 迁到 `detail_screen/handler.rs` |
|------------------------|-------------------------------|-------------------------------|
| `DetailFocus` enum | `render_metadata()` | `handle_key()` 完整体 |
| `DetailScreenState` struct + `new()` | `render_variables()` | `commit_name_edit()` |
| `render()` 分派 + scroll 更新 | `render_commands()` | |
| `render_items_list()` — 共享渲染辅助 | `render_status_bar()` | |
| `cycle_group()` / `cycle_shell()` / `cycle_exec_mode()` | | |
| `cycle_enum()` 自由函数 | | |
| 新增: `pub(crate) mod render; pub(crate) mod handler;` | | |

**步骤：**

1. 创建 `detail_screen/render.rs`，提取 4 个渲染方法 + `render_items_list`。
   `render_items_list` 虽然被 `render_variables` 和 `render_commands` 使用，但要保留在 `render.rs` 中（它本身是渲染辅助函数）。

2. 创建 `detail_screen/handler.rs`，提取 `handle_key` + `commit_name_edit`。
   `cycle_group/shell/exec_mode` 和 `cycle_enum` 留在 `detail_screen.rs` 父文件中（它们是键盘处理的辅助函数，被 handler 和父文件都使用）。

3. 精简 `detail_screen.rs`：保留 struct + `new()` + `render()` 分派 + `handle_key` 分派 + `cycle_*` 方法。

注意：与 `main_screen` 类似，`render()` 在父文件调用 `self.render_metadata(...)` 等方法，这些方法在 `render.rs` 中定义。`handle_key()` 在父文件中的签名在 `handler.rs` 中实现，不需要额外的分派函数。

---

### 子阶段 4c：`execution_screen/` 拆分

**边界：**

| 保留在 `execution_screen.rs` | 迁到 `execution_screen/render.rs` | 迁到 `execution_screen/events.rs` |
|-----------------------------|----------------------------------|---------------------------------|
| `CmdStatus` enum | `render()` 完整方法体 | `process_events()` |
| `CmdState` struct | `format_duration()` 自由函数 | `mark_remaining_as_skipped()` |
| `ExecutionScreenState` struct + `new()` | | `reset_from()` |
| `handle_key()` 签名 | | `items_offset_for_command()` |
| 新增: `pub(crate) mod render; pub(crate) mod events;` | | |

**步骤：**

1. 创建 `execution_screen/render.rs`，提取 `render()` + `format_duration()`。
   注意：`render()` 要作为 `impl ExecutionScreenState` 中的方法。

2. 创建 `execution_screen/events.rs`，提取 `process_events()`, `mark_remaining_as_skipped()`, `reset_from()`, `items_offset_for_command()`。

3. 精简 `execution_screen.rs`：保留 struct + `new()` + `render()` 分派 + `handle_key()` 分派。

---

### 验证

每个子阶段后运行：
```bash
cargo check
```

全部完成后：
```bash
cargo test
cargo clippy
cargo fmt
```

最终提交：
```bash
git add src/ui/
git commit -m "refactor(phase4): 拆分三大 Screen → render/handler/events 子模块"
```
