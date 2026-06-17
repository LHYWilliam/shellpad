use crate::models::types::{AppData, CommandSet, Group};
use crate::ui::main_screen::search::fuzzy_match;
use uuid::Uuid;

/// Result of filtering command sets by search query.
/// Carries pre-computed match positions for render-side highlighting.
pub struct FilterResult<'a> {
    pub group_index: usize,
    pub set_index: usize,
    pub set: &'a CommandSet,
    /// Byte-offset pairs in `set.name` where fuzzy_match found query chars.
    /// Empty if the match was in command text rather than set name.
    pub name_matches: Vec<(usize, usize)>,
}

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

    /// Filter command sets whose name or commands fuzzy-match `query`.
    /// Empty query returns all sets.
    pub fn filter_sets(&self, query: &str) -> Vec<FilterResult<'_>> {
        if query.is_empty() {
            return self
                .groups
                .iter()
                .enumerate()
                .flat_map(|(gi, g)| {
                    g.sets.iter().enumerate().map(move |(si, s)| FilterResult {
                        group_index: gi,
                        set_index: si,
                        set: s,
                        name_matches: Vec::new(),
                    })
                })
                .collect();
        }

        let mut results = Vec::new();
        for (gi, group) in self.groups.iter().enumerate() {
            for (si, set) in group.sets.iter().enumerate() {
                // Try matching set name first
                if let Some(matches) = fuzzy_match(&set.name, query) {
                    results.push(FilterResult {
                        group_index: gi,
                        set_index: si,
                        set,
                        name_matches: matches,
                    });
                    continue;
                }
                // Fall back: search in command text
                let cmd_match = set
                    .commands
                    .iter()
                    .any(|cmd| fuzzy_match(&cmd.command, query).is_some());
                if cmd_match {
                    results.push(FilterResult {
                        group_index: gi,
                        set_index: si,
                        set,
                        name_matches: Vec::new(),
                    });
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
        let data = AppData {
            groups: vec![group],
        };
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
        let data = AppData {
            groups: vec![g1, g2],
        };
        let names: Vec<&str> = data.all_sets_iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["S1", "S2", "S3"]);
    }

    #[test]
    fn test_filter_sets_fuzzy_name() {
        let mut g = Group::new("Dev".to_string());
        g.sets
            .push(CommandSet::new("Deploy Backend".to_string(), g.id));
        g.sets
            .push(CommandSet::new("Build Frontend".to_string(), g.id));
        g.sets
            .push(CommandSet::new("Database Migrate".to_string(), g.id));
        let data = AppData { groups: vec![g] };
        // "dpl" fuzzy-matches "Deploy" → finds "Deploy Backend"
        let results = data.filter_sets("dpl");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].set.name, "Deploy Backend");
        assert!(!results[0].name_matches.is_empty());
    }

    #[test]
    fn test_filter_sets_command_text() {
        let mut g = Group::new("Dev".to_string());
        let mut set = CommandSet::new("My Tasks".to_string(), g.id);
        set.commands.push(crate::models::Command {
            position: 0,
            command: "curl https://api.example.com".to_string(),
        });
        g.sets.push(set);
        let data = AppData { groups: vec![g] };
        let results = data.filter_sets("curl");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].set.name, "My Tasks");
        // Matched in command text, not name
        assert!(results[0].name_matches.is_empty());
    }
}
