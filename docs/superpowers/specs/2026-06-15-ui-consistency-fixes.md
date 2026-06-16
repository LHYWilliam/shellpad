# UI 同步与一致性修复 — 设计文档

## 问题

经过 33 轮 UI 增强后，应用在多个交互细节上存在同步延迟和视觉不一致：

### 同步问题
1. 分组重命名时列表中名称不实时更新
2. 详情屏外边框标题在编辑名称时不实时更新

### 焦点反馈
3. Properties 块边框在焦点时不亮起（与 Variables/Commands 块不一致）
4. 名称编辑缺少背景高亮（与变量/命令内联编辑不一致）

### 空状态
5. Sets 面板、Variables 列表、Commands 列表没有空状态提示

### 视觉一致性
6. 详情屏状态栏缺少分隔线（与主屏不一致）
7. 执行屏输出块缺少标题（与其他所有块不一致）

## 修复方案

### 1. 分组重命名实时同步
`main_screen.rs` `render_group_panel()` — 渲染选中的分组时使用 `self.rename_input.content`

### 2. 详情屏外边框标题实时同步
`detail_screen.rs` — 编辑名称时用 `self.name_input.content` 替代 `self.set.name`

### 3. Properties 块焦点边框
`detail_screen.rs` — 焦点在 Name/Group/Shell/ExecMode 时 border 变为 `accent_primary`

### 4. 名称编辑背景高亮
`detail_screen.rs` — 编辑时使用 `text_on_selected` + `accent_primary` 背景 + BOLD

### 5. 空状态提示
- `main_screen.rs` — Sets 面板空时显示 "(empty — press n to add)"
- `detail_screen.rs` — Variables 空时显示 "(empty — press a to add)"
- `detail_screen.rs` — Commands 空时显示 "(empty — press a to add)"

### 6. 详情屏状态栏分隔线
`detail_screen.rs` — 添加 `─` 分隔线（与主屏一致）

### 7. 执行屏输出块标题
`execution_screen.rs` — 添加 " Output " 标题

## 涉及文件

| 文件 | 修改项 |
|------|--------|
| `src/ui/main_screen.rs` | 1, 5 (Sets) |
| `src/ui/detail_screen.rs` | 2, 3, 4, 5 (Vars+Commands), 6 |
| `src/ui/execution_screen.rs` | 7 |

## 测试

- 现有 60 个测试全部通过
- 无新功能，纯 UI 修复，无需新增测试
