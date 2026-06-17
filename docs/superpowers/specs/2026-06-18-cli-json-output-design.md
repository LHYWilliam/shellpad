# CLI `--json` Output — Design Spec

**Date:** 2026-06-18
**Status:** Approved
**Scope:** Add `--json` flag to `launcher search` for machine-readable output

## Problem

`launcher search` outputs ASCII tables — human-readable but not scriptable.
Integration with tools like `jq`, `fzf`, or CI pipelines requires stable
machine-readable output.

## Solution

Add a `--json` flag to the `Search` command. When set, output is
`serde_json::to_string_pretty` of a response struct instead of the current
`println!` table formatting. When not set, output is unchanged.

## New Clap flag

```rust
#[derive(Subcommand)]
enum Commands {
    Search {
        #[arg(long, conflicts_with = "group")]
        set: Option<String>,
        #[arg(long, conflicts_with = "set")]
        group: Option<String>,
        #[arg(long)]                    // ← new
        json: bool,
    },
}
```

## Output format

### Set search (`--set`)

```json
{
  "query": "deploy",
  "results": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "Prod",
      "group_name": "Deploy",
      "shell": "bash",
      "exec_mode": "stop_on_error",
      "command_count": 3
    }
  ]
}
```

### Group search (`--group`)

```json
{
  "query": "dev",
  "results": [
    {
      "id": "...",
      "name": "Development",
      "set_count": 5
    }
  ]
}
```

## Implementation

### New struct in `src/cli.rs`

```rust
#[derive(serde::Serialize)]
struct SearchResult {
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

### `handle_search` signature change

```rust
fn handle_search(data: &AppData, set_query: Option<String>, group_query: Option<String>, json: bool) {
    if json {
        // Build SearchResult, serde_json::to_string_pretty, println!
    } else {
        // Existing table formatting (unchanged)
    }
}
```

### CLI dispatch

```rust
Commands::Search { set, group, json } => {
    handle_search(&data, set, group, json);
    Some(0)
}
```

## Error handling

- No results → JSON output is `{"query": "...", "results": []}` (exit 0)
- Invalid data is impossible since data is already loaded and validated at startup

## Tests

| Test | What it verifies |
|------|-----------------|
| `search_set_json_returns_object` | `--set deploy --json` outputs valid JSON with expected fields |
| `search_group_json_returns_object` | `--group dev --json` outputs valid JSON |
| `search_no_results_json_empty_list` | Query with no matches → `results: []` |

Tests parse the output string with `serde_json::from_str::<serde_json::Value>` and
assert field presence.

## Files Affected

| File | Change |
|------|--------|
| `src/cli.rs` | Add `--json` flag, `SearchResult` structs, JSON branch in `handle_search` |

Estimated: ~40 lines production code, ~25 lines tests.
