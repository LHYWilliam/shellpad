use crate::error::CliError;
use crate::models::AppData;
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::io::{self, Write};
use uuid::Uuid;

/// shellpad — command set manager and executor
#[derive(Parser)]
#[command(name = "shellpad", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a command set
    Run {
        /// Execute by UUID
        #[arg(long, conflicts_with_all = ["group", "set"])]
        id: Option<String>,

        /// Group name (used with --set)
        #[arg(long, requires = "set", conflicts_with = "id")]
        group: Option<String>,

        /// Set name (used with --group)
        #[arg(long, requires = "group", conflicts_with = "id")]
        set: Option<String>,

        /// Variable overrides (key=value). Pass "default" to use defaults without prompting
        #[arg(long, num_args = 0..)]
        var: Vec<String>,
    },
    /// Search command sets or groups
    Search {
        /// Search command sets by name
        #[arg(long, conflicts_with = "group")]
        set: Option<String>,

        /// Search groups by name
        #[arg(long, conflicts_with = "set")]
        group: Option<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
    /// Export command sets as JSON. Use --id for a single set, --all for everything.
    Export {
        /// UUID of the command set to export
        #[arg(long, conflicts_with = "all")]
        id: Option<String>,

        /// Export all command sets
        #[arg(long, conflicts_with = "id")]
        all: bool,

        /// Output file path (writes to stdout if omitted)
        #[arg(long, short)]
        output: Option<String>,
    },
    /// Import command sets from a JSON file. Reads from stdin if no file given.
    Import {
        /// Input file path (reads from stdin if omitted)
        #[arg(long, short)]
        input: Option<String>,
    },
}

// ---- JSON output structs ----

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

// ---- Entry point ----

/// Entry point for CLI mode. Returns `Some(exit_code)` if a CLI subcommand was
/// given and handled. Returns `None` if no subcommand was given (fall through to TUI).
pub fn run_cli() -> Option<i32> {
    // Parse args. On --help/--version, clap prints and exits automatically.
    // On invalid args, clap prints error and exits with code 2.
    let cli = Cli::try_parse().unwrap_or_else(|e| e.exit());
    let command = cli.command?;

    let mut data = match crate::storage::load_app_data() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return Some(1);
        }
    };

    match command {
        Commands::Run {
            id,
            group,
            set,
            var,
        } => Some(handle_run(&data, id, group, set, var)),
        Commands::Search { set, group, json } => {
            handle_search(&data, set, group, json);
            Some(0)
        }
        Commands::Export { id, all, output } => {
            handle_export(&data, id, all, output);
            Some(0)
        }
        Commands::Import { input } => {
            handle_import(&mut data, input);
            Some(0)
        }
    }
}

// ---- Run ----

fn handle_run(
    data: &AppData,
    id: Option<String>,
    group: Option<String>,
    set: Option<String>,
    var: Vec<String>,
) -> i32 {
    // Resolve the command set
    let (set_ref, _gi, _si) = match resolve_set(data, id, group, set) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };

    // Resolve shell executable
    let shell_cmd = set_ref.shell.resolve_command();

    // Resolve variables
    let use_defaults = var.len() == 1 && var[0].eq_ignore_ascii_case("default");
    let resolved_vars = if use_defaults {
        // Build from defaults, no prompting
        set_ref
            .variables
            .iter()
            .map(|v| (v.name.clone(), v.default_value.clone()))
            .collect::<HashMap<_, _>>()
    } else {
        match resolve_variables(set_ref, &var) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Error: {}", e);
                return 1;
            }
        }
    };

    // Execute
    let working_dir = set_ref.working_dir.as_deref();
    match crate::executor::execute_set_blocking(set_ref, &shell_cmd, &resolved_vars, working_dir) {
        Ok(r) => {
            if r.failed > 0 {
                eprintln!(
                    "Completed: {}/{} succeeded, {}/{} failed",
                    r.succeeded, r.total, r.failed, r.total
                );
                1
            } else {
                0
            }
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

pub(crate) fn resolve_set(
    data: &AppData,
    id: Option<String>,
    group: Option<String>,
    set: Option<String>,
) -> Result<(&crate::models::CommandSet, usize, usize), CliError> {
    if let Some(id_str) = id {
        let uuid = Uuid::parse_str(&id_str).map_err(|_| CliError::InvalidUuid(id_str.clone()))?;
        let id_str_clone = uuid.to_string();
        let (gi, si) = data
            .find_set_by_id(uuid)
            .ok_or(CliError::SetNotFound(id_str_clone))?;
        Ok((&data.groups[gi].sets[si], gi, si))
    } else if let (Some(gname), Some(sname)) = (group, set) {
        let gl = gname.to_lowercase();
        let sl = sname.to_lowercase();
        let mut matches = Vec::new();
        for (gi, g) in data.groups.iter().enumerate() {
            if g.name.to_lowercase() == gl {
                for (si, s) in g.sets.iter().enumerate() {
                    if s.name.to_lowercase() == sl {
                        matches.push((gi, si));
                    }
                }
            }
        }
        match matches.len() {
            0 => Err(CliError::SetByGroupNotFound {
                group: gname,
                set: sname,
            }),
            1 => {
                let (gi, si) = matches[0];
                Ok((&data.groups[gi].sets[si], gi, si))
            }
            n => {
                let detail: Vec<String> = matches
                    .iter()
                    .map(|&(gi, si)| {
                        let g = &data.groups[gi];
                        let s = &g.sets[si];
                        format!("  {} | {} | {}", s.id, g.name, s.name)
                    })
                    .collect();
                Err(CliError::Ambiguous {
                    count: n,
                    detail: detail.join("\n"),
                })
            }
        }
    } else {
        Err(CliError::MissingArgs)
    }
}

pub(crate) fn resolve_variables(
    set: &crate::models::CommandSet,
    overrides: &[String],
) -> Result<HashMap<String, String>, CliError> {
    // Parse overrides into a map
    let mut overrides_map: HashMap<String, String> = HashMap::new();
    for ov in overrides {
        if let Some(eq_pos) = ov.find('=') {
            let key = ov[..eq_pos].trim().to_string();
            let val = ov[eq_pos + 1..].trim().to_string();
            overrides_map.insert(key, val);
        } else {
            return Err(CliError::InvalidVar(ov.clone()));
        }
    }

    let mut result = HashMap::new();
    for var in &set.variables {
        if let Some(val) = overrides_map.remove(&var.name) {
            result.insert(var.name.clone(), val);
        } else {
            // Prompt on stderr, read from stdin
            eprint!("{} [{}]: ", var.name, var.default_value);
            let _ = io::stderr().flush();
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(_) => {
                    let trimmed = input.trim().to_string();
                    if trimmed.is_empty() {
                        result.insert(var.name.clone(), var.default_value.clone());
                    } else {
                        result.insert(var.name.clone(), trimmed);
                    }
                }
                Err(_) => {
                    result.insert(var.name.clone(), var.default_value.clone());
                }
            }
        }
    }
    Ok(result)
}

// ---- Search ----

fn handle_search(
    data: &AppData,
    set_query: Option<String>,
    group_query: Option<String>,
    json: bool,
) {
    if json {
        let (query, results): (String, Vec<SearchItem>) = if let Some(q) = set_query {
            let items = data
                .filter_sets(&q)
                .iter()
                .map(|result| {
                    let s = result.set;
                    let gname = &data.groups[result.group_index].name;
                    SearchItem::Set(SetInfo {
                        id: s.id.to_string(),
                        name: s.name.clone(),
                        group_name: gname.clone(),
                        shell: s.shell.label(),
                        exec_mode: match s.exec_mode {
                            crate::models::ExecMode::StopOnError => "stop_on_error".to_string(),
                            crate::models::ExecMode::ContinueOnError => {
                                "continue_on_error".to_string()
                            }
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

    if let Some(query) = set_query {
        let results = data.filter_sets(&query);
        if results.is_empty() {
            eprintln!("No command sets matching '{}'", query);
            return;
        }
        println!(
            "{:<38} | {:<20} | {:<20} | {:<12} | Commands",
            "UUID", "Group", "Set Name", "Shell"
        );
        println!("{}", "-".repeat(110));
        for result in &results {
            let gname = &data.groups[result.group_index].name;
            println!(
                "{:<38} | {:<20} | {:<20} | {:<12} | {}",
                result.set.id.to_string(),
                truncate(gname, 20),
                truncate(&result.set.name, 20),
                result.set.shell.label(),
                result.set.commands.len()
            );
        }
    }

    if let Some(query) = group_query {
        let q = query.to_lowercase();
        let matching: Vec<_> = data
            .groups
            .iter()
            .filter(|g| g.name.to_lowercase().contains(&q))
            .collect();
        if matching.is_empty() {
            eprintln!("No groups matching '{}'", query);
            return;
        }
        println!("{:<38} | {:<20} | Sets", "UUID", "Group Name");
        println!("{}", "-".repeat(65));
        for g in &matching {
            println!(
                "{:<38} | {:<20} | {}",
                g.id,
                truncate(&g.name, 20),
                g.sets.len()
            );
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let end = s.floor_char_boundary(max.saturating_sub(1));
        format!("{}…", &s[..end])
    }
}

// ---- Export ----

fn handle_export(data: &AppData, id: Option<String>, all: bool, output: Option<String>) {
    let export_data = if all {
        data.clone()
    } else if let Some(id_str) = id {
        let uuid = match Uuid::parse_str(&id_str) {
            Ok(u) => u,
            Err(_) => {
                eprintln!("Invalid UUID: {}", id_str);
                return;
            }
        };
        let (gi, si) = match data.find_set_by_id(uuid) {
            Some(idx) => idx,
            None => {
                eprintln!("No command set with UUID {}", uuid);
                return;
            }
        };
        let group = &data.groups[gi];
        let set = &group.sets[si];
        crate::models::AppData {
            groups: vec![crate::models::Group {
                id: group.id,
                name: group.name.clone(),
                sets: vec![set.clone()],
            }],
        }
    } else {
        eprintln!("Specify --id <uuid> or --all");
        return;
    };

    let json = match serde_json::to_string_pretty(&export_data) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("Failed to serialize: {}", e);
            return;
        }
    };

    if let Some(path) = &output {
        match std::fs::write(path, json) {
            Ok(_) => eprintln!("Exported to {}", path),
            Err(e) => eprintln!("{}", CliError::ExportWriteFailed(e.to_string())),
        }
    } else {
        println!("{}", json);
    }
}

// ---- Import ----

fn handle_import(_data: &mut AppData, _input: Option<String>) {
    eprintln!("import: not yet implemented");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CliError;
    use crate::models::{CommandSet, Group};

    #[test]
    fn test_resolve_set_by_id() {
        let mut g = Group::new("Deploy".to_string());
        let set = CommandSet::new("Prod".to_string(), g.id);
        let set_id = set.id;
        g.sets.push(set);
        let data = AppData { groups: vec![g] };

        let result = resolve_set(&data, Some(set_id.to_string()), None, None);
        assert!(result.is_ok());
        let (found, _gi, _si) = result.unwrap();
        assert_eq!(found.name, "Prod");
    }

    #[test]
    fn test_resolve_set_by_group_and_set_name() {
        let mut g = Group::new("Deploy".to_string());
        g.sets.push(CommandSet::new("Prod".to_string(), g.id));
        let data = AppData { groups: vec![g] };

        let result = resolve_set(&data, None, Some("Deploy".into()), Some("Prod".into()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolve_set_not_found() {
        let data = AppData::empty();
        let result = resolve_set(&data, None, Some("Missing".into()), Some("Missing".into()));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CliError::SetByGroupNotFound { .. }
        ));
    }

    #[test]
    fn test_resolve_set_no_args() {
        let data = AppData::empty();
        let result = resolve_set(&data, None, None, None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CliError::MissingArgs));
    }

    #[test]
    fn test_resolve_set_invalid_uuid() {
        let data = AppData::empty();
        let result = resolve_set(&data, Some("not-a-uuid".into()), None, None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CliError::InvalidUuid(_)));
    }

    #[test]
    fn test_resolve_set_ambiguous() {
        let mut g = Group::new("G".to_string());
        let set = CommandSet::new("S".to_string(), g.id);
        g.sets.push(set);
        let set2 = CommandSet::new("S".to_string(), g.id);
        g.sets.push(set2);
        let data = AppData { groups: vec![g] };

        let result = resolve_set(&data, None, Some("G".into()), Some("S".into()));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CliError::Ambiguous { .. }));
    }

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        let long = "a".repeat(50);
        let result = truncate(&long, 10);
        assert_eq!(result.chars().count(), 10);
        assert!(result.ends_with('…'));
    }

    #[test]
    fn test_truncate_empty() {
        assert_eq!(truncate("", 5), "");
    }

    #[test]
    fn test_use_defaults_branch_lowercase() {
        let var = vec!["default".to_string()];
        assert!(var.len() == 1 && var[0].eq_ignore_ascii_case("default"));
    }

    #[test]
    fn test_use_defaults_branch_not_default() {
        let var = vec!["host=prod".to_string()];
        assert!(!(var.len() == 1 && var[0].eq_ignore_ascii_case("default")));
    }

    #[test]
    fn test_parse_var_overrides_valid() {
        let overrides: Vec<String> = vec!["key=value".into(), "a=b".into()];
        let mut overrides_map = std::collections::HashMap::new();
        for ov in overrides {
            if let Some(eq_pos) = ov.find('=') {
                let key = ov[..eq_pos].trim().to_string();
                let val = ov[eq_pos + 1..].trim().to_string();
                overrides_map.insert(key, val);
            }
        }
        assert_eq!(overrides_map.len(), 2);
        assert_eq!(overrides_map.get("key").unwrap(), "value");
    }

    #[test]
    fn test_search_set_json_returns_valid_object() {
        let mut g = Group::new("Deploy".to_string());
        g.sets.push(CommandSet::new("Prod".to_string(), g.id));
        let data = AppData { groups: vec![g] };

        let items: Vec<SearchItem> = data
            .filter_sets("P")
            .iter()
            .map(|result| {
                SearchItem::Set(SetInfo {
                    id: result.set.id.to_string(),
                    name: result.set.name.clone(),
                    group_name: data.groups[result.group_index].name.clone(),
                    shell: result.set.shell.label(),
                    exec_mode: "stop_on_error".to_string(),
                    command_count: result.set.commands.len(),
                })
            })
            .collect();
        let output = SearchOutput {
            query: "P".to_string(),
            results: items,
        };
        let json = serde_json::to_string_pretty(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["query"], "P");
        assert_eq!(parsed["results"].as_array().unwrap().len(), 1);
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
        let output = SearchOutput {
            query: "d".to_string(),
            results: items,
        };
        let json = serde_json::to_string_pretty(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["results"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_search_no_results_json_empty_list() {
        let _data = AppData::empty();
        let output = SearchOutput {
            query: "none".to_string(),
            results: vec![],
        };
        let json = serde_json::to_string_pretty(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["results"].as_array().unwrap().is_empty());
    }

    // ---- Export tests ----

    #[test]
    fn test_export_all_creates_valid_json() {
        let mut g = Group::new("Deploy".to_string());
        let set = CommandSet::new("Prod".to_string(), g.id);
        g.sets.push(set);
        let data = AppData { groups: vec![g] };

        let export_data = data.clone();
        let json = serde_json::to_string_pretty(&export_data).unwrap();
        let deserialized: AppData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.groups.len(), 1);
        assert_eq!(deserialized.groups[0].name, "Deploy");
        assert_eq!(deserialized.groups[0].sets.len(), 1);
        assert_eq!(deserialized.groups[0].sets[0].name, "Prod");
    }

    #[test]
    fn test_export_single_set_minimal_appdata() {
        let mut g = Group::new("Deploy".to_string());
        let set = CommandSet::new("Prod".to_string(), g.id);
        let set_id = set.id;
        g.sets.push(set);
        let data = AppData { groups: vec![g] };

        let (gi, si) = data.find_set_by_id(set_id).unwrap();
        let group = &data.groups[gi];
        let set = &group.sets[si];
        let export_data = AppData {
            groups: vec![Group {
                id: group.id,
                name: group.name.clone(),
                sets: vec![set.clone()],
            }],
        };

        let json = serde_json::to_string_pretty(&export_data).unwrap();
        let deserialized: AppData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.groups.len(), 1);
        assert_eq!(deserialized.groups[0].sets.len(), 1);
        assert_eq!(deserialized.groups[0].sets[0].id, set_id);
    }

    #[test]
    fn test_export_nonexistent_id_returns_none() {
        let data = AppData::empty();
        let random_id = Uuid::new_v4();
        let found = data.find_set_by_id(random_id);
        assert!(found.is_none());
    }
}
