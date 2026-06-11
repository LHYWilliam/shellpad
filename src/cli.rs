use crate::models::AppData;
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::io::{self, Write};
use uuid::Uuid;

/// Launcher — command set manager and executor
#[derive(Parser)]
#[command(name = "launcher", version, about)]
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
    },
}

/// Entry point for CLI mode. Returns `Some(exit_code)` if a CLI subcommand was
/// given and handled. Returns `None` if no subcommand was given (fall through to TUI).
pub fn run_cli() -> Option<i32> {
    // Parse args. On --help/--version, clap prints and exits automatically.
    // On invalid args, clap prints error and exits with code 2.
    let cli = Cli::try_parse().unwrap_or_else(|e| e.exit());
    let command = cli.command?;

    let data = match crate::storage::load_app_data() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: {}", e);
            return Some(1);
        }
    };

    match command {
        Commands::Run { id, group, set, var } => {
            Some(handle_run(&data, id, group, set, var))
        }
        Commands::Search { set, group } => {
            handle_search(&data, set, group);
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
            eprintln!("{}", e);
            return 1;
        }
    };

    // Resolve shell executable
    let shell = set_ref.shell.resolve_executable();

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
                eprintln!("{}", e);
                return 1;
            }
        }
    };

    // Execute
    match crate::executor::execute_set_blocking(set_ref, &shell, &resolved_vars) {
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
            eprintln!("Execution error: {}", e);
            1
        }
    }
}

fn resolve_set<'a>(
    data: &'a AppData,
    id: Option<String>,
    group: Option<String>,
    set: Option<String>,
) -> Result<(&'a crate::models::CommandSet, usize, usize), String> {
    if let Some(id_str) = id {
        let uuid = Uuid::parse_str(&id_str).map_err(|_| format!("Invalid UUID: {}", id_str))?;
        let (gi, si) = data
            .find_set_by_id(uuid)
            .ok_or_else(|| format!("No command set with UUID {}", uuid))?;
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
            0 => Err(format!("No command set found for group '{}' set '{}'", gname, sname)),
            1 => {
                let (gi, si) = matches[0];
                Ok((&data.groups[gi].sets[si], gi, si))
            }
            n => {
                let mut msg = format!("Ambiguous: found {} matches:\n", n);
                for &(gi, si) in &matches {
                    let g = &data.groups[gi];
                    let s = &g.sets[si];
                    msg.push_str(&format!("  {} | {} | {}\n", s.id, g.name, s.name));
                }
                Err(msg)
            }
        }
    } else {
        Err("Specify --id <uuid> or --group <name> --set <name>".to_string())
    }
}

fn resolve_variables(
    set: &crate::models::CommandSet,
    overrides: &[String],
) -> Result<HashMap<String, String>, String> {
    // Parse overrides into a map
    let mut overrides_map: HashMap<String, String> = HashMap::new();
    for ov in overrides {
        if let Some(eq_pos) = ov.find('=') {
            let key = ov[..eq_pos].trim().to_string();
            let val = ov[eq_pos + 1..].trim().to_string();
            overrides_map.insert(key, val);
        } else {
            return Err(format!("Invalid --var format '{}' (expected key=value)", ov));
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

fn handle_search(data: &AppData, set_query: Option<String>, group_query: Option<String>) {
    if let Some(query) = set_query {
        let results = data.filter_sets(&query);
        if results.is_empty() {
            eprintln!("No command sets matching '{}'", query);
            return;
        }
        println!("{:<38} | {:<20} | {:<20} | {:<12} | Commands", "UUID", "Group", "Set Name", "Shell");
        println!("{}", "-".repeat(110));
        for &(gi, _si, s) in &results {
            let gname = &data.groups[gi].name;
            println!(
                "{:<38} | {:<20} | {:<20} | {:<12} | {}",
                s.id.to_string(),
                truncate(gname, 20),
                truncate(&s.name, 20),
                s.shell.label(),
                s.commands.len()
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
            println!("{:<38} | {:<20} | {}", g.id, truncate(&g.name, 20), g.sets.len());
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

#[cfg(test)]
mod tests {
    use super::*;
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
        assert!(result.unwrap_err().contains("No command set found"));
    }

    #[test]
    fn test_resolve_set_no_args() {
        let data = AppData::empty();
        let result = resolve_set(&data, None, None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Specify"));
    }

    #[test]
    fn test_resolve_set_invalid_uuid() {
        let data = AppData::empty();
        let result = resolve_set(&data, Some("not-a-uuid".into()), None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid UUID"));
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
        assert!(result.unwrap_err().contains("Ambiguous"));
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
}
