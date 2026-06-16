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
    fn test_filter_sets() {
        let mut g = Group::new("Dev".to_string());
        g.sets
            .push(CommandSet::new("Deploy Backend".to_string(), g.id));
        g.sets
            .push(CommandSet::new("Build Frontend".to_string(), g.id));
        g.sets
            .push(CommandSet::new("Database Migrate".to_string(), g.id));
        let data = AppData { groups: vec![g] };
        let results = data.filter_sets("backend");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].2.name, "Deploy Backend");
    }
}
