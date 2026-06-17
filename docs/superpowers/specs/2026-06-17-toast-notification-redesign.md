# Toast Notification Redesign — Design Spec

**Date:** 2026-06-17
**Status:** Approved
**Scope:** Redesign toast rendering to show all active toasts in a stacked column at the bottom-right, each wrapped in a bordered block

## Problem

Current toast system is a single line of colored text centered on the title bar.
Only the most recent toast is rendered (`toasts.last()`). Consecutive operations
overwrite the display with no visual indication of the previous operation. The
text-only format is easily overlooked.

## Solution

Render **all active toasts** in a vertical stack at the bottom-right corner of
the content area. Each toast is wrapped in a `bordered_block_info` with icon
prefix. Oldest toast at the top, newest at the bottom. Automatically cleaned up
after 3 seconds.

## Layout

Toast stack is anchored at the **bottom-right of the content area** (above the
status bar), independent of which mode is active. Width auto-fits by message
length, height = 3 rows per toast.

```
┌ Groups ────────┐ ┌ Sets ────────────────────────────────────────┐
│                 │ │                                               │
│                 │ │       ┌ ● Group created ───────────────────┐ │ ← 最旧
│                 │ │       └────────────────────────────────────┘ │
│                 │ │       ┌ ● Set deleted ────────────────────┐  │
│                 │ │       └────────────────────────────────────┘ │
│                 │ │       ┌ ● Set moved ──────────────────────┐  │ ← 最新
│                 │ │       └────────────────────────────────────┘ │
│                 │ │                                               │
├─ status bar ─────────────────────────────────────────────────────┤
```

Toasts render across **all modes** (Main, Detail, Execution, Help) — they are
overlaid on top of the content area in the same way regardless of what screen
is active underneath.

## Design

### Toast block

Each toast uses `bordered_block_info` with the icon + message as title:

```rust
let title = format!(" {} {} ", severity_icon, toast.message);
let block = bordered_block_info(theme, &title);
```

Title fits within the toast's `bordered_block_info` — the border color is
`accent_info` (blue), consistent with other overlay dialogs.

### Stack layout

Toasts are stacked bottom-up within the **Sets panel area** (right column).
They do NOT overflow into the Groups panel.

Width: auto-fit to message length (max ~40 chars).  
Height: 3 rows per toast (2 borders + 1 content).  
Position: bottom-right of the Sets panel.

### Toast lifecycle

- **Create**: `add()` pushes to `Vec<Toast>` (unchanged from current)
- **Render**: all toasts in the Vec, newest at bottom
- **Expire**: `clean_expired()` removes toasts older than 3 seconds (unchanged, called every tick)

### Severity icons

| Severity | Icon | Color |
|----------|------|-------|
| Success | `✓` | `accent_success` |
| Error | `✗` | `accent_error` |
| Info | `●` | `accent_info` |

Same as current — no new severities needed.

## Implementation

### Rendering location

Toasts are rendered in `app/render.rs` at the **bottom-right of `content_area`**,
after the underlying screen is drawn. The toast column stacks upward from the
bottom.

Layout computation:

```rust
let toasts = &self.toasts.toasts;
if toasts.is_empty() { return; }

let max_w: u16 = toasts.iter()
    .map(|t| unicode_width::UnicodeWidthStr::width(t.message.as_str()) as u16 + 6)
    .max().unwrap_or(20)
    .min(40);
let toast_h = 3u16; // 2 borders + 1 content
let stack_h = toasts.len() as u16 * toast_h;

let stack_area = Rect::new(
    content_area.x + content_area.width.saturating_sub(max_w + 2),
    content_area.y + content_area.height.saturating_sub(stack_h),
    max_w,
    stack_h,
);

for (i, toast) in toasts.iter().enumerate() {
    let y = stack_area.y + i as u16 * toast_h;
    let area = Rect::new(stack_area.x, y, stack_area.width, toast_h);
    frame.render_widget(Clear, area);
    let block = bordered_block_info(theme, &format!(" {} {} ", icon, toast.message));
    frame.render_widget(&block, area);
}
```

Note: `Clear` is rendered per-toast rather than once for the whole stack, so
each toast can independently clear and redraw on each frame.

### `app/render.rs` changes

Replace the current inline single-toast rendering (lines 116-136) with the stack
rendering code above.

### Clean expired toasts

`clean_expired()` is already called every tick in `App::run()`. No change needed.

## `toast.rs` changes

Add `impl Display` or a format method to map severity to icon char:

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

## Files Affected

| File | Change |
|------|--------|
| `src/ui/toast.rs` | Add `icon()` method on `ToastSeverity` |
| `src/app/render.rs` | Replace inline toast rendering with `render_toast_stack` |
| `src/ui/render.rs` | (no change, `bordered_block_info` already shared) |

## Tests

No new handler tests — rendering-only change. Existing toast unit tests (add/clean/severity) unchanged.

Estimated: ~40 lines production code.
