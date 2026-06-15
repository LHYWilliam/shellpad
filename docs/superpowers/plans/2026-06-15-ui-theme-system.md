# Wave 1: Theme System + Visual Polish — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a centralized Theme system, thread it through all screens, replace hardcoded colors, then switch to a truecolor dark theme.

**Architecture:** Single `Theme` struct in a new `theme.rs` with two constructors (`default_simple` for exact current colors, `default_dark` for Catppuccin-inspired truecolor). Each screen's `render()` method gains a `theme: &Theme` parameter. This is pure refactoring + color change — no new widgets or behaviors.

**Tech Stack:** ratatui 0.30.1, crossterm 0.29.0

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `src/ui/theme.rs` | **Create** | Theme struct + default_simple() + default_dark() |
| `src/ui/mod.rs` | Modify | Add `pub mod theme;` |
| `src/app.rs` | Modify | Add `theme: Theme` field, pass `&self.theme` to all render calls |
| `src/ui/main_screen.rs` | Modify | Add `theme: &Theme` param, replace 14 hardcoded colors |
| `src/ui/detail_screen.rs` | Modify | Add `theme: &Theme` param, replace 17 hardcoded colors |
| `src/ui/execution_screen.rs` | Modify | Add `theme: &Theme` param, replace 10 hardcoded colors |
| `src/ui/help_screen.rs` | Modify | Add `theme: &Theme` param, replace 3 hardcoded colors |
| `src/ui/variable_screen.rs` | Modify | Add `theme: &Theme` param, replace 4 hardcoded colors |
| `src/ui/components.rs` | Modify | Add `theme: &Theme` param to TextInput::render, replace 2 colors |

---

### Task 1: Create `src/ui/theme.rs`

- [ ] **Write the Theme struct and constructors**

```rust
use ratatui::style::{Color, Modifier, Style};

/// Central theme containing all named styles used across the application.
/// Each screen uses `theme.field_name` instead of hardcoded colors.
#[derive(Debug, Clone)]
pub struct Theme {
    // -- Panel / Surface colors --
    /// Default background for all screens
    pub background: Color,
    /// Panel / dialog background
    pub surface: Color,
    /// Default border for inactive widgets
    pub surface_border: Color,

    // -- Text colors --
    /// Main body text
    pub text_primary: Color,
    /// Less important text (status bar, hints)
    pub text_secondary: Color,
    /// Empty state, placeholder
    pub text_disabled: Color,
    /// Text color when on selected/highlighted background
    pub text_on_selected: Color,

    // -- Accent colors --
    /// Active focus, primary actions (replaces Color::Yellow)
    pub accent_primary: Color,
    /// Success states (replaces Color::Green)
    pub accent_success: Color,
    /// Error states (replaces Color::Red)
    pub accent_error: Color,
    /// Warning states (replaces Color::Yellow in non-focus contexts)
    pub accent_warning: Color,
    /// Informational (replaces Color::Cyan)
    pub accent_info: Color,

    // -- Selection backgrounds --
    /// Background for focused panel/list selection (e.g., Groups panel)
    pub selection_bg_primary: Color,
    /// Background for alternate list selection (e.g., Sets panel, Variables list)
    pub selection_bg_secondary: Color,
}

impl Theme {
    /// Exact replica of current 8-color behavior.
    /// Use this during transition — zero visual change from existing code.
    pub const fn default_simple() -> Self {
        Self {
            background: Color::Reset,
            surface: Color::Reset,
            surface_border: Color::Cyan,
            text_primary: Color::White,
            text_secondary: Color::DarkGray,
            text_disabled: Color::DarkGray,
            text_on_selected: Color::Black,
            accent_primary: Color::Yellow,
            accent_success: Color::Green,
            accent_error: Color::Red,
            accent_warning: Color::Yellow,
            accent_info: Color::Cyan,
            selection_bg_primary: Color::Cyan,
            selection_bg_secondary: Color::Green,
        }
    }

    /// Truecolor dark theme (Catppuccin Mocha-inspired palette).
    pub const fn default_dark() -> Self {
        Self {
            background: Color::Rgb(30, 30, 46),      // mantle
            surface: Color::Rgb(24, 24, 37),          // base
            surface_border: Color::Rgb(69, 71, 90),   // surface0
            text_primary: Color::Rgb(205, 214, 244),  // text
            text_secondary: Color::Rgb(147, 153, 178),// subtext1
            text_disabled: Color::Rgb(108, 112, 134), // overlay1
            text_on_selected: Color::Rgb(17, 17, 27), // crust (dark for contrast)
            accent_primary: Color::Rgb(137, 180, 250),// blue
            accent_success: Color::Rgb(166, 227, 161),// green
            accent_error: Color::Rgb(243, 139, 168),  // red
            accent_warning: Color::Rgb(249, 226, 175),// yellow
            accent_info: Color::Rgb(137, 220, 235),   // sky
            selection_bg_primary: Color::Rgb(137, 180, 250), // blue
            selection_bg_secondary: Color::Rgb(166, 227, 161), // green
        }
    }
}
```

- [ ] **Commit**

```bash
git add src/ui/theme.rs
git commit -m "feat(theme): add Theme struct with default_simple and default_dark constructors"
```

---

### Task 2: Register theme module

- [ ] **Add `pub mod theme;` to `src/ui/mod.rs`**

Edit line 6 of `src/ui/mod.rs`:
```rust
pub mod main_screen;
```
to:
```rust
pub mod main_screen;
pub mod theme;
```

- [ ] **Commit**

```bash
git add src/ui/mod.rs
git commit -m "chore: register theme module"
```

---

### Task 3: Thread theme through `app.rs`

- [ ] **Add `theme: Theme` field to `App` struct, initialize with `default_simple()`**

Field block (after line 41 `pending_set`):
```rust
    // -- UI theme --
    theme: Theme,
```

Constructor initialization (in `Self {` block at line 50-62, add before the closing `}`):
```rust
    theme: Theme::default_simple(),
```

- [ ] **Pass `&self.theme` to all screen render calls**

In `app.rs` `render()` method (lines 90-125), change each render call:

**Line 106** (`AppMode::Main`):
```rust
// Before:
self.main_screen.render(frame, area, &self.data);
// After:
self.main_screen.render(frame, area, &self.data, &self.theme);
```

**Line 110** (`AppMode::Detail`):
```rust
// Before:
ds.render(frame, area);
// After:
ds.render(frame, area, &self.theme);
```

**Line 115** (`AppMode::Execution`):
```rust
// Before:
es.render(frame, area);
// After:
es.render(frame, area, &self.theme);
```

**Line 120** (`AppMode::Help`, first render):
```rust
// Before:
self.main_screen.render(frame, area, &self.data);
// After:
self.main_screen.render(frame, area, &self.data, &self.theme);
```

**Line 120** (`draw_help`):
```rust
// Before:
draw_help(frame, area);
// After:
draw_help(frame, area, &self.theme);
```

**Line 124** (`variable_screen`):
```rust
// Before:
self.variable_screen.render(frame, area);
// After:
self.variable_screen.render(frame, area, &self.theme);
```

Add `use` import for Theme (after existing imports):
```rust
use crate::ui::theme::Theme;
```

- [ ] **Commit**

```bash
git add src/app.rs
git commit -m "refactor: add theme field to App and thread through render calls"
```

---

### Task 4: Thread theme through main_screen.rs

This file has 4 render methods that use colors:
1. `render()` — status bar (line 111-114) and rename input (line 112)
2. `render_group_panel()` — border, selected item, unselected item, empty state (lines 132-186)
3. `render_set_panel()` — border, selected item, unselected item, highlight style (lines 189-267)
4. `render_status_bar()` — status bar text (lines 269-275)

- [ ] **Add `theme: &Theme` parameter to `render()`**

Change signature (line 86):
```rust
pub fn render(&mut self, frame: &mut Frame, area: Rect, data: &AppData, theme: &Theme)
```

Pass `theme` to sub-calls (lines 100, 104, 127):
```rust
self.render_group_panel(frame, left_area, data, theme);
self.render_set_panel(frame, right_area, data, &sets, theme);
self.render_status_bar(frame, status_area, theme);
```

Rename input (line 117): replace `Color::White` with `theme.text_primary`.

- [ ] **Add `theme: &Theme` parameter to `render_group_panel()`**

Change signature (line 131):
```rust
fn render_group_panel(&mut self, frame: &mut Frame, area: Rect, data: &AppData, theme: &Theme)
```

Replace colors:
- Line 133: `Color::Yellow` → `theme.accent_primary`
- Line 135: `Color::Cyan` → `theme.surface_border`
- Line 158: `Color::Black` → `theme.text_on_selected`
- Line 159: `Color::Cyan` → `theme.selection_bg_primary`
- Line 163: `Color::White` → `theme.text_primary`
- Line 173: `Color::DarkGray` → `theme.text_disabled`
- Line 183: `Color::Black` → `theme.text_on_selected`
- Line 184: `Color::Cyan` → `theme.selection_bg_primary`

- [ ] **Add `theme: &Theme` parameter to `render_set_panel()`**

Change signature (line 189):
```rust
fn render_set_panel(
    &self,
    frame: &mut Frame,
    area: Rect,
    data: &AppData,
    sets: &[(usize, usize, &crate::models::CommandSet)],
    theme: &Theme,
)
```

Replace colors:
- Line 207: `Color::Yellow` → `theme.accent_primary`
- Line 209: `Color::Cyan` → `theme.surface_border`
- Line 242: `Color::Black` → `theme.text_on_selected`
- Line 243: `Color::Green` → `theme.selection_bg_secondary`
- Line 247: `Color::White` → `theme.text_primary`
- Line 262: `Color::Black` → `theme.text_on_selected`
- Line 263: `Color::Green` → `theme.selection_bg_secondary`

- [ ] **Add `theme: &Theme` parameter to `render_status_bar()`**

Change signature (line 269):
```rust
fn render_status_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme)
```

Replace color:
- Line 272: `Color::DarkGray` → `theme.text_secondary`

- [ ] **Add `use crate::ui::theme::Theme;` to imports**

After line 1:
```rust
use crate::ui::theme::Theme;
```

- [ ] **Commit**

```bash
git add src/ui/main_screen.rs
git commit -m "refactor(main_screen): thread theme parameter and replace hardcoded colors"
```

---

### Task 5: Thread theme through detail_screen.rs

- [ ] **Add `theme: &Theme` parameter to `render()` and sub-methods**

Add `use crate::ui::theme::Theme;` to imports.

Change signature (line 54):
```rust
pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme)
```

Pass `theme` to sub-calls (lines 76-79):
```rust
self.render_metadata(frame, meta_area, theme);
self.render_variables(frame, var_area, theme);
self.render_commands(frame, cmd_area, theme);
self.render_status_bar(frame, status_area, theme);
```

- [ ] **Update `render_metadata()`**

Change signature:
```rust
fn render_metadata(&self, frame: &mut Frame, area: Rect, theme: &Theme)
```

Replace colors:
- Line 90: `Color::Yellow` → `theme.accent_primary`
- Line 92: `Color::White` → `theme.text_primary`
- Line 113: `Color::Yellow` → `theme.accent_primary`
- Line 115: `Color::White` → `theme.text_primary`
- Line 125: `Color::Yellow` → `theme.accent_primary`
- Line 127: `Color::White` → `theme.text_primary`
- Line 137: `Color::Yellow` → `theme.accent_primary`
- Line 139: `Color::White` → `theme.text_primary`

- [ ] **Update `render_variables()`**

Change signature:
```rust
fn render_variables(&self, frame: &mut Frame, area: Rect, theme: &Theme)
```

Replace colors:
- Line 152: `Color::Yellow` → `theme.accent_primary`
- Line 154: `Color::DarkGray` → `theme.surface_border`
- Line 178: `Color::Black` → `theme.text_on_selected`
- Line 180: `Color::Yellow` → `theme.accent_primary`
- Line 185: `Color::Black` → `theme.text_on_selected`
- Line 187: `Color::Green` → `theme.selection_bg_secondary`
- Line 190: `Color::White` → `theme.text_primary`
- Line 201: `Color::Yellow` → `theme.accent_primary`

- [ ] **Update `render_commands()`**

Change signature:
```rust
fn render_commands(&self, frame: &mut Frame, area: Rect, theme: &Theme)
```

Replace colors:
- Line 225: `Color::Yellow` → `theme.accent_primary`
- Line 227: `Color::DarkGray` → `theme.surface_border`
- Line 260: `Color::Black` → `theme.text_on_selected`
- Line 262: `Color::Yellow` → `theme.accent_primary`
- Line 267: `Color::Black` → `theme.text_on_selected`
- Line 269: `Color::Green` → `theme.selection_bg_secondary`
- Line 272: `Color::White` → `theme.text_primary`
- Line 284: `Color::Yellow` → `theme.accent_primary`

- [ ] **Update `render_status_bar()`**

Change signature:
```rust
fn render_status_bar(&self, frame: &mut Frame, area: Rect, theme: &Theme)
```

Replace color:
- Line 321: `Color::DarkGray` → `theme.text_secondary`

- [ ] **Commit**

```bash
git add src/ui/detail_screen.rs
git commit -m "refactor(detail_screen): thread theme parameter and replace hardcoded colors"
```

---

### Task 6: Thread theme through execution_screen.rs

- [ ] **Add `theme: &Theme` parameter to `render()`**

Add `use crate::ui::theme::Theme;` to imports.

Change signature (line 154):
```rust
pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme)
```

Replace colors:
- Line 168: `Color::Cyan` → `theme.accent_info`
- Line 174: `Color::Green` → `theme.accent_success`
- Line 177: `Color::Yellow` → `theme.accent_warning`
- Line 197: `Color::Green` → `theme.accent_success`
- Line 198: `Color::Red` → `theme.accent_error`
- Line 199: `Color::Yellow` → `theme.accent_warning`
- Line 200: `Color::DarkGray` → `theme.text_disabled`
- Line 201: `Color::DarkGray` → `theme.text_disabled`
- Line 220: `Color::Red` → `theme.accent_error`
- Line 221: `Color::White` → `theme.text_primary`
- Line 248: `Color::Cyan` → `theme.accent_info`
- Line 269: `Color::DarkGray` → `theme.surface_border`
- Line 276: `Color::DarkGray` → `theme.text_secondary`

- [ ] **Commit**

```bash
git add src/ui/execution_screen.rs
git commit -m "refactor(execution_screen): thread theme parameter and replace hardcoded colors"
```

---

### Task 7: Thread theme through help_screen.rs

- [ ] **Add `theme: &Theme` parameter to `draw_help()`**

Add `use crate::ui::theme::Theme;` to imports.

Change signature (line 7):
```rust
pub fn draw_help(frame: &mut Frame, area: Rect, theme: &Theme)
```

Replace colors:
- Line 19: `Color::Cyan` → `theme.accent_info`
- Line 21: `Color::DarkGray` → `theme.surface`
- Line 26: `Color::Cyan` used for section headers → keep but explain: define `let section_color = theme.accent_info;` and use it consistently

The current code uses `cyan` for section headers (line 26: `let cyan = Color::Cyan;`), referenced on lines 29, 33, 45, 53.

Replace line 26:
```rust
let section_color = theme.accent_info;
```

And replace all `cyan` references in the `lines` vector with `section_color`.

- [ ] **Commit**

```bash
git add src/ui/help_screen.rs
git commit -m "refactor(help_screen): thread theme parameter and replace hardcoded colors"
```

---

### Task 8: Thread theme through variable_screen.rs

- [ ] **Add `theme: &Theme` parameter to `render()`**

Add `use crate::ui::theme::Theme;` to imports.

Change signature (line 82):
```rust
pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme)
```

Replace colors:
- Line 100: `Color::Cyan` → `theme.accent_info`
- Line 101: `Color::DarkGray` → `theme.surface`
- Line 109: `Color::Yellow` → `theme.accent_primary`
- Line 109: `Color::White` → `theme.text_primary`
- Line 127: `Color::DarkGray` → `theme.text_secondary`

- [ ] **Commit**

```bash
git add src/ui/variable_screen.rs
git commit -m "refactor(variable_screen): thread theme parameter and replace hardcoded colors"
```

---

### Task 9: Thread theme through components.rs TextInput::render

- [ ] **Add `theme: &Theme` parameter to `TextInput::render()`**

Add `use crate::ui::theme::Theme;` to imports.

Change signature (line 82):
```rust
pub fn render(&self, frame: &mut Frame, area: Rect, focused: bool, title: &str, theme: &Theme)
```

Replace colors:
- Line 84: `Color::Yellow` → `theme.accent_primary`
- Line 86: `Color::DarkGray` → `theme.surface_border`

Note: `TextInput::render()` is called only from `detail_editor.rs` (not in scope for Wave 1 — the function is only used inside `DetailEditState` which uses it directly). Since these are stateful screens that don't pass theme yet, the `TextInput::render` signature change will cause a compilation error in `detail_editor.rs` if called there.

Let me check: does `detail_editor.rs` call `TextInput::render`? Let me check.

Looking at detail_editor.rs — it's a state management file, not rendering. It doesn't call render. So the only caller of `TextInput::render` with the 4-arg signature would be... actually, looking at the code, `TextInput::render` takes 4 args: `frame, area, focused, title`. Let me search for callers.

Actually, the current `TextInput::render` signature is `render(&self, frame, area, focused, title)` with 4 explicit parameters. Looking at the codebase, it's possible it's never called — the detail screen renders inline editing using Paragraph directly in `render_variables`/`render_commands`, not via `TextInput::render`.

Let me verify by running `cargo check` after adding the theme parameter. But since I don't need to worry about callers (the function is there as a utility), just adding the parameter and not breaking anything is fine.

Actually, TextInput::render currently takes (frame, area, focused, title). If no one calls it, changing the signature won't break anything. If someone does call it, they'll get a compile error and we add the theme param at the call site.

Since I can't grep for calls right now, let me just add it and note in the commit that we may need to fix any call sites.

- [ ] **Commit**

```bash
git add src/ui/components.rs
git commit -m "refactor(components): thread theme parameter through TextInput::render"
```

---

### Task 10: Fix compilation and verify

- [ ] **Run cargo check**

```bash
cargo check 2>&1
```

Fix any compilation errors:
- If `detail_editor.rs` or another file calls `TextInput::render` with the old signature, add the theme parameter at the call site.
- If any file uses a screen method with the old signature, add the theme parameter.

- [ ] **Run cargo test**

```bash
cargo test
```

Expected: All 30 existing tests pass.

- [ ] **Commit any fixes**

```bash
git add -A
git commit -m "fix: fix compilation errors from theme threading"
```

---

### Task 11: Switch to default_dark theme

- [ ] **Change Theme initialization in App::new()**

In `src/app.rs`, change line where `theme: Theme::default_simple()` is initialized:
```rust
// Before:
theme: Theme::default_simple(),
// After:
theme: Theme::default_dark(),
```

- [ ] **Run cargo check**

```bash
cargo check
```

- [ ] **Run cargo test**

```bash
cargo test
```

- [ ] **Run cargo clippy**

```bash
cargo clippy 2>&1
```

Verify no regressions.

- [ ] **Commit**

```bash
git add src/app.rs
git commit -m "feat(theme): switch to default_dark truecolor palette"
```

---

### Task 12: Optimize Modifier usage

- [ ] **Audit BOLD usage and reduce to key elements only**

Current pattern: most selected items and headers use `add_modifier(Modifier::BOLD)`. We keep BOLD on:
- Selected list items (to indicate focus)
- Screen titles and section headers
- Command names in execution view

Remove BOLD from items that don't need it. Since the `default_simple()` theme doesn't change colors, the visual impact is minimal. Wait — the modifier changes are purely visual refinements that don't affect color mapping. Let me note this: the plan says to optimize modifiers, but since we want clear visual hierarchy, we should actually keep BOLD on selections but add DIM/ITALIC in appropriate places.

However, adding DIM/ITALIC requires changing more code. Let's be pragmatic — for Wave 1, we handle the colors. Modifier optimization can be done as a lightweight additional commit.

- [ ] **Add DIM modifier to status bar and secondary text**

In `main_screen.rs` `render_status_bar()`:
```rust
// Before:
Style::default().fg(theme.text_secondary)
// After:
Style::default().fg(theme.text_secondary).add_modifier(Modifier::DIM)
```

In `detail_screen.rs` `render_status_bar()`:
```rust
// Before:
Style::default().fg(theme.text_secondary)
// After:
Style::default().fg(theme.text_secondary).add_modifier(Modifier::DIM)
```

In `execution_screen.rs` footer:
```rust
// Before:
Style::default().fg(theme.text_secondary)
// After:
Style::default().fg(theme.text_secondary).add_modifier(Modifier::DIM)
```

In `variable_screen.rs` hint text:
```rust
// Before:
Style::default().fg(theme.text_secondary)
// After:
Style::default().fg(theme.text_secondary).add_modifier(Modifier::DIM)
```

- [ ] **Add ITALIC to empty-state hints**

In `main_screen.rs` `render_group_panel()` (line ~173):
```rust
// Before:
Style::default().fg(theme.text_disabled),
// After:
Style::default().fg(theme.text_disabled).add_modifier(Modifier::ITALIC),
```

- [ ] **Commit**

```bash
git add src/ui/main_screen.rs src/ui/detail_screen.rs src/ui/execution_screen.rs src/ui/variable_screen.rs
git commit -m "style: optimize modifier usage — add DIM to status bars, ITALIC to empty state"
```

---

### Task 13: Final verification

- [ ] **Run full test suite**

```bash
cargo test
```

Expected output: `ok` across all test groups.

- [ ] **Run clippy**

```bash
cargo clippy 2>&1
```

Expected: no warnings (or only pre-existing ones unrelated to our changes).

- [ ] **Build release**

```bash
cargo build
```

- [ ] **Commit any remaining fixes**

```bash
git add -A
git commit -m "chore: final fixes after theme migration"
```
