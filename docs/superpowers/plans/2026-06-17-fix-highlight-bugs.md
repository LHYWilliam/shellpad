# Fix Highlight Style Bugs — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix selected text invisible (near-black on dark bg) and group rename not using editing style.

**Architecture:** `selected_style()` and `editing_style()` switch from `fg(text_on_selected)` (#11111B, near-black) to `fg(text_primary)` (#CDD6F4, light). Color values brightened slightly. Group rename uses `editing_style()` in render_group_panel.

**Tech Stack:** Rust, Ratatui

---

### Task 1: Fix foreground color in selected/editing styles

**Files:**
- Modify: `src/ui/theme.rs`

- [ ] **Step 1: Switch fg from text_on_selected to text_primary, brighten bg**

In `selected_style` and `editing_style`, change `.fg(self.text_on_selected)` → `.fg(self.text_primary)`:

```rust
    pub fn selected_style(&self) -> Style {
        Style::default()
            .fg(self.text_primary)
            .bg(self.selection_bg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn editing_style(&self) -> Style {
        Style::default()
            .fg(self.text_primary)
            .bg(self.editing_bg)
            .add_modifier(Modifier::BOLD)
    }
```

Also brighten the dark theme bg values slightly for better contrast:

```rust
            selection_bg: Color::Rgb(70, 82, 125),  // slightly lighter indigo
            editing_bg: Color::Rgb(60, 95, 75),      // slightly lighter green
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 3: Commit**

```bash
git add src/ui/theme.rs
git commit -m "fix: use text_primary fg in selected/editing highlights

text_on_selected (#11111B, near-black) was designed for bright
selection backgrounds. With dark muted backgrounds, use
text_primary (#CDD6F4) for readability. Slight bg brightening.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Fix group rename to use editing_style

**Files:**
- Modify: `src/ui/main_screen/render.rs`

- [ ] **Step 1: Add rename editing style dispatch**

In `render_group_panel`, the selected group's style currently uses:

```rust
                let style = if i == self.group_list.selected {
                    theme.selected_style(theme.selection_bg_primary)
```

Which was already replaced to `theme.selected_style()`. Change to:

```rust
                let style = if self.rename_mode && i == self.group_list.selected {
                    theme.editing_style()
                } else if i == self.group_list.selected {
                    theme.selected_style()
```

This applies to two locations in the file — the list item style computation and the `List::highlight_style`.

- [ ] **Step 2: Verify compilation and tests**

Run: `cargo check && cargo test`
Expected: All 228 tests PASS

- [ ] **Step 3: Commit**

```bash
git add src/ui/main_screen/render.rs
git commit -m "fix: group rename uses editing_style instead of selected_style

render_group_panel now dispatches editing_style when rename_mode
is active, matching the semantic distinction: selected = blue-gray,
editing = green tint.

Co-Authored-By: Claude <noreply@anthropic.com>"
```
