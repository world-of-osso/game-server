//! Reputation / faction standing system.
//!
//! Each faction has a base standing per player race. Standing increases
//! through quest turn-ins, mob kills, and repeatable activities.
//! Ref: AzerothCore `ReputationMgr.cpp`, `FactionEntry`.

use serde::{Deserialize, Serialize};

/// Standing tiers with their numeric thresholds.
///
/// Each tier spans a range of reputation points. The total range
/// from Hated to Exalted is 84,000 points (retail WoW values).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Standing {
    Hated,
    Hostile,
    Unfriendly,
    Neutral,
    Friendly,
    Honored,
    Revered,
    Exalted,
}

/// Reputation thresholds for each standing tier (cumulative from Hated = 0).
///
/// Hated:      0 –  5,999
/// Hostile:    6,000 – 8,999
/// Unfriendly: 9,000 – 11,999
/// Neutral:    12,000 – 20,999
/// Friendly:   21,000 – 29,999
/// Honored:    30,000 – 41,999
/// Revered:    42,000 – 62,999
/// Exalted:    63,000 – 83,999
const TIER_THRESHOLDS: [(Standing, i32); 8] = [
    (Standing::Hated, 0),
    (Standing::Hostile, 6_000),
    (Standing::Unfriendly, 9_000),
    (Standing::Neutral, 12_000),
    (Standing::Friendly, 21_000),
    (Standing::Honored, 30_000),
    (Standing::Revered, 42_000),
    (Standing::Exalted, 63_000),
];

/// Maximum reputation value.
pub const REP_MAX: i32 = 83_999;
/// Minimum reputation value.
pub const REP_MIN: i32 = 0;
/// Default neutral standing value.
pub const REP_NEUTRAL: i32 = 12_000;

/// Determine the standing tier for a raw reputation value.
pub fn standing_for_value(rep: i32) -> Standing {
    let clamped = rep.clamp(REP_MIN, REP_MAX);
    let mut result = Standing::Hated;
    for &(tier, threshold) in &TIER_THRESHOLDS {
        if clamped >= threshold {
            result = tier;
        }
    }
    result
}

/// Get the threshold (minimum rep value) for a standing tier.
pub fn threshold_for_standing(standing: Standing) -> i32 {
    TIER_THRESHOLDS
        .iter()
        .find(|(s, _)| *s == standing)
        .map(|(_, t)| *t)
        .unwrap_or(0)
}

/// Progress within the current tier: (current_into_tier, tier_size).
pub fn tier_progress(rep: i32) -> (i32, i32) {
    let clamped = rep.clamp(REP_MIN, REP_MAX);
    let mut tier_start = 0;
    let mut tier_end = REP_MAX + 1;
    for i in 0..TIER_THRESHOLDS.len() {
        if clamped >= TIER_THRESHOLDS[i].1 {
            tier_start = TIER_THRESHOLDS[i].1;
            tier_end = if i + 1 < TIER_THRESHOLDS.len() {
                TIER_THRESHOLDS[i + 1].1
            } else {
                REP_MAX + 1
            };
        }
    }
    (clamped - tier_start, tier_end - tier_start)
}

/// Static definition of a faction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactionData {
    /// Unique faction ID.
    pub id: u32,
    /// Faction name (e.g. "Stormwind", "Orgrimmar").
    pub name: String,
    /// Base reputation value per player race (indexed by race ID).
    /// Races not in the map start at neutral (12,000).
    pub base_standing: Vec<(u8, i32)>,
}

impl FactionData {
    /// Get the base reputation value for a race.
    /// Returns neutral (12,000) if the race has no specific entry.
    pub fn base_rep_for_race(&self, race: u8) -> i32 {
        self.base_standing
            .iter()
            .find(|(r, _)| *r == race)
            .map(|(_, rep)| *rep)
            .unwrap_or(REP_NEUTRAL)
    }

    /// Get the starting standing tier for a race.
    pub fn starting_standing(&self, race: u8) -> Standing {
        standing_for_value(self.base_rep_for_race(race))
    }
}

/// Registry of all faction definitions.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FactionRegistry {
    factions: Vec<FactionData>,
}

impl FactionRegistry {
    pub fn new() -> Self {
        Self { factions: Vec::new() }
    }

    /// Register a faction.
    pub fn add(&mut self, faction: FactionData) {
        self.factions.retain(|f| f.id != faction.id);
        self.factions.push(faction);
    }

    /// Look up a faction by ID.
    pub fn get(&self, id: u32) -> Option<&FactionData> {
        self.factions.iter().find(|f| f.id == id)
    }

    /// Look up a faction by name.
    pub fn find_by_name(&self, name: &str) -> Option<&FactionData> {
        self.factions.iter().find(|f| f.name == name)
    }

    /// Number of registered factions.
    pub fn len(&self) -> usize {
        self.factions.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.factions.is_empty()
    }

    /// Iterate over all factions.
    pub fn iter(&self) -> impl Iterator<Item = &FactionData> {
        self.factions.iter()
    }
}

// --- Rep gains (per-character tracking) ---

use std::collections::HashMap;

/// Source of a reputation change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepGainSource {
    /// One-time quest completion reward.
    QuestTurnIn,
    /// Killing a mob associated with the faction.
    MobKill,
    /// Repeatable turn-in (e.g. cloth quartermaster, tokens).
    RepeatableTurnIn,
    /// Spillover from a related faction.
    Spillover,
    /// Direct GM/admin adjustment.
    Admin,
}

/// Result of applying a reputation change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepChangeResult {
    /// Faction that changed.
    pub faction_id: u32,
    /// Rep value before the change.
    pub old_value: i32,
    /// Rep value after the change.
    pub new_value: i32,
    /// Standing before the change.
    pub old_standing: Standing,
    /// Standing after the change.
    pub new_standing: Standing,
}

impl RepChangeResult {
    /// Whether the standing tier changed.
    pub fn tier_changed(&self) -> bool {
        self.old_standing != self.new_standing
    }

    /// The actual amount gained (may differ from requested due to clamping).
    pub fn actual_change(&self) -> i32 {
        self.new_value - self.old_value
    }
}

/// Per-character reputation tracker across all factions.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CharacterReputation {
    /// faction_id → current reputation value.
    values: HashMap<u32, i32>,
}

impl CharacterReputation {
    /// Initialize reputation for a character based on their race.
    pub fn new_for_race(registry: &FactionRegistry, race: u8) -> Self {
        let mut values = HashMap::new();
        for faction in registry.iter() {
            values.insert(faction.id, faction.base_rep_for_race(race));
        }
        Self { values }
    }

    /// Get the raw reputation value for a faction.
    /// Returns neutral if the faction hasn't been encountered.
    pub fn get_value(&self, faction_id: u32) -> i32 {
        self.values.get(&faction_id).copied().unwrap_or(REP_NEUTRAL)
    }

    /// Get the standing tier for a faction.
    pub fn get_standing(&self, faction_id: u32) -> Standing {
        standing_for_value(self.get_value(faction_id))
    }

    /// Apply a reputation gain or loss. Returns the change result.
    pub fn modify_rep(
        &mut self,
        faction_id: u32,
        amount: i32,
        _source: RepGainSource,
    ) -> RepChangeResult {
        let old_value = self.get_value(faction_id);
        let new_value = (old_value + amount).clamp(REP_MIN, REP_MAX);
        self.values.insert(faction_id, new_value);
        RepChangeResult {
            faction_id,
            old_value,
            new_value,
            old_standing: standing_for_value(old_value),
            new_standing: standing_for_value(new_value),
        }
    }

    /// Gain reputation with a faction (positive amount).
    pub fn gain_rep(&mut self, faction_id: u32, amount: i32, source: RepGainSource) -> RepChangeResult {
        self.modify_rep(faction_id, amount.abs(), source)
    }

    /// Lose reputation with a faction (negative amount).
    pub fn lose_rep(&mut self, faction_id: u32, amount: i32, source: RepGainSource) -> RepChangeResult {
        self.modify_rep(faction_id, -amount.abs(), source)
    }

    /// Set reputation to an exact value (admin/debug).
    pub fn set_rep(&mut self, faction_id: u32, value: i32) {
        self.values.insert(faction_id, value.clamp(REP_MIN, REP_MAX));
    }

    /// Number of tracked factions.
    pub fn faction_count(&self) -> usize {
        self.values.len()
    }

    /// All faction IDs this character has reputation with.
    pub fn tracked_factions(&self) -> Vec<u32> {
        self.values.keys().copied().collect()
    }

    /// Whether the character has reached a standing with a faction.
    pub fn has_standing(&self, faction_id: u32, required: Standing) -> bool {
        self.get_standing(faction_id) >= required
    }
}

#[cfg(test)]
#[path = "reputation_tests.rs"]
mod tests;
