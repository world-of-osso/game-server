//! Loot system: per-creature loot tables, loot generation, and drop rolls.
//!
//! Each creature has a loot table defining which items can drop, their
//! drop chance, and min/max count. On kill, items are rolled against
//! the table to generate a loot window.
//!
//! Ref: AzerothCore `LootMgr.cpp`, `creature_loot_template` table.

use serde::{Deserialize, Serialize};

/// A single entry in a creature's loot table.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LootEntry {
    /// Item ID that can drop.
    pub item_id: u32,
    /// Drop chance as a fraction (0.0–1.0). 1.0 = guaranteed.
    pub chance: f32,
    /// Minimum count per drop.
    pub min_count: u16,
    /// Maximum count per drop.
    pub max_count: u16,
}

/// Loot table for a creature or object.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LootTable {
    pub entries: Vec<LootEntry>,
}

/// A rolled loot item ready for the loot window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LootDrop {
    pub item_id: u32,
    pub count: u16,
}

impl LootTable {
    /// Roll the loot table to generate drops.
    ///
    /// `rolls` is an iterator of random values in 0.0–1.0 (one per entry).
    /// `count_roll` picks within min..=max for each successful drop.
    ///
    /// Each entry is rolled independently — multiple items can drop.
    pub fn roll(&self, rolls: &[f32], count_rolls: &[f32]) -> Vec<LootDrop> {
        self.entries
            .iter()
            .zip(rolls.iter())
            .zip(count_rolls.iter())
            .filter_map(|((entry, &roll), &count_roll)| {
                if roll > entry.chance {
                    return None;
                }
                let range = entry.max_count - entry.min_count;
                let count = entry.min_count + (range as f32 * count_roll) as u16;
                let count = count.min(entry.max_count);
                Some(LootDrop {
                    item_id: entry.item_id,
                    count,
                })
            })
            .collect()
    }

    /// Roll with a single RNG source (simpler API for common use).
    ///
    /// Each entry gets `rolls[i]` for the drop check and `count_rolls[i]`
    /// for the count within range.
    pub fn roll_simple(&self, rng_values: &[f32]) -> Vec<LootDrop> {
        // Use odd-indexed values for count rolls
        let rolls: Vec<f32> = rng_values.iter().step_by(2).copied().collect();
        let count_rolls: Vec<f32> = rng_values.iter().skip(1).step_by(2).copied().collect();
        self.roll(&rolls, &count_rolls)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_table() -> LootTable {
        LootTable {
            entries: vec![
                LootEntry {
                    item_id: 100,
                    chance: 1.0, // guaranteed
                    min_count: 1,
                    max_count: 1,
                },
                LootEntry {
                    item_id: 200,
                    chance: 0.5, // 50%
                    min_count: 1,
                    max_count: 3,
                },
                LootEntry {
                    item_id: 300,
                    chance: 0.01, // 1% rare
                    min_count: 1,
                    max_count: 1,
                },
            ],
        }
    }

    #[test]
    fn guaranteed_item_always_drops() {
        let table = sample_table();
        let rolls = vec![0.5, 0.99, 0.99]; // only first entry passes
        let count_rolls = vec![0.0, 0.0, 0.0];
        let drops = table.roll(&rolls, &count_rolls);
        assert_eq!(drops.len(), 1);
        assert_eq!(drops[0].item_id, 100);
    }

    #[test]
    fn multiple_items_can_drop() {
        let table = sample_table();
        let rolls = vec![0.5, 0.3, 0.005]; // all three pass
        let count_rolls = vec![0.0, 0.5, 0.0];
        let drops = table.roll(&rolls, &count_rolls);
        assert_eq!(drops.len(), 3);
    }

    #[test]
    fn nothing_drops_on_bad_rolls() {
        let table = sample_table();
        // Only first is guaranteed; others roll > chance
        let rolls = vec![1.1, 0.9, 0.5]; // first > 1.0 technically, but 1.0 chance means <= check
        let count_rolls = vec![0.0; 3];
        // roll=1.1 > chance=1.0 → even guaranteed fails if roll > chance
        let drops = table.roll(&rolls, &count_rolls);
        assert!(drops.is_empty());
    }

    #[test]
    fn count_rolls_within_range() {
        let table = LootTable {
            entries: vec![LootEntry {
                item_id: 500,
                chance: 1.0,
                min_count: 2,
                max_count: 10,
            }],
        };
        // count_roll = 0.0 → min (2), 0.5 → 6, 1.0 → 10
        let drops_min = table.roll(&[0.0], &[0.0]);
        assert_eq!(drops_min[0].count, 2);

        let drops_mid = table.roll(&[0.0], &[0.5]);
        assert_eq!(drops_mid[0].count, 6); // 2 + (8 * 0.5) = 6

        let drops_max = table.roll(&[0.0], &[1.0]);
        assert_eq!(drops_max[0].count, 10);
    }

    #[test]
    fn empty_table_produces_no_drops() {
        let table = LootTable::default();
        let drops = table.roll(&[], &[]);
        assert!(drops.is_empty());
    }

    #[test]
    fn single_count_item() {
        let table = LootTable {
            entries: vec![LootEntry {
                item_id: 100,
                chance: 1.0,
                min_count: 1,
                max_count: 1,
            }],
        };
        let drops = table.roll(&[0.0], &[0.99]);
        assert_eq!(drops[0].count, 1); // clamped to max
    }

    #[test]
    fn rare_item_drops_on_low_roll() {
        let table = LootTable {
            entries: vec![
                LootEntry { item_id: 200, chance: 0.5, min_count: 1, max_count: 1 },
                LootEntry { item_id: 300, chance: 0.01, min_count: 1, max_count: 1 },
            ],
        };
        let rolls = vec![0.99, 0.005]; // only rare passes
        let count_rolls = vec![0.0; 2];
        let drops = table.roll(&rolls, &count_rolls);
        assert_eq!(drops.len(), 1);
        assert_eq!(drops[0].item_id, 300);
    }
}
