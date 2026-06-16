# 状态栏编辑显示删除 + Sets 删除逻辑修复 — 设计文档

## 问题

1. 详情屏内联编辑时状态栏仍显示 `Editing: [content]`，光标已在编辑位置，此显示冗余
2. Sets 面板删除操作使用 `set_list.reset()` 导致选中永远回到 0，与其他列表的删除逻辑不一致

## 修复方案

### 1. 详情屏状态栏编辑显示

`detail_screen.rs:render_status_bar()` — 编辑时仅显示按键提示，不显示编辑内容：
- 编辑中：`[Enter] Confirm [Esc] Cancel`
- 非编辑：保持现有上下文提示

### 2. Sets 面板删除逻辑

`app.rs:DeleteSet` — 替换 `set_list.reset()` 为与 `DeleteGroup` 一致的选择调整：
- 删除非尾部项：selected 保持不变（下一项自动顶上）
- 删除尾部项：selected 上移一位
- 空列表不选择

## 涉及文件

| 文件 | 修改项 |
|------|--------|
| `src/ui/detail_screen.rs` | 状态栏编辑内容移除 |
| `src/app.rs` | Sets 删除选择逻辑修复 |

## 测试

- 现有 60 个测试全部通过
- 无需新增测试
