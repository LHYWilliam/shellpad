# Highlight Style Unification — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Unify all selected/editing highlights to two styles: `selected_style()` (translucent blue bg) and `editing_style()` (translucent green bg). Picker unchanged.

**Architecture:** Theme gains `selection_bg` + `editing_bg` color fields, `selected_style()` loses its bg parameter, `editing_style()` is new. All 12 call sites across 5 files updated.

**Tech Stack:** Rust, Ratatui (no new dependencies)

---

### Task 1: Update Theme — new colors, new methods

**Files:**
- Modify: `src/ui/theme.rs`

- [ ] **Step 1: Replace selection_bg_primary/secondary with selection_bg + editing_bg**

In the `Theme` struct:

```rust
// REMOVE:
pub selection_bg_primary: Color,
pub selection_bg_secondary: Color,

// ADD:
/// Translucent-feel blue for selected items (all lists, Properties fields).
pub selection_bg: Color,
/// Translucent-feel green for inline editing (Name, WorkDir, variables, commands, group rename).
pub editing_bg: Color,
```

In `default_simple`:
```rust
        selection_bg: Color::Cyan,   // light tint
        editing_bg: Color::Green,    // light tint
```
Remove `selection_bg_primary` and `selection_bg_secondary`.

In `default_dark`:
```rust
        selection_bg: Color::Rgb(60, 70, 110),   // muted indigo-blue
        editing_bg: Color::Rgb(50, 85, 65),       // muted green
```
Remove `selection_bg_primary` and `selection_bg_secondary`.

- [ ] **Step 2: Update `selected_style` — remove bg parameter**

```rust
    /// Style for a selected/highlighted list item.
    pub fn selected_style(&self) -> Style {
        Style::default()
            .fg(self.text_on_selected)
            .bg(self.selection_bg)
            .add_modifier(Modifier::BOLD)
    }
```

- [ ] **Step 3: Add `editing_style`**

After `selected_style`:
```rust
    /// Style for an inline editing field.
    pub fn editing_style(&self) -> Style {
        Style::default()
            .fg(self.text_on_selected)
            .bg(self.editing_bg)
            .add_modifier(Modifier::BOLD)
    }
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: Errors at all `selected_style(theme.selection_bg_*)` call sites — expected, fixed in Task 2.

- [ ] **Step 5: Commit**

```bash
git add src/ui/theme.rs
git commit -m "feat: add selection_bg + editing_bg, unified selected_style signature

selected_style() no longer takes a bg parameter — there is
only one selection background. editing_style() is new for
inline editing fields. Removed selection_bg_primary/secondary.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Update all 12 call sites to use new methods

**Files:**
- Modify: `src/ui/render.rs`
- Modify: `src/ui/main_screen/render.rs`
- Modify: `src/ui/detail_screen/render.rs`
- Modify: `src/ui/variable_screen.rs`

- [ ] **Step 1: Update `src/ui/render.rs` — `list_item_style`**

Current (line ~166-173):
```rust
pub fn list_item_style(is_editing: bool, is_selected: bool, theme: &Theme) -> Style {
    if is_editing {
        Style::default()
            .fg(theme.text_on_selected)
            .bg(theme.accent_primary)
            .add_modifier(Modifier::BOLD)
    } else if is_selected {
        theme.selected_style(theme.selection_bg_secondary)
    } else {
        theme.normal_style()
    }
}
```

Replace with:
```rust
pub fn list_item_style(is_editing: bool, is_selected: bool, theme: &Theme) -> Style {
    if is_editing {
        theme.editing_style()
    } else if is_selected {
        theme.selected_style()
    } else {
        theme.normal_style()
    }
}
```

- [ ] **Step 2: Update `src/ui/main_screen/render.rs` — all 4 call sites**

**Groups list (2 occurrences)** — lines ~57, ~72:
```rust
// change: selected_style(theme.selection_bg_primary) → selected_style()
theme.selected_style()
```

**Sets list (2 occurrences)** — lines ~156, ~234:
```rust
// change: selected_style(theme.selection_bg_secondary) → selected_style()
theme.selected_style()
```

**Group rename** — line ~46-57 does NOT call `selected_style` separately (it uses the same selected style for display since rename replaces the group name in the list). After unification it naturally gets `selected_style()`. No code change needed — the list item rendering already uses the shared style.

- [ ] **Step 3: Update `src/ui/variable_screen.rs` — 2 occurrences**

Lines ~104, ~106:
```rust
// change: selected_style(theme.selection_bg_primary) → selected_style()
theme.selected_style()
```

- [ ] **Step 4: Update `src/ui/detail_screen/render.rs` — `render_editable_field`**

Current (lines ~60-100, the helper method body):
```rust
    fn render_editable_field(
        &self, frame: &mut Frame, row: Rect, theme: &Theme,
        label: &str, focused: bool, editing: bool,
        input: &TextInput, display: &str, dim: bool,
    ) {
        let style = if focused {
            if editing {
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

        let display_style = if dim && !focused {
            Style::default()
                .fg(theme.text_disabled)
                .add_modifier(Modifier::DIM)
        } else {
            style
        };
```

Replace with:
```rust
        let style = if editing {
            theme.editing_style()
        } else if focused {
            theme.selected_style()
        } else {
            theme.normal_style()
        };

        let display_style = if dim && !focused && !editing {
            Style::default()
                .fg(theme.text_disabled)
                .add_modifier(Modifier::DIM)
        } else {
            style
        };
```

The `dim && !focused && !editing` fix: WorkDir should show dim only when not focused AND not editing. Previously editing always set the bold blue style, so dim was overridden. Now explicit check.

- [ ] **Step 5: Update `src/ui/detail_screen/render.rs` — Options ◄► labels**

Group, Shell, and Mode labels currently use `fg(theme.accent_primary)` when focused. Change to `theme.selected_style()`:

Lines ~84 (Shell), ~100 (Group), ~116 (Mode), ~150 (Group full width):
```rust
// change: Style::default().fg(theme.accent_primary) → theme.selected_style()
let group_style = if self.focus == DetailFocus::Group {
    theme.selected_style()
} else {
    theme.normal_style()
};
// Same pattern for shell_style and mode_style
```

Affects: `group_style`, `shell_style`, `mode_style` (2 locations each = 6 replacements).

- [ ] **Step 6: Verify compilation and tests**

Run: `cargo check && cargo test`
Expected: All 228 tests PASS

- [ ] **Step 7: Commit**

```bash
git add src/ui/render.rs src/ui/main_screen/render.rs src/ui/detail_screen/render.rs src/ui/variable_screen.rs
git commit -m "refactor: unify all highlights to selected_style() and editing_style()

12 call sites updated: lists, Properties fields, Options labels,
inline edit fields. selected_style() no longer takes bg parameter.
editing_style() new for all inline editing modes incl. group rename.

Co-Authored-By: Claude <noreply@anthropic.com>"
```
