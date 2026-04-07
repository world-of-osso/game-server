use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Account-wide collection of learned item appearances.
///
/// Each appearance is identified by its `display_info_id` (u32). Once an item
/// is equipped or bound, its appearance is added to the collection and can be
/// used for transmogrification on any character on the account.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AppearanceCollection {
    appearances: BTreeSet<u32>,
}

impl AppearanceCollection {
    /// Learn a new appearance. Returns `true` if it was newly added.
    pub fn learn(&mut self, display_info_id: u32) -> bool {
        self.appearances.insert(display_info_id)
    }

    /// Whether this appearance has been learned.
    pub fn has(&self, display_info_id: u32) -> bool {
        self.appearances.contains(&display_info_id)
    }

    /// Number of learned appearances.
    pub fn count(&self) -> usize {
        self.appearances.len()
    }

    /// Iterator over all learned appearance IDs (sorted).
    pub fn all(&self) -> impl Iterator<Item = u32> + '_ {
        self.appearances.iter().copied()
    }

    /// Learn multiple appearances at once. Returns the count of newly added.
    pub fn learn_many(&mut self, ids: impl IntoIterator<Item = u32>) -> usize {
        ids.into_iter().filter(|&id| self.learn(id)).count()
    }

    /// Remove an appearance (for GM commands). Returns `true` if it was present.
    pub fn remove(&mut self, display_info_id: u32) -> bool {
        self.appearances.remove(&display_info_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_collection() {
        let collection = AppearanceCollection::default();
        assert_eq!(collection.count(), 0);
        assert!(!collection.has(100));
        assert_eq!(collection.all().count(), 0);
    }

    #[test]
    fn learn_new_appearance() {
        let mut collection = AppearanceCollection::default();
        assert!(collection.learn(42));
        assert!(collection.has(42));
        assert_eq!(collection.count(), 1);
    }

    #[test]
    fn learn_duplicate_returns_false() {
        let mut collection = AppearanceCollection::default();
        assert!(collection.learn(42));
        assert!(!collection.learn(42));
        assert_eq!(collection.count(), 1);
    }

    #[test]
    fn learn_many_returns_new_count() {
        let mut collection = AppearanceCollection::default();
        collection.learn(10);
        let added = collection.learn_many([10, 20, 30]);
        assert_eq!(added, 2);
        assert_eq!(collection.count(), 3);
    }

    #[test]
    fn remove_existing() {
        let mut collection = AppearanceCollection::default();
        collection.learn(42);
        assert!(collection.remove(42));
        assert!(!collection.has(42));
        assert_eq!(collection.count(), 0);
    }

    #[test]
    fn remove_nonexistent() {
        let mut collection = AppearanceCollection::default();
        assert!(!collection.remove(99));
    }

    #[test]
    fn all_returns_sorted() {
        let mut collection = AppearanceCollection::default();
        collection.learn(30);
        collection.learn(10);
        collection.learn(20);
        let ids: Vec<u32> = collection.all().collect();
        assert_eq!(ids, vec![10, 20, 30]);
    }

    #[test]
    fn serialization_round_trip() {
        let mut collection = AppearanceCollection::default();
        collection.learn(100);
        collection.learn(200);
        collection.learn(300);

        let json = serde_json::to_string(&collection).unwrap();
        let restored: AppearanceCollection = serde_json::from_str(&json).unwrap();
        assert_eq!(collection, restored);
    }
}
