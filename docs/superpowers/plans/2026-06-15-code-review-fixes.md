# Code Review Bug Fixes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 4 bugs and 3 code quality issues found during Code Review of `feat/ui-theme-system` branch.

**Architecture:** All fixes are in `main_screen.rs` and `app.rs`. The search highlighting fix requires rewriting the case-insensitive matching to avoid `to_lowercase()` byte-length mismatch. Toast fixes are one-liners.

**Prerequisites:** Checkout the feat/ui-theme-system branch first: `git checkout feat/ui-theme-system`

---

## File Structure

| File | Task | Changes |
|------|------|---------|
| `src/ui/main_screen.rs` | Task 1 | Replace to_lowercase()+byte-slice with char-level case-insensitive matching |
| `src/app.rs` | Task 2 | Check es.failed/es.skipped for toast severity |
| `src/app.rs` | Task 3 | Replace `.len()` with `UnicodeWidthStr::width()` for toast width |
| `src/app.rs` | Task 4 | Remove auto-save toast on trivial operations |

---

### Task 1: Fix Search Highlighting Unicode Panic

**Files:**
- Modify: `src/ui/main_screen.rs` (the `render_set_panel` method's search highlighting logic)

**Root Cause:** The code calls `set.name.to_lowercase()` to get a lowercased copy, then uses byte positions from `lower_name.find()` to slice `set.name`. But `to_lowercase()` can change string byte length for certain Unicode characters (e.g., Turkish İ 2 bytes → i̇ 3 bytes, ẞ 3 bytes → ß 2 bytes), causing byte indices that don't align with `set.name` — leading to a panic at runtime.

**Fix Approach:** Replace the string-level case folding with character-level case-insensitive comparison using `char_indices()` on the original string. This ensures all byte indices come from the original string and are guaranteed valid.

- [ ] **Add a helper function for Unicode-safe case-insensitive matching**

Add this module-level helper function to `main_screen.rs` (after the imports):

```rust
/// Find case-insensitive matches of `query` in `text`, returning byte-offset pairs
/// into `text` that are guaranteed valid for slicing.
/// Uses character-level case folding to avoid to_lowercase() byte-length mismatch.
fn find_matches_case_insensitive<'a>(text: &'a str, query: &str) -> Vec<(usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }

    let text_chars: Vec<(usize, char)> = text.char_indices().collect();
    let query_lower: Vec<char> = query.chars().flat_map(|c| c.to_lowercase()).collect();
    let text_lower: Vec<char> = text.chars().map(|c| {
        c.to_lowercase().next().unwrap_or(c)
    }).collect();

    let text_len = text_chars.len();
    let q_len = query_lower.len();
    let mut matches = Vec::new();
    let mut i = 0;
    while i + q_len <= text_len {
        if text_lower[i..i + q_len] == query_lower[..] {
            let byte_start = text_chars[i].0;
            let byte_end = if i + q_len < text_len {
                text_chars[i + q_len].0
            } else {
                text.len()
            };
            matches.push((byte_start, byte_end));
            i += q_len;
        } else {
            i += 1;
        }
    }
    matches
}
```

- [ ] **Replace the name_part building block in `render_set_panel()`**

Find the search highlighting code block in `render_set_panel()` (the `if self.search_mode && !self.search_query.is_empty() && !is_selected` block that builds `name_part: Vec<Span>`).

Replace the entire name_part building section:

```rust
                // Build name part with optional search highlighting
                let name_part: Vec<Span> = if self.search_mode && !self.search_query.is_empty() && !is_selected {
                    let matches = find_matches_case_insensitive(&set.name, &self.search_query);
                    if matches.is_empty() {
                        vec![Span::styled(set.name.clone(), text_style)]
                    } else {
                        let mut spans: Vec<Span> = Vec::new();
                        let mut last_end = 0usize;
                        for (match_start, match_end) in &matches {
                            // Text before this match
                            if *match_start > last_end {
                                spans.push(Span::styled(
                                    &set.name[last_end..*match_start],
                                    text_style,
                                ));
                            }
                            // The matched text (highlighted)
                            spans.push(Span::styled(
                                &set.name[*match_start..*match_end],
                                Style::default().fg(theme.accent_primary).add_modifier(Modifier::BOLD),
                            ));
                            last_end = *match_end;
                        }
                        // Remaining text after last match
                        if last_end < set.name.len() {
                            spans.push(Span::styled(&set.name[last_end..], text_style));
                        }
                        spans
                    }
                } else {
                    vec![Span::styled(set.name.clone(), text_style)]
                };
```

Key changes:
- Uses `find_matches_case_insensitive()` which returns byte positions into the original `set.name`
- All `&set.name[range]` slices use positions from `char_indices()` — guaranteed char-aligned
- Falls back to non-highlighted rendering when no matches found
- Handles multiple non-overlapping matches and text before/after/between matches

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
# Then create a test to verify the fix handles edge cases
```

- [ ] **Add a unit test for the find_matches_case_insensitive function**

Add this test at the bottom of `main_screen.rs` (before any closing):

```rust
#[cfg(test)]
mod tests {
    use super::find_matches_case_insensitive;

    #[test]
    fn test_find_matches_ascii() {
        let m = find_matches_case_insensitive("deploy backend", "deploy");
        assert_eq!(m, vec![(0, 6)]);
    }

    #[test]
    fn test_find_matches_case_insensitive_ascii() {
        let m = find_matches_case_insensitive("Deploy Backend", "deploy");
        assert_eq!(m, vec![(0, 6)]);
    }

    #[test]
    fn test_find_matches_no_match() {
        let m = find_matches_case_insensitive("hello world", "xyz");
        assert!(m.is_empty());
    }

    #[test]
    fn test_find_matches_empty_query() {
        let m = find_matches_case_insensitive("hello", "");
        assert!(m.is_empty());
    }

    #[test]
    fn test_find_matches_turkish_i() {
        // İ (U+0130) is 2 bytes, its lowercase i̇ (U+0069 U+0307) is 3 bytes
        // The match byte positions must come from the original string, not the lowercased copy
        let m = find_matches_case_insensitive("AİBC", "i");
        // Should find the İ at byte position 1..3
        assert!(!m.is_empty());
        // Byte range should be valid for original string
        assert_eq!(&"AİBC"[m[0].0..m[0].1], "İ");
    }

    #[test]
    fn test_find_matches_multiple() {
        let m = find_matches_case_insensitive("test test test", "test");
        assert_eq!(m.len(), 3);
        assert_eq!(m[0], (0, 4));
        assert_eq!(m[1], (5, 9));
        assert_eq!(m[2], (10, 14));
    }

    #[test]
    fn test_find_matches_overlapping_not_allowed() {
        // Non-overlapping matches only
        let m = find_matches_case_insensitive("aaaa", "aa");
        assert_eq!(m.len(), 2); // "aa" at 0..2, "aa" at 2..4
    }

    #[test]
    fn test_find_matches_eszett() {
        // ẞ (U+1E9E, capital sharp S, 3 bytes in UTF-8)
        // Its lowercase is ß (U+00DF, 2 bytes)
        let m = find_matches_case_insensitive("STRAẞE", "straße");
        assert!(!m.is_empty());
        // Should find "ẞ" in the original string at valid byte positions
        let s = "STRAẞE";
        let substr = &s[m[0].0..m[0].1];
        assert_eq!(substr, "ẞ");
    }
}
```

- [ ] **Run the tests**

```bash
cargo test main_screen 2>&1
```

Expected: All new tests pass.

- [ ] **Commit**

```bash
git add src/ui/main_screen.rs
git commit -m "fix: search highlighting Unicode panic — use char-level case matching"
```

---

### Task 2: Fix BackToMain Toast Severity

**Files:**
- Modify: `src/app.rs`

**Root Cause:** `on_exec_action` unconditionally uses `ToastSeverity::Success` for the execution summary toast, even when all commands failed or were skipped.

- [ ] **Fix the toast severity in BackToMain handler**

Find this block:
```rust
            ExecutionScreenAction::BackToMain => {
                if let Some(ref es) = self.exec_screen
                    && es.completed {
                    let summary = format!(
                        "Done: {}/{}",
                        es.succeeded + es.failed + es.skipped,
                        es.total,
                    );
                    self.push_toast(summary, ToastSeverity::Success);
                }
                self.kill_execution();
                self.mode = AppMode::Main;
            }
```

Replace with:
```rust
            ExecutionScreenAction::BackToMain => {
                if let Some(ref es) = self.exec_screen
                    && es.completed {
                    let summary = format!(
                        "Done: {}/{}",
                        es.succeeded + es.failed + es.skipped,
                        es.total,
                    );
                    let severity = if es.failed > 0 {
                        ToastSeverity::Error
                    } else if es.skipped > 0 {
                        ToastSeverity::Info
                    } else {
                        ToastSeverity::Success
                    };
                    self.push_toast(summary, severity);
                }
                self.kill_execution();
                self.mode = AppMode::Main;
            }
```

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

Expected: No errors, 51 tests pass.

- [ ] **Commit**

```bash
git add src/app.rs
git commit -m "fix: BackToMain toast severity reflects actual command outcomes"
```

---

### Task 3: Fix Toast Width Calculation

**Files:**
- Modify: `src/app.rs`

**Root Cause:** Toast centering uses `toast_msg.len()` (byte length) instead of Unicode display width. Multi-byte characters (✓ 3 bytes, ✗ 3 bytes, ● 3 bytes) cause the toast width to be overestimated by 4-6 characters.

- [ ] **Fix toast width to use display width**

Find the toast rendering block (around the `toast_width = ...` calculation):

```rust
            let toast_msg = format!("{}{}", toast_label, toast.message);
            let toast_width = (toast_msg.len() as u16 + 2).min(area.width.saturating_sub(4));
            let x = (area.width.saturating_sub(toast_width)) / 2;
```

Replace with:
```rust
            let toast_msg = format!("{}{}", toast_label, toast.message);
            let toast_display_width = unicode_width::UnicodeWidthStr::width(toast_msg.as_str());
            let toast_width = (toast_display_width as u16 + 2).min(area.width.saturating_sub(4));
            let x = (area.width.saturating_sub(toast_width)) / 2;
```

Also update the `toast_area` width to use the corrected width:
```rust
            let toast_area = Rect::new(x, title_area.y, toast_width, 1);
```

The `toast_width` is already used correctly for the Rect — just the calculation now uses display width.

- [ ] **Verify `unicode-width` is already in dependencies**

Check `Cargo.toml`:
```bash
grep unicode-width Cargo.toml
```

Expected: `unicode-width = "0.2.2"` — already a dependency.

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

- [ ] **Commit**

```bash
git add src/app.rs
git commit -m "fix: toast width uses Unicode display width instead of byte length"
```

---

### Task 4: Reduce Auto-Save Toast Noise

**Files:**
- Modify: `src/app.rs`

**Root Cause:** `auto_save()` pushes a "Saved" toast on every call — including NewGroup, DeleteSet, RenameGroup, etc. This makes the toast system noisy and desensitizing. Also, in `handle_variable_action`, the toast fires one frame before execution begins, making it invisible.

**Approach:** Keep "Saved" toasts only for explicit user save actions (Ctrl+S in detail screen). Remove toast from auto-save for CRUD operations. Keep the "Save failed" error toast.

- [ ] **Modify `auto_save()` to only push on error**

```rust
    fn auto_save(&mut self) {
        if let Err(e) = storage::save_app_data(&self.data) {
            self.push_toast(format!("Save failed: {}", e), ToastSeverity::Error);
        }
    }
```

This restores the original behavior of silent auto-save on success, while still surfacing errors via toast.

- [ ] **Add explicit Save toast in `on_detail_action` Save handler**

In the `DetailScreenAction::Save` match arm (around line 263-275), after successfully saving, add a toast:

```rust
            DetailScreenAction::Save(set) => {
                let sid = set.id;
                for group in &mut self.data.groups {
                    if let Some(existing) = group.sets.iter_mut().find(|s| s.id == sid) {
                        *existing = set;
                        existing.updated_at = chrono::Utc::now();
                        break;
                    }
                }
                self.detail_screen = None;
                self.mode = AppMode::Main;
                self.auto_save();
                self.push_toast("Command set saved", ToastSeverity::Success);
            }
```

- [ ] **Compile & test**

```bash
cargo check 2>&1 | grep error
cargo test 2>&1 | tail -3
```

Expected: No errors, 51 tests pass.

- [ ] **Commit**

```bash
git add src/app.rs
git commit -m "fix: reduce auto-save toast noise — only toast on explicit Ctrl+S save"
```

---

### Task 5: Fix Toast Comment (minor cleanup)

**Files:**
- Modify: `src/app.rs`

**Root Cause:** Comment says "right side of title bar" but the formula `(area.width - toast_width) / 2` centers the toast.

- [ ] **Fix the comment**

Change:
```rust
        // Render toast notification (right side of title bar)
```
To:
```rust
        // Render toast notification (centered on title bar)
```

- [ ] **Commit**

```bash
git add src/app.rs
git commit -m "chore: fix toast comment — centered not right-aligned"
```

---

### Verification

- [ ] **Run full test suite**

```bash
cargo test
```

Expected: All 51 + new tests pass.

- [ ] **Run clippy**

```bash
cargo clippy 2>&1 | grep '^error'
```

Expected: No errors.

- [ ] **Build**

```bash
cargo build
```
