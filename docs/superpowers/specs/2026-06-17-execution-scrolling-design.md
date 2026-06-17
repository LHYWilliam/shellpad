# 执行屏幕自由滚动设计

## 问题

执行完成后（或运行时用户关闭 auto-scroll），只能通过 ←/→ 在命令间跳转，无法在输出内逐行浏览。尤其长输出场景（如 `seq 1 10005`），用户无法自由向上/向下滚动查看感兴趣的片段。

## 目标

- ↑/↓ 逐行滚动，PageUp/PageDown 翻页
- 与现有 ←/→ 命令跳转不冲突
- 浏览→自由滚动过渡无跳变
- 无需新增状态字段，复用现有 `scroll_offset`/`focus_index`/`auto_scroll`

## 状态模型（三种模式）

| 模式 | `focus_index` | `auto_scroll` | `scroll_offset` 来源 | 触发方式 |
|------|:---:|:---:|------|------|
| 自动跟随 | `None` | `true` | `items_offset_for_command(current_index)` | 默认 / 按 `z` |
| 命令浏览 | `Some(i)` | `false` | `items_offset_for_command(i)` | 按 `←` / `→` |
| 自由滚动 | `None` | `false` | 用户直接修改 | 按 `↑` / `↓` / `PgUp` / `PgDn` |

状态迁移规则：
- 任何方向键（↑/↓/PgUp/PgDn）→ 进入自由滚动模式（`focus_index = None`, `auto_scroll = false`）
- ←/→ → 进入命令浏览模式（现有行为，不变）
- `z` → 切回自动跟随（现有行为，不变）

**关键设计决策：** 渲染时同步 `self.scroll_offset`，确保从浏览/跟随模式切换到自由滚动模式时不跳变。详见渲染变更。

## 按键设计

### 新增

| 键 | 行为 | 实现 |
|---|---|---|
| `↑` | 上滚 1 行 | `focus_index = None; auto_scroll = false; scroll_offset = scroll_offset.saturating_sub(1)` |
| `↓` | 下滚 1 行 | `focus_index = None; auto_scroll = false; scroll_offset = scroll_offset.saturating_add(1)` |
| `PageUp` | 上翻一页 | `focus_index = None; auto_scroll = false; scroll_offset = scroll_offset.saturating_sub(PAGE_SIZE)` |
| `PageDown` | 下翻一页 | `focus_index = None; auto_scroll = false; scroll_offset = scroll_offset.saturating_add(PAGE_SIZE)` |

> `PAGE_SIZE = 20` — 硬编码，不需要 viewport 高度信息。Ratatui List 会自行处理超出范围的 offset。

### 修改

| 键 | 旧行为 | 新行为 |
|---|---|---|
| `←` | 浏览上一个命令 | 不变 |
| `→` | 浏览下一个命令 | 不变 |

**为什么不需要下界检查：** `saturating_sub` 保证不小于 0。上界由 Ratatui List 内部处理。

## 已有 bug 修复

### Bug 1: items_offset 少计截断标记行

`truncated` 命令在渲染中额外有 1 行标记（`"─ (output truncated, ...) ─"`），但 `items_offset_for_command` 未计入。

### Bug 2: items_offset 多计末尾分隔符

渲染仅在 `i + 1 < cmd_states.len()` 时添加分隔符（最后一个命令无末尾分隔符），但 offset 计算始终 +1。

修复后：

```rust
pub(crate) fn items_offset_for_command(&self, cmd_idx: usize) -> usize {
    let mut offset = 0;
    let count = cmd_idx.min(self.cmd_states.len());
    for i in 0..count {
        offset += 1; // command header line
        if self.cmd_states[i].truncated {
            offset += 1; // truncation marker  ← Bug 1 修复
        }
        offset += self.cmd_states[i].output_lines.len(); // output lines
        if i + 1 < self.cmd_states.len() {
            offset += 1; // separator（仅非末尾）← Bug 2 修复
        }
    }
    offset
}
```

## items_total 方法

```rust
/// Total rendered items including summary footer.
pub(crate) fn items_total(&self) -> usize {
    let mut total = self.items_offset_for_command(self.cmd_states.len());
    if self.completed {
        total += 2; // blank line + summary line
    }
    total
}
```

## 渲染变更

### 渲染签名改为 &mut self

`render(&self, ...)` → `render(&mut self, ...)`。原因是：当 render 计算浏览/自动模式下的 offset 时，需要同步写入 `self.scroll_offset`，确保切换到自由滚动时不跳变。

```rust
pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
```

### 同步 scroll_offset

在现有 offset 计算处改为同步写入：

```rust
// 旧
let scroll_offset = if self.focus_index.is_some() || self.auto_scroll {
    self.items_offset_for_command(target_cmd)
} else {
    self.scroll_offset
};

// 新
if self.focus_index.is_some() || self.auto_scroll {
    self.scroll_offset = self.items_offset_for_command(target_cmd);
}
let scroll_offset = self.scroll_offset;
```

这样用户从浏览模式切到自由滚动时，`self.scroll_offset` 已在浏览时被同步为正确位置，不会跳变。

### Footer 提示

所有变体加入 ↑/↓/PgUp/PgDn 提示：

```rust
(Some(_), _, _) => "[←/→] Browse  [↑/↓] Scroll  [PgUp/PgDn] Page  [z] Follow  [q] Back",
(None, true, None) => "[←/→] Browse  [↑/↓] Scroll  [PgUp/PgDn] Page  [r] Re-execute  [q] Back",
(None, true, Some(_)) => "[←/→] Browse  [↑/↓] Scroll  [PgUp/PgDn] Page  [n] Continue  [r] Re-execute  [q] Back",
(None, false, _) => "[←/→] Browse  [↑/↓] Scroll  [PgUp/PgDn] Page  [s] Skip  [z] Follow  [Ctrl+C] Kill  [q] Back",
```

### 滚动条

从"命令级"提升为"行级"精度：

```rust
// 旧
render_scrollbar(frame, scrollbar_area, theme, self.cmd_states.len(), target_cmd);

// 新
let total = self.items_total();
render_scrollbar(frame, scrollbar_area, theme, total, scrollbar_pos);

// scrollbar_pos 取自上述同步后的 self.scroll_offset（已是当前实际位置）
```

## 涉及文件

| 文件 | 改动内容 |
|------|---------|
| `src/ui/execution_screen/mod.rs` | `handle_key` 新增 4 个方向键 arm；新增 `PAGE_SIZE` 常量；新增 `items_total()`；修复 `items_offset_for_command`（两个 bug） |
| `src/ui/execution_screen/render.rs` | `render(&self)` → `render(&mut self)`；同步 `self.scroll_offset`；footer 更新；scrollbar 改用 `items_total()` + `self.scroll_offset` |
| `src/ui/execution_screen/events.rs` | 不变 |

## 测试

### 新增

1. **`test_scroll_up_enters_free_scroll`** — ↑ 减小 scroll_offset，清除 focus_index，关闭 auto_scroll
2. **`test_scroll_down`** — ↓ 增大 scroll_offset
3. **`test_page_up_down`** — PageUp 减 PAGE_SIZE，PageDown 加 PAGE_SIZE
4. **`test_items_offset_includes_truncation_marker`** — 截断命令 offset 多 +1
5. **`test_items_offset_no_trailing_separator`** — 末尾命令不计入分隔符
6. **`test_items_total_with_and_without_footer`** — 完成态 +2，未完成态不 +
7. **`test_scroll_from_browse_seamless`** — ← 进入浏览后再按 ↓，scroll_offset 不跳变（需要 mock render，或用直接字段验证：hndle_key(←) + handle_key(↓) 后 scroll_offset 是连续值）

### 修改

- `make_state` 默认 `truncated = false`，不影响现有测试
- render 签名改为 `&mut self`，所有调用处需适配（`app/render.rs` 中的 `screen.render(...)`）

## 未纳入范围

- 不用 viewport 高度做精确 clamp（PAGE_SIZE 常量足够）
- 不改动 scrollbar 渲染的其他方面
- 不引入鼠标滚轮支持（未来独立功能）
