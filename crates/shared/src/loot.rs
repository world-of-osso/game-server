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

// --- Loot window ---

/// State of a single item in a loot window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootSlotState {
    /// Available to be looted.
    Available,
    /// Already looted by someone.
    Looted,
}

/// A slot in the loot window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LootSlot {
    pub item_id: u32,
    pub count: u16,
    pub state: LootSlotState,
}

/// A loot window generated from a mob death.
///
/// Created when a creature dies, populated by rolling its loot table.
/// Players interact with the window to pick up items. Gold is tracked
/// separately from item drops.
#[derive(Debug, Clone, PartialEq)]
pub struct LootWindow {
    /// Entity bits of the creature corpse.
    pub source: u64,
    /// Item slots available for looting.
    pub slots: Vec<LootSlot>,
    /// Gold drop in copper.
    pub gold: u32,
    /// Whether gold has been looted.
    pub gold_looted: bool,
}

impl LootWindow {
    /// Generate a loot window from a mob death.
    ///
    /// Rolls the loot table with the provided random values and adds
    /// a gold drop based on creature level.
    pub fn generate(
        source: u64,
        table: &LootTable,
        rolls: &[f32],
        count_rolls: &[f32],
        gold: u32,
    ) -> Self {
        let drops = table.roll(rolls, count_rolls);
        let slots = drops
            .into_iter()
            .map(|d| LootSlot {
                item_id: d.item_id,
                count: d.count,
                state: LootSlotState::Available,
            })
            .collect();
        Self {
            source,
            slots,
            gold,
            gold_looted: false,
        }
    }

    /// Loot an item from a slot by index. Returns the item if available.
    pub fn loot_item(&mut self, index: usize) -> Option<LootDrop> {
        let slot = self.slots.get_mut(index)?;
        if slot.state != LootSlotState::Available {
            return None;
        }
        slot.state = LootSlotState::Looted;
        Some(LootDrop {
            item_id: slot.item_id,
            count: slot.count,
        })
    }

    /// Loot the gold. Returns the amount if not already looted.
    pub fn loot_gold(&mut self) -> Option<u32> {
        if self.gold_looted || self.gold == 0 {
            return None;
        }
        self.gold_looted = true;
        Some(self.gold)
    }

    /// Whether all items and gold have been looted (corpse can despawn).
    pub fn is_fully_looted(&self) -> bool {
        let items_done = self.slots.iter().all(|s| s.state == LootSlotState::Looted);
        let gold_done = self.gold == 0 || self.gold_looted;
        items_done && gold_done
    }

    /// Number of items still available to loot.
    pub fn available_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|s| s.state == LootSlotState::Available)
            .count()
    }
}

/// Estimate gold drop from creature level (copper).
///
/// Rough approximation: `level * level * 2` copper.
/// Real values come from AzerothCore `creature_template.mingold/maxgold`.
pub fn estimated_gold_drop(creature_level: u8, roll: f32) -> u32 {
    let base = creature_level as u32 * creature_level as u32 * 2;
    let variance = (base as f32 * 0.3 * roll) as u32;
    base + variance
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

    // --- Loot window tests ---

    #[test]
    fn generate_loot_window() {
        let table = sample_table();
        let rolls = vec![0.5, 0.3, 0.005]; // all three pass
        let count_rolls = vec![0.0, 0.0, 0.0];
        let window = LootWindow::generate(42, &table, &rolls, &count_rolls, 500);
        assert_eq!(window.source, 42);
        assert_eq!(window.slots.len(), 3);
        assert_eq!(window.gold, 500);
        assert!(!window.gold_looted);
        assert!(!window.is_fully_looted());
    }

    #[test]
    fn loot_item_from_window() {
        let table = sample_table();
        let rolls = vec![0.0; 3];
        let count_rolls = vec![0.0; 3];
        let mut window = LootWindow::generate(1, &table, &rolls, &count_rolls, 0);

        let item = window.loot_item(0).unwrap();
        assert_eq!(item.item_id, 100);
        assert_eq!(window.available_count(), 2);

        // Can't loot same slot twice
        assert!(window.loot_item(0).is_none());
    }

    #[test]
    fn loot_gold_from_window() {
        let mut window = LootWindow::generate(1, &LootTable::default(), &[], &[], 1000);
        let gold = window.loot_gold().unwrap();
        assert_eq!(gold, 1000);
        assert!(window.loot_gold().is_none()); // already looted
    }

    #[test]
    fn fully_looted_check() {
        let table = LootTable {
            entries: vec![LootEntry {
                item_id: 100,
                chance: 1.0,
                min_count: 1,
                max_count: 1,
            }],
        };
        let mut window = LootWindow::generate(1, &table, &[0.0], &[0.0], 50);
        assert!(!window.is_fully_looted());

        window.loot_item(0);
        assert!(!window.is_fully_looted()); // gold still

        window.loot_gold();
        assert!(window.is_fully_looted());
    }

    #[test]
    fn empty_loot_window_is_fully_looted() {
        let window = LootWindow::generate(1, &LootTable::default(), &[], &[], 0);
        assert!(window.is_fully_looted());
    }

    #[test]
    fn loot_item_out_of_bounds() {
        let mut window = LootWindow::generate(1, &LootTable::default(), &[], &[], 0);
        assert!(window.loot_item(99).is_none());
    }

    #[test]
    fn estimated_gold_scales_with_level() {
        let low = estimated_gold_drop(10, 0.0);
        let high = estimated_gold_drop(80, 0.0);
        assert!(high > low);
        assert_eq!(low, 200); // 10 * 10 * 2
        assert_eq!(high, 12800); // 80 * 80 * 2
    }
}
