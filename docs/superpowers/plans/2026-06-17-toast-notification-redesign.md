# Toast Notification Redesign — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign toast rendering as a stacked column of bordered blocks at bottom-right, showing all active toasts.

**Architecture:** `ToastSeverity::icon()` method added. `app/render.rs` replaces single-toast inline rendering with a loop that builds one `bordered_block_info` per toast, stacking bottom-up within `content_area`.

**Tech Stack:** Rust, Ratatui (no new dependencies)

---

### Task 1: Add `icon()` method to ToastSeverity

**Files:**
- Modify: `src/ui/toast.rs`

- [ ] **Step 1: Add icon method**

```rust
impl ToastSeverity {
    pub fn icon(&self) -> &str {
        match self {
            Self::Success => "✓",
            Self::Error => "✗",
            Self::Info => "●",
        }
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 3: Commit**

```bash
git add src/ui/toast.rs
git commit -m "feat: add icon() method to ToastSeverity

Maps Success→✓, Error→✗, Info→● for toast rendering.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Replace single-toast render with stacked bordered block rendering

**Files:**
- Modify: `src/app/render.rs`

- [ ] **Step 1: Replace toast rendering**

Replace lines 116-136 (the entire single-toast rendering block) with:

```rust
        // Render toast notifications — stacked bottom-right in content_area
        let toasts = &self.toasts.toasts;
        if !toasts.is_empty() {
            let max_w: u16 = toasts.iter()
                .map(|t| {
                    let msg_w = unicode_width::UnicodeWidthStr::width(t.message.as_str()) as u16;
                    // icon(2) + space + message + title padding(4) + borders(2)
                    msg_w + 8
                })
                .max()
                .unwrap_or(20)
                .min(40);
            let toast_h = 3u16;
            let stack_h = toasts.len() as u16 * toast_h;
            let x = content_area.x + content_area.width.saturating_sub(max_w + 2);
            let y = content_area.y + content_area.height.saturating_sub(stack_h);

            for (i, toast) in toasts.iter().enumerate() {
                let row_y = y + i as u16 * toast_h;
                let area = Rect::new(x, row_y, max_w, toast_h);
                frame.render_widget(Clear, area);
                let title = format!(" {} {} ", toast.severity.icon(), toast.message);
                let block = crate::ui::render::bordered_block_info(&self.theme, &title);
                frame.render_widget(&block, area);
            }
        }
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 3: Run tests**

Run: `cargo test`
Expected: All 228 tests PASS

- [ ] **Step 4: Run clippy**

Run: `cargo clippy`
Expected: No new warnings

- [ ] **Step 5: Commit**

```bash
git add src/app/render.rs
git commit -m "feat: render all toasts as stacked bordered blocks bottom-right

Replaces single-line centered toast with vertical stack of
bordered_block_info blocks. Oldest at top, newest at bottom.
Clears each toast independently per frame.

Co-Authored-By: Claude <noreply@anthropic.com>"
```
