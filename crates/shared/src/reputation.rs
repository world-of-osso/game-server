//! Reputation / faction standing system.
//!
//! Each faction has a base standing per player race. Standing increases
//! through quest turn-ins, mob kills, and repeatable activities.
//! Ref: AzerothCore `ReputationMgr.cpp`, `FactionEntry`.

use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

/// Standing tiers with their numeric thresholds.
///
/// Each tier spans a range of reputation points. The total range
/// from Hated to Exalted is 84,000 points (retail WoW values).
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
)]
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

/// A spillover rule: gaining rep with one faction affects another.
#[derive(
    Debug, Clone, Copy, PartialEq, Serialize, Deserialize, bitcode::Encode, bitcode::Decode,
)]
pub struct SpilloverRule {
    /// Target faction that receives the spillover.
    pub faction_id: u32,
    /// Fraction of the original gain applied to this faction.
    /// Positive = allied (e.g. 0.25 = 25% of the gain).
    /// Negative = enemy (e.g. -0.5 = lose 50% of the gain as rep).
    pub rate: f32,
}

/// Static definition of a faction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, bitcode::Encode, bitcode::Decode)]
pub struct FactionData {
    /// Unique faction ID.
    pub id: u32,
    /// Faction name (e.g. "Stormwind", "Orgrimmar").
    pub name: String,
    /// Base reputation value per player race (indexed by race ID).
    /// Races not in the map start at neutral (12,000).
    pub base_standing: Vec<(u8, i32)>,
    /// Spillover rules: rep changes with this faction cascade to others.
    pub spillover: Vec<SpilloverRule>,
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
        Self {
            factions: Vec::new(),
        }
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

/// Display info for a faction in the reputation panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactionDisplayEntry {
    pub faction_id: u32,
    pub standing: Standing,
    pub value: i32,
    /// Progress within the current tier: (current, max).
    pub progress: (i32, i32),
}

/// Per-character reputation tracker across all factions.
#[derive(
    Component,
    Debug,
    Clone,
    PartialEq,
    Default,
    Serialize,
    Deserialize,
    bitcode::Encode,
    bitcode::Decode,
)]
pub struct CharacterReputation {
    /// faction_id → current reputation value.
    values: HashMap<u32, i32>,
    /// Factions shown in the reputation panel (player-selected).
    tracked: Vec<u32>,
    /// The single faction displayed on the XP/rep bar (if any).
    watched: Option<u32>,
}

/// Minimal built-in faction registry used by the gameplay server today.
pub fn default_faction_registry() -> FactionRegistry {
    let mut registry = FactionRegistry::new();
    registry.add(FactionData {
        id: 72,
        name: "Stormwind".to_string(),
        base_standing: vec![(1, 21_000), (2, 0)],
        spillover: vec![
            SpilloverRule {
                faction_id: 47,
                rate: 0.25,
            },
            SpilloverRule {
                faction_id: 76,
                rate: -0.5,
            },
        ],
    });
    registry.add(FactionData {
        id: 76,
        name: "Orgrimmar".to_string(),
        base_standing: vec![(2, 21_000), (1, 0)],
        spillover: Vec::new(),
    });
    registry.add(FactionData {
        id: 47,
        name: "Ironforge".to_string(),
        base_standing: vec![(1, 21_000), (2, 0)],
        spillover: Vec::new(),
    });
    registry
}

impl CharacterReputation {
    /// Initialize reputation for a character based on their race.
    pub fn new_for_race(registry: &FactionRegistry, race: u8) -> Self {
        let mut values = HashMap::new();
        for faction in registry.iter() {
            values.insert(faction.id, faction.base_rep_for_race(race));
        }
        Self {
            values,
            tracked: Vec::new(),
            watched: None,
        }
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
    pub fn gain_rep(
        &mut self,
        faction_id: u32,
        amount: i32,
        source: RepGainSource,
    ) -> RepChangeResult {
        self.modify_rep(faction_id, amount.abs(), source)
    }

    /// Lose reputation with a faction (negative amount).
    pub fn lose_rep(
        &mut self,
        faction_id: u32,
        amount: i32,
        source: RepGainSource,
    ) -> RepChangeResult {
        self.modify_rep(faction_id, -amount.abs(), source)
    }

    /// Set reputation to an exact value (admin/debug).
    pub fn set_rep(&mut self, faction_id: u32, value: i32) {
        self.values
            .insert(faction_id, value.clamp(REP_MIN, REP_MAX));
    }

    /// Number of tracked factions.
    pub fn faction_count(&self) -> usize {
        self.values.len()
    }

    /// All faction IDs this character has any reputation value for.
    pub fn all_factions(&self) -> Vec<u32> {
        self.values.keys().copied().collect()
    }

    /// Whether the character has reached a standing with a faction.
    pub fn has_standing(&self, faction_id: u32, required: Standing) -> bool {
        self.get_standing(faction_id) >= required
    }

    // --- Tracked / watched factions (UI display) ---

    /// Add a faction to the tracked list (shown in rep panel).
    pub fn track(&mut self, faction_id: u32) {
        self.push_tracked_faction(faction_id);
    }

    /// Remove a faction from the tracked list.
    pub fn untrack(&mut self, faction_id: u32) {
        self.tracked.retain(|&id| id != faction_id);
        if self.watched == Some(faction_id) {
            self.watched = None;
        }
    }

    /// Whether a faction is in the tracked list.
    pub fn is_tracked(&self, faction_id: u32) -> bool {
        self.tracked.contains(&faction_id)
    }

    /// The tracked faction list.
    pub fn tracked_factions(&self) -> &[u32] {
        &self.tracked
    }

    /// Set the watched faction (shown on the rep/XP bar).
    /// Automatically tracks it if not already tracked.
    pub fn watch(&mut self, faction_id: u32) {
        self.track(faction_id);
        self.watched = Some(faction_id);
    }

    /// Clear the watched faction.
    pub fn unwatch(&mut self) {
        self.watched = None;
    }

    /// The currently watched faction, if any.
    pub fn watched_faction(&self) -> Option<u32> {
        self.watched
    }

    /// Build display entries for all tracked factions.
    pub fn display_tracked(&self) -> Vec<FactionDisplayEntry> {
        self.tracked
            .iter()
            .map(|&id| self.display_entry(id))
            .collect()
    }

    /// Build a display entry for a single faction.
    pub fn display_entry(&self, faction_id: u32) -> FactionDisplayEntry {
        let value = self.get_value(faction_id);
        FactionDisplayEntry {
            faction_id,
            standing: standing_for_value(value),
            value,
            progress: tier_progress(value),
        }
    }

    fn push_tracked_faction(&mut self, faction_id: u32) {
        if self.tracked.contains(&faction_id) {
            return;
        }
        // bounded: ~20 tracked factions, preserve UI order
        self.tracked.push(faction_id);
    }

    /// Gain rep with spillover to allied/enemy factions.
    ///
    /// Applies the primary gain, then cascades to each spillover target
    /// at the configured rate. Returns all changes (primary first).
    pub fn gain_rep_with_spillover(
        &mut self,
        faction_id: u32,
        amount: i32,
        source: RepGainSource,
        registry: &FactionRegistry,
    ) -> Vec<RepChangeResult> {
        let primary = self.gain_rep(faction_id, amount, source);
        let spillovers = collect_spillover(registry, faction_id, amount);
        let mut results = vec![primary];
        for (target_id, spill_amount) in spillovers {
            results.push(self.modify_rep(target_id, spill_amount, RepGainSource::Spillover));
        }
        results
    }
}

/// Compute spillover amounts for a faction gain.
fn collect_spillover(registry: &FactionRegistry, faction_id: u32, amount: i32) -> Vec<(u32, i32)> {
    let Some(faction) = registry.get(faction_id) else {
        return Vec::new();
    };
    faction
        .spillover
        .iter()
        .map(|rule| {
            let spill_amount = (amount as f32 * rule.rate) as i32;
            (rule.faction_id, spill_amount)
        })
        .collect()
}

// --- Mob kill rep diminishing returns ---

/// Grey-level threshold: mobs this many levels below the player give no rep.
const GREY_LEVEL_DIFF: u8 = 8;

/// Apply diminishing returns to mob kill rep based on level difference.
///
/// - Same level or higher: full rep.
/// - 1–7 levels below: linear reduction (87.5% at -1, down to 12.5% at -7).
/// - 8+ levels below (grey): zero rep.
///
/// `base_rep`: the unscaled rep amount from the mob.
/// `player_level`: the player's current level.
/// `mob_level`: the mob's level.
pub fn diminished_mob_rep(base_rep: i32, player_level: u8, mob_level: u8) -> i32 {
    if mob_level >= player_level {
        return base_rep;
    }
    let diff = player_level - mob_level;
    if diff >= GREY_LEVEL_DIFF {
        return 0;
    }
    let scale = (GREY_LEVEL_DIFF - diff) as f32 / GREY_LEVEL_DIFF as f32;
    (base_rep as f32 * scale) as i32
}

// --- Gated rewards ---

/// Type of reward gated behind a reputation standing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GatedRewardType {
    /// A vendor item unlocked at this standing.
    VendorItem,
    /// A crafting recipe unlocked at this standing.
    Recipe,
    /// An enchantment or enchant formula.
    Enchant,
    /// A tabard or cosmetic.
    Tabard,
    /// A mount.
    Mount,
}

/// A reward gated behind a faction standing requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatedReward {
    /// Item or recipe ID.
    pub item_id: u32,
    /// Faction that gates this reward.
    pub faction_id: u32,
    /// Minimum standing required to purchase/learn.
    pub required_standing: Standing,
    /// What kind of reward this is.
    pub reward_type: GatedRewardType,
    /// Cost in copper (0 if free or learned automatically).
    pub cost: u32,
}

/// Check if a character can access a gated reward.
pub fn can_access_reward(reward: &GatedReward, rep: &CharacterReputation) -> bool {
    rep.has_standing(reward.faction_id, reward.required_standing)
}

/// Filter a list of rewards to those accessible at current standing.
pub fn available_rewards<'a>(
    rewards: &'a [GatedReward],
    faction_id: u32,
    rep: &CharacterReputation,
) -> Vec<&'a GatedReward> {
    rewards
        .iter()
        .filter(|r| r.faction_id == faction_id && can_access_reward(r, rep))
        .collect()
}

/// Find rewards that will unlock at the next standing tier.
pub fn upcoming_rewards<'a>(
    rewards: &'a [GatedReward],
    faction_id: u32,
    rep: &CharacterReputation,
) -> Vec<&'a GatedReward> {
    let current = rep.get_standing(faction_id);
    rewards
        .iter()
        .filter(|r| r.faction_id == faction_id && r.required_standing > current)
        .collect()
}

// --- Tabard championing ---

/// A faction tabard that can be worn to champion a faction in dungeons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactionTabard {
    /// Item ID of the tabard.
    pub item_id: u32,
    /// Faction this tabard champions.
    pub faction_id: u32,
}

/// Resolve which faction receives dungeon mob kill rep.
///
/// If the player is wearing a champion tabard, rep is redirected to the
/// tabard's faction. Otherwise, returns the original faction (if any).
///
/// `equipped_tabard` should be the faction tabard the player is wearing,
/// or `None` if they have no faction tabard equipped.
/// `in_dungeon` must be true for championing to apply.
pub fn resolve_champion_faction(
    original_faction: Option<u32>,
    equipped_tabard: Option<&FactionTabard>,
    in_dungeon: bool,
) -> Option<u32> {
    if in_dungeon && let Some(tabard) = equipped_tabard {
        return Some(tabard.faction_id);
    }
    original_faction
}

/// Apply a dungeon mob kill rep gain, respecting tabard championing.
///
/// If championing, the gain goes to the tabard's faction instead of the
/// mob's native faction. Spillover still applies to the receiving faction.
pub fn gain_dungeon_rep(
    rep: &mut CharacterReputation,
    mob_faction: Option<u32>,
    amount: i32,
    equipped_tabard: Option<&FactionTabard>,
    in_dungeon: bool,
    registry: &FactionRegistry,
) -> Vec<RepChangeResult> {
    let target = resolve_champion_faction(mob_faction, equipped_tabard, in_dungeon);
    let Some(faction_id) = target else {
        return Vec::new();
    };
    rep.gain_rep_with_spillover(faction_id, amount, RepGainSource::MobKill, registry)
}

#[cfg(test)]
#[path = "reputation_tests.rs"]
mod tests;
