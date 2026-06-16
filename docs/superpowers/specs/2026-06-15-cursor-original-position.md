# 输入光标移至原位置 + 空变量插入修复 — 设计文档

## 问题

### 1+2: 光标在原位置 + 删除左下角输入栏
重命名和搜索的输入框当前显示在状态栏（左下角），光标也在那里。用户希望在**输入的原位置**显示光标，并删除状态栏的输入显示。

### 3: 空变量时插入无响应
变量列表为空时按 `a` 插入第一个变量，所有按键均无反应。根因在 `detail_editor.rs:71` 的 `n > 0` 守卫。

## 修复方案

### 1. 重命名 — 光标移至组名位置

**改动文件**: `src/ui/main_screen.rs`

- 删除状态栏的 `if self.rename_mode` 显示块
- 在 `render_group_panel()` 末尾添加光标定位，位置为列表中选中组的名称行

### 2. 搜索 — 光标移至面板内查询行

**改动文件**: `src/ui/main_screen.rs`

- 删除状态栏的 `if self.search_mode` 显示块
- Set 面板 Block 标题改为固定 `" Search "`（不含查询内容）
- 在 Block 内部、列表上方渲染一行 `" Search: [query] "` + 光标

### 3. 空变量插入修复

**改动文件**: `src/ui/detail_editor.rs`

- `handle_variable_edit` 的按键处理条件从 `n > 0` 改为 `(n > 0 || self.insert_at.is_some())`

## 涉及文件

| 文件 | 修改项 |
|------|--------|
| `src/ui/main_screen.rs` | 1 (重命名光标) + 2 (搜索光标) |
| `src/ui/detail_editor.rs` | 3 (空变量插入) |

## 测试

- 现有 60 个测试全部通过
- 无新功能，无需新增测试
