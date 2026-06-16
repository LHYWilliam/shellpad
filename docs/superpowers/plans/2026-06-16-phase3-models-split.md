# Phase 3: `models/` 拆分 — 详细实施计划

> **For agentic workers:** 此计划是 Phase 3 的详细实施步骤。使用 `superpowers:subagent-driven-development` 执行。

**Goal:** 将 `src/models.rs` (441 行) 拆分为 `models/types.rs`（类型定义 + 标签方法 + 构造函数）和 `models/queries.rs`（AppData 查询方法），`models.rs` 作为模块根保留 re-export。

**范围:**
- 创建 2 个新文件：`models/types.rs`、`models/queries.rs`
- 精简 1 个文件：`models.rs` 从 441 行减到 ~30 行（子模块声明 + re-export + 测试转发）
- 不修改任何其他文件（models.rs 的 `pub use` re-export 使外部 import 路径不变）

**验证:** `cargo check` → `cargo test`(≥60) → `cargo clippy` → `cargo fmt`

---

## 代码分布

### `models/types.rs` (~250 行)

从当前 `models.rs` 提取：

```
第 1-3 行:  use 导入 (chrono, serde, uuid)
第 13-17 行: ShellCommand struct
第 20-152 行: ShellType enum + impl (executable, resolve_executable, resolve_command, label, builtin_variants)
第 155-170 行: ExecMode enum + impl (label)
第 177-181 行: Variable struct
第 184-190 行: Command struct
第 193-239 行: CommandSet struct + impl (new), Group struct + impl (new), AppData struct + AppData::empty()
第 300-341 行: 类型测试 (test_app_data_empty, test_create_group, test_create_command_set,
                test_serde_roundtrip_app_data, test_serde_roundtrip_shell_types,
                test_serde_roundtrip_exec_mode)
第 416-440 行: 标签测试 (test_shell_label, test_exec_mode_label, test_builtin_variants_count,
                test_command_set_new_sets_updated_at)
```

### `models/queries.rs` (~80 行)

从当前 `models.rs` 提取：

```
第 254-293 行: AppData 查询方法 (all_sets_iter, find_group_by_id, find_group_by_id_mut,
               find_set_by_id, filter_sets)
第 344-413 行: 查询测试 (test_find_set_by_id, test_all_sets_iter, test_filter_sets)
```

### `models.rs` (模块根，~30 行)

```rust
pub(crate) mod types;
pub(crate) mod queries;

pub use types::*;
pub use queries::*;
```

---

### Task 1: 创建 `models/types.rs`

- [ ] **Step 1: 创建 `models/types.rs`**

从当前 `models.rs` 提取类型定义 + 类型相关方法 + 类型标签测试。

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A resolved shell executable and the flag used to pass a command string.
#[derive(Debug, Clone)]
pub struct ShellCommand {
    pub program: String,
    pub flag: String,
}

/// Supported shell types for executing commands.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ShellType {
    #[serde(rename = "system_default")]
    SystemDefault,
    #[serde(rename = "bash")]
    Bash,
    #[serde(rename = "zsh")]
    Zsh,
    #[serde(rename = "fish")]
    Fish,
    #[serde(rename = "powershell")]
    PowerShell,
    #[serde(rename = "custom")]
    Custom(String),
}

impl ShellType {
    pub fn executable(&self) -> Option<&str> {
        match self {
            ShellType::SystemDefault => None,
            ShellType::Bash => Some("bash"),
            ShellType::Zsh => Some("zsh"),
            ShellType::Fish => Some("fish"),
            ShellType::PowerShell => None,
            ShellType::Custom(path) => Some(path.as_str()),
        }
    }

    pub fn resolve_executable(&self) -> String {
        match self {
            ShellType::SystemDefault => std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string()),
            ShellType::Bash => "bash".to_string(),
            ShellType::Zsh => "zsh".to_string(),
            ShellType::Fish => "fish".to_string(),
            ShellType::PowerShell => {
                #[cfg(windows)]
                { "powershell.exe".to_string() }
                #[cfg(not(windows))]
                { "pwsh".to_string() }
            }
            ShellType::Custom(path) => path.clone(),
        }
    }

    pub fn resolve_command(&self) -> ShellCommand {
        match self {
            ShellType::SystemDefault => {
                #[cfg(windows)]
                {
                    let comspec = std::env::var("ComSpec").unwrap_or_else(|_| "cmd.exe".to_string());
                    ShellCommand { program: comspec, flag: "/C".to_string() }
                }
                #[cfg(not(windows))]
                {
                    let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
                    ShellCommand { program: shell, flag: "-c".to_string() }
                }
            }
            ShellType::Bash | ShellType::Zsh | ShellType::Fish => ShellCommand {
                program: self.executable().unwrap().to_string(),
                flag: "-c".to_string(),
            },
            ShellType::PowerShell => {
                #[cfg(windows)]
                { ShellCommand { program: "powershell.exe".to_string(), flag: "-Command".to_string() } }
                #[cfg(not(windows))]
                { ShellCommand { program: "pwsh".to_string(), flag: "-Command".to_string() } }
            }
            ShellType::Custom(path) => {
                let lower = path.to_lowercase();
                let flag = if lower.contains("cmd.exe") || lower.contains("cmd ") {
                    "/C"
                } else if lower.contains("powershell") {
                    "-Command"
                } else {
                    "-c"
                };
                ShellCommand { program: path.clone(), flag: flag.to_string() }
            }
        }
    }

    pub fn label(&self) -> String {
        match self {
            ShellType::SystemDefault => "System Default".to_string(),
            ShellType::Bash => "bash".to_string(),
            ShellType::Zsh => "zsh".to_string(),
            ShellType::Fish => "fish".to_string(),
            ShellType::PowerShell => "PowerShell".to_string(),
            ShellType::Custom(path) => format!("custom: {}", path),
        }
    }

    pub fn builtin_variants() -> Vec<ShellType> {
        vec![
            ShellType::SystemDefault,
            ShellType::Bash,
            ShellType::Zsh,
            ShellType::Fish,
            ShellType::PowerShell,
        ]
    }
}

/// Execution mode for a command set.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecMode {
    #[serde(rename = "stop_on_error")]
    StopOnError,
    #[serde(rename = "continue_on_error")]
    ContinueOnError,
}

impl ExecMode {
    pub fn label(&self) -> &str {
        match self {
            ExecMode::StopOnError => "Stop on Error",
            ExecMode::ContinueOnError => "Continue on Error",
        }
    }
}

/// A template variable with a default value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Variable {
    pub name: String,
    pub default_value: String,
}

/// A single shell command within a command set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Command {
    pub position: usize,
    pub command: String,
}

/// A named collection of commands to execute.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandSet {
    pub id: Uuid,
    pub name: String,
    pub group_id: Uuid,
    pub shell: ShellType,
    pub exec_mode: ExecMode,
    pub variables: Vec<Variable>,
    pub commands: Vec<Command>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CommandSet {
    pub fn new(name: String, group_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            group_id,
            shell: ShellType::SystemDefault,
            exec_mode: ExecMode::StopOnError,
            variables: Vec::new(),
            commands: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// A group (folder) that organises command sets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Group {
    pub id: Uuid,
    pub name: String,
    pub sets: Vec<CommandSet>,
}

impl Group {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            sets: Vec::new(),
        }
    }
}

/// Root data structure for the entire application.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppData {
    pub groups: Vec<Group>,
}

impl AppData {
    pub fn empty() -> Self {
        Self { groups: Vec::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_data_empty() {
        let data = AppData::empty();
        assert!(data.groups.is_empty());
    }

    #[test]
    fn test_create_group() {
        let group = Group::new("Deploy".to_string());
        assert_eq!(group.name, "Deploy");
        assert!(group.sets.is_empty());
    }

    #[test]
    fn test_create_command_set() {
        let group = Group::new("Deploy".to_string());
        let set = CommandSet::new("Deploy to Prod".to_string(), group.id);
        assert_eq!(set.name, "Deploy to Prod");
        assert_eq!(set.group_id, group.id);
        assert_eq!(set.shell, ShellType::SystemDefault);
        assert_eq!(set.exec_mode, ExecMode::StopOnError);
        assert!(set.variables.is_empty());
        assert!(set.commands.is_empty());
    }

    #[test]
    fn test_serde_roundtrip_app_data() {
        let mut group = Group::new("Test".to_string());
        let set = CommandSet::new("Test Set".to_string(), group.id);
        group.sets.push(set);
        let data = AppData { groups: vec![group] };
        let json = serde_json::to_string_pretty(&data).expect("serialize");
        let deserialized: AppData = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(data, deserialized);
    }

    #[test]
    fn test_serde_roundtrip_shell_types() {
        for shell in &[
            ShellType::SystemDefault,
            ShellType::Bash,
            ShellType::Zsh,
            ShellType::Fish,
            ShellType::Custom("/usr/bin/zsh".to_string()),
        ] {
            let json = serde_json::to_string(shell).expect("serialize");
            let deserialized: ShellType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*shell, deserialized);
        }
    }

    #[test]
    fn test_serde_roundtrip_exec_mode() {
        for mode in &[ExecMode::StopOnError, ExecMode::ContinueOnError] {
            let json = serde_json::to_string(mode).expect("serialize");
            let deserialized: ExecMode = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*mode, deserialized);
        }
    }

    #[test]
    fn test_shell_label() {
        assert_eq!(ShellType::Bash.label(), "bash");
        assert_eq!(ShellType::SystemDefault.label(), "System Default");
        assert_eq!(ShellType::Zsh.label(), "zsh");
    }

    #[test]
    fn test_exec_mode_label() {
        assert_eq!(ExecMode::StopOnError.label(), "Stop on Error");
        assert_eq!(ExecMode::ContinueOnError.label(), "Continue on Error");
    }

    #[test]
    fn test_builtin_variants_count() {
        assert_eq!(ShellType::builtin_variants().len(), 5);
    }

    #[test]
    fn test_command_set_new_sets_updated_at() {
        let group_id = Uuid::new_v4();
        let set = CommandSet::new("Test".to_string(), group_id);
        assert_eq!(set.created_at, set.updated_at);
        assert!(set.created_at <= Utc::now());
    }
}
```

### Task 2: 创建 `models/queries.rs`

- [ ] **Step 2: 创建 `models/queries.rs`**

从当前 `models.rs` 提取 AppData 查询方法 + 查询测试。

```rust
use crate::models::types::{AppData, CommandSet, Group};
use uuid::Uuid;

impl AppData {
    /// Iterate over every command set across all groups.
    pub fn all_sets_iter(&self) -> impl Iterator<Item = &CommandSet> {
        self.groups.iter().flat_map(|g| g.sets.iter())
    }

    /// Find a group by its ID.
    pub fn find_group_by_id(&self, id: Uuid) -> Option<&Group> {
        self.groups.iter().find(|g| g.id == id)
    }

    /// Find a group by its ID (mutable).
    pub fn find_group_by_id_mut(&mut self, id: Uuid) -> Option<&mut Group> {
        self.groups.iter_mut().find(|g| g.id == id)
    }

    /// Find a command set by its ID (returns (group_index, set_index)).
    pub fn find_set_by_id(&self, id: Uuid) -> Option<(usize, usize)> {
        for (gi, group) in self.groups.iter().enumerate() {
            for (si, set) in group.sets.iter().enumerate() {
                if set.id == id {
                    return Some((gi, si));
                }
            }
        }
        None
    }

    /// Filter command sets whose name contains `query` (case-insensitive).
    pub fn filter_sets(&self, query: &str) -> Vec<(usize, usize, &CommandSet)> {
        let q = query.to_lowercase();
        let mut results = Vec::new();
        for (gi, group) in self.groups.iter().enumerate() {
            for (si, set) in group.sets.iter().enumerate() {
                if set.name.to_lowercase().contains(&q) {
                    results.push((gi, si, set));
                }
            }
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use crate::models::types::{AppData, CommandSet, Group};

    #[test]
    fn test_find_set_by_id() {
        let mut group = Group::new("G".to_string());
        let set = CommandSet::new("S".to_string(), group.id);
        let set_id = set.id;
        group.sets.push(set);
        let data = AppData { groups: vec![group] };
        let found = data.find_set_by_id(set_id);
        assert!(found.is_some());
        let (gi, si) = found.unwrap();
        assert_eq!(data.groups[gi].sets[si].name, "S");
    }

    #[test]
    fn test_all_sets_iter() {
        let mut g1 = Group::new("G1".to_string());
        g1.sets.push(CommandSet::new("S1".to_string(), g1.id));
        g1.sets.push(CommandSet::new("S2".to_string(), g1.id));
        let mut g2 = Group::new("G2".to_string());
        g2.sets.push(CommandSet::new("S3".to_string(), g2.id));
        let data = AppData { groups: vec![g1, g2] };
        let names: Vec<&str> = data.all_sets_iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["S1", "S2", "S3"]);
    }

    #[test]
    fn test_filter_sets() {
        let mut g = Group::new("Dev".to_string());
        g.sets.push(CommandSet::new("Deploy Backend".to_string(), g.id));
        g.sets.push(CommandSet::new("Build Frontend".to_string(), g.id));
        g.sets.push(CommandSet::new("Database Migrate".to_string(), g.id));
        let data = AppData { groups: vec![g] };
        let results = data.filter_sets("backend");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].2.name, "Deploy Backend");
    }
}
```

### Task 3: 精简 `models.rs` 为模块根

- [ ] **Step 3: 用以下内容替换 `models.rs`**

```rust
pub(crate) mod types;
pub(crate) mod queries;

pub use types::*;
pub use queries::*;
```

### Task 4: 编译验证

- [ ] **Step 4.1: 编译检查**

```bash
cargo check 2>&1
```
预期输出：无错误（仅先前已有的 warning）。

如果报错，检查：
- `models/queries.rs` 中的 `use crate::models::types::{...}` 路径是否正确
- 外部文件如 `app/handler.rs`、`action.rs` 中的 `use crate::models::*` 是否仍能解析（通过 re-export）

- [ ] **Step 4.2: 单元测试**

```bash
cargo test 2>&1
```
预期：全部测试通过（≥60，12 个 models 测试全在）。

- [ ] **Step 4.3: Clippy**

```bash
cargo clippy 2>&1
```
预期：无新增 warning。

- [ ] **Step 4.4: 格式化 + 提交**

```bash
cargo fmt
git add src/models.rs src/models/
git commit -m "refactor(phase3): 拆分 models.rs → models/types.rs + queries.rs"
```
