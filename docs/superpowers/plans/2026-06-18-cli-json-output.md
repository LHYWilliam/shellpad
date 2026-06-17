# CLI `--json` Output — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `--json` flag to `launcher search` subcommand for machine-readable JSON output.

**Architecture:** `Search` subcommand gains `#[arg(long)] json: bool`. Two new `#[derive(Serialize)]` structs for set and group results. `handle_search` gains a `json` parameter; when `true`, builds the result struct and calls `serde_json::to_string_pretty`. 3 new tests verify JSON output.

**Tech Stack:** serde_json (existing dependency)

---

### Task: Add --json flag, structs, JSON branch, tests

**Files:**
- Modify: `src/cli.rs`

- [ ] **Step 1: Add --json flag to Search subcommand**

```rust
    Search {
        #[arg(long, conflicts_with = "group")]
        set: Option<String>,
        #[arg(long, conflicts_with = "set")]
        group: Option<String>,
        #[arg(long)]
        json: bool,
    },
```

- [ ] **Step 2: Add JSON output structs**

Before `fn run_cli`, after `enum Commands`:

```rust
#[derive(serde::Serialize)]
struct SearchOutput {
    query: String,
    results: Vec<SearchItem>,
}

#[derive(serde::Serialize)]
#[serde(tag = "type")]
enum SearchItem {
    Set(SetInfo),
    Group(GroupInfo),
}

#[derive(serde::Serialize)]
struct SetInfo {
    id: String,
    name: String,
    group_name: String,
    shell: String,
    exec_mode: String,
    command_count: usize,
}

#[derive(serde::Serialize)]
struct GroupInfo {
    id: String,
    name: String,
    set_count: usize,
}
```

- [ ] **Step 3: Update handle_search signature and dispatch**

Signature:
```rust
fn handle_search(data: &AppData, set_query: Option<String>, group_query: Option<String>, json: bool) {
```

Dispatch (line ~72):
```rust
        Commands::Search { set, group, json } => {
            handle_search(&data, set, group, json);
            Some(0)
        }
```

- [ ] **Step 4: Add JSON branch in handle_search**

Before the set-query branch (before `if let Some(query) = set_query {`), add a `json` guard. Replace the entire `handle_search` body with a dual-mode version:

```rust
fn handle_search(data: &AppData, set_query: Option<String>, group_query: Option<String>, json: bool) {
    if json {
        let (query, results): (String, Vec<SearchItem>) = if let Some(q) = set_query {
            let items = data
                .filter_sets(&q)
                .iter()
                .map(|&(gi, _si, s)| {
                    let gname = &data.groups[gi].name;
                    SearchItem::Set(SetInfo {
                        id: s.id.to_string(),
                        name: s.name.clone(),
                        group_name: gname.clone(),
                        shell: s.shell.label(),
                        exec_mode: match s.exec_mode {
                            crate::models::ExecMode::StopOnError => "stop_on_error".to_string(),
                            crate::models::ExecMode::ContinueOnError => "continue_on_error".to_string(),
                        },
                        command_count: s.commands.len(),
                    })
                })
                .collect();
            (q, items)
        } else if let Some(q) = group_query {
            let items = data
                .groups
                .iter()
                .filter(|g| g.name.to_lowercase().contains(&q.to_lowercase()))
                .map(|g| {
                    SearchItem::Group(GroupInfo {
                        id: g.id.to_string(),
                        name: g.name.clone(),
                        set_count: g.sets.len(),
                    })
                })
                .collect();
            (q, items)
        } else {
            return;
        };
        let output = SearchOutput { query, results };
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
        return;
    }

    // Existing table formatting (unchanged)
    if let Some(query) = set_query {
        ...
```

- [ ] **Step 5: Write 3 tests**

Add to `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn test_search_set_json_returns_valid_object() {
        let mut g = Group::new("Deploy".to_string());
        g.sets.push(CommandSet::new("Prod".to_string(), g.id));
        let data = AppData { groups: vec![g] };

        // Capture stdout
        let output = std::panic::catch_unwind(|| {
            // Can't directly capture stdout in tests; test via serde round-trip
            let items: Vec<SearchItem> = data
                .filter_sets("P")
                .iter()
                .map(|&(gi, _si, s)| {
                    SearchItem::Set(SetInfo {
                        id: s.id.to_string(),
                        name: s.name.clone(),
                        group_name: data.groups[gi].name.clone(),
                        shell: s.shell.label(),
                        exec_mode: "stop_on_error".to_string(),
                        command_count: s.commands.len(),
                    })
                })
                .collect();
            let output = SearchOutput { query: "P".to_string(), results: items };
            let json = serde_json::to_string_pretty(&output).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed["query"], "P");
            assert_eq!(parsed["results"].as_array().unwrap().len(), 1);
        });
        assert!(output.is_ok(), "JSON serialization should succeed");
    }

    #[test]
    fn test_search_group_json_returns_valid_object() {
        let mut g = Group::new("Dev".to_string());
        g.sets.push(CommandSet::new("S".to_string(), g.id));
        let data = AppData { groups: vec![g] };

        let items: Vec<SearchItem> = data
            .groups
            .iter()
            .filter(|grp| grp.name.to_lowercase().contains("d"))
            .map(|grp| {
                SearchItem::Group(GroupInfo {
                    id: grp.id.to_string(),
                    name: grp.name.clone(),
                    set_count: grp.sets.len(),
                })
            })
            .collect();
        let output = SearchOutput { query: "d".to_string(), results: items };
        let json = serde_json::to_string_pretty(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["results"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_search_no_results_json_empty_list() {
        let data = AppData::empty();
        let output = SearchOutput { query: "none".to_string(), results: vec![] };
        let json = serde_json::to_string_pretty(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["results"].as_array().unwrap().is_empty());
    }
```

- [ ] **Step 6: Verify compilation and tests**

Run: `cargo check && cargo test`
Expected: All tests PASS (228 + 3 = 231)

- [ ] **Step 7: Commit**

```bash
git add src/cli.rs
git commit -m "feat: add --json flag to launcher search subcommand

Search output in JSON format for scripting integration.
Two item types: Set (id, name, group, shell, count) and
Group (id, name, set_count). 3 new tests.

Co-Authored-By: Claude <noreply@anthropic.com>"
```
