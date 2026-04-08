//! Loot system: per-creature loot tables, loot generation, and drop rolls.
//!
//! Each creature has a loot table defining which items can drop, their
//! drop chance, and min/max count. On kill, items are rolled against
//! the table to generate a loot window.
//!
//! Ref: AzerothCore `LootMgr.cpp`, `creature_loot_template` table.

use serde::{Deserialize, Serialize};

/// Loot entry type — determines drop behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LootEntryType {
    /// Normal drop: uses the entry's `chance` field.
    Normal,
    /// Rare/special drop: uses `chance` but flagged for UI highlight.
    Rare,
    /// Quest item: drops at 100% for eligible players, hidden for others.
    Quest,
}

/// A single entry in a creature's loot table.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LootEntry {
    /// Item ID that can drop.
    pub item_id: u32,
    /// Drop chance as a fraction (0.0–1.0). 1.0 = guaranteed.
    /// Ignored for Quest entries (always 100% for eligible players).
    pub chance: f32,
    /// Minimum count per drop.
    pub min_count: u16,
    /// Maximum count per drop.
    pub max_count: u16,
    /// Entry type: Normal, Rare, or Quest.
    pub entry_type: LootEntryType,
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

/// Resolve whether a loot entry drops, given its type and a roll.
fn entry_drops(entry: &LootEntry, roll: f32, eligible_quest_ids: &[u32]) -> bool {
    match entry.entry_type {
        LootEntryType::Quest => eligible_quest_ids.contains(&entry.item_id),
        LootEntryType::Normal | LootEntryType::Rare => roll <= entry.chance,
    }
}

/// Compute the drop count from an entry's min/max range.
fn roll_count(entry: &LootEntry, count_roll: f32) -> u16 {
    let range = entry.max_count - entry.min_count;
    let count = entry.min_count + (range as f32 * count_roll) as u16;
    count.min(entry.max_count)
}

impl LootTable {
    /// Roll the loot table to generate drops.
    ///
    /// - Normal/Rare entries: `rolls[i] <= chance` to drop.
    /// - Quest entries: drop at 100% if `item_id` is in `eligible_quest_ids`.
    ///
    /// `rolls` and `count_rolls` are pre-generated random values in 0.0–1.0.
    pub fn roll(
        &self,
        rolls: &[f32],
        count_rolls: &[f32],
        eligible_quest_ids: &[u32],
    ) -> Vec<LootDrop> {
        self.entries
            .iter()
            .zip(rolls.iter())
            .zip(count_rolls.iter())
            .filter_map(|((entry, &roll), &count_roll)| {
                if !entry_drops(entry, roll, eligible_quest_ids) {
                    return None;
                }
                Some(LootDrop {
                    item_id: entry.item_id,
                    count: roll_count(entry, count_roll),
                })
            })
            .collect()
    }
}

// --- Group loot ---

/// Group loot distribution mode.
///
/// Ref: AzerothCore `LootMethod` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LootMode {
    /// Free for All — anyone can loot anything.
    FreeForAll,
    /// Round Robin — common items rotate between group members.
    RoundRobin,
    /// Need Before Greed — rare+ items trigger a roll window.
    NeedBeforeGreed,
    /// Personal Loot — each player gets their own independent roll (retail default).
    PersonalLoot,
}

/// A player's roll choice on a need/greed item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootRollChoice {
    Need,
    Greed,
    Pass,
}

/// Result of a need/greed roll for one player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LootRollEntry {
    pub player_id: u64,
    pub choice: LootRollChoice,
    /// Random roll value (1–100) for tiebreaking within Need or Greed.
    pub roll_value: u8,
}

/// Resolve a need/greed roll: highest Need wins, then highest Greed.
/// Returns the winner's player_id, or None if everyone passed.
pub fn resolve_need_greed(entries: &[LootRollEntry]) -> Option<u64> {
    // Need rolls win over Greed
    let best_need = entries
        .iter()
        .filter(|e| e.choice == LootRollChoice::Need)
        .max_by_key(|e| e.roll_value);
    if let Some(winner) = best_need {
        return Some(winner.player_id);
    }
    // Then Greed
    let best_greed = entries
        .iter()
        .filter(|e| e.choice == LootRollChoice::Greed)
        .max_by_key(|e| e.roll_value);
    best_greed.map(|w| w.player_id)
}

/// Assign a loot item based on the group's loot mode.
///
/// Returns the player_id who receives the item.
/// - FFA/PersonalLoot: `looter` (whoever opened the corpse / the designated player).
/// - RoundRobin: `round_robin_next` (rotated by the server).
/// - NeedBeforeGreed: resolved from `roll_entries`.
pub fn assign_loot(
    mode: LootMode,
    looter: u64,
    round_robin_next: u64,
    roll_entries: &[LootRollEntry],
) -> Option<u64> {
    match mode {
        LootMode::FreeForAll | LootMode::PersonalLoot => Some(looter),
        LootMode::RoundRobin => Some(round_robin_next),
        LootMode::NeedBeforeGreed => resolve_need_greed(roll_entries),
    }
}

// --- Need/greed roll window ---

/// Seconds players have to choose Need/Greed/Pass before auto-pass.
const ROLL_WINDOW_TIMEOUT: f32 = 60.0;

/// Tracks a per-item need/greed roll in progress.
///
/// Created when a rare+ item drops in NeedBeforeGreed mode.
/// Each eligible player must respond (Need/Greed/Pass) or be auto-passed
/// after the timeout. Players who can't equip the item are auto-passed immediately.
#[derive(Debug, Clone, PartialEq)]
pub struct RollWindow {
    /// The item being rolled on.
    pub item: LootDrop,
    /// Players who are eligible to roll (not auto-passed).
    pub pending_players: Vec<u64>,
    /// Completed roll responses.
    pub responses: Vec<LootRollEntry>,
    /// Time remaining before auto-pass for non-respondents.
    pub time_remaining: f32,
}

impl RollWindow {
    /// Create a roll window for an item.
    ///
    /// `eligible_players` are those in range. `can_equip` filters who can
    /// Need — players who can't equip are auto-passed immediately.
    pub fn new(item: LootDrop, group_players: &[u64], can_equip: &[u64]) -> Self {
        let (pending_players, responses) = partition_roll_players(group_players, can_equip);
        Self {
            item,
            pending_players,
            responses,
            time_remaining: ROLL_WINDOW_TIMEOUT,
        }
    }

    /// Record a player's choice. Returns `true` if accepted.
    pub fn respond(&mut self, player_id: u64, choice: LootRollChoice, roll_value: u8) -> bool {
        let Some(idx) = self.pending_players.iter().position(|&p| p == player_id) else {
            return false;
        };
        self.pending_players.swap_remove(idx);
        self.responses.push(LootRollEntry {
            player_id,
            choice,
            roll_value,
        });
        true
    }

    /// Tick the timer. Auto-passes remaining players if time expires.
    /// Returns `true` if the roll is complete (all responded or timed out).
    pub fn tick(&mut self, dt: f32) -> bool {
        if self.pending_players.is_empty() {
            return true;
        }
        self.time_remaining -= dt;
        if self.time_remaining <= 0.0 {
            self.auto_pass_remaining();
            return true;
        }
        false
    }

    /// Whether all players have responded.
    pub fn is_complete(&self) -> bool {
        self.pending_players.is_empty()
    }

    /// Resolve the winner. Only valid when complete.
    pub fn resolve(&self) -> Option<u64> {
        resolve_need_greed(&self.responses)
    }

    fn auto_pass_remaining(&mut self) {
        for player_id in self.pending_players.drain(..) {
            self.responses.push(LootRollEntry {
                player_id,
                choice: LootRollChoice::Pass,
                roll_value: 0,
            });
        }
    }
}

fn partition_roll_players(
    group_players: &[u64],
    can_equip: &[u64],
) -> (Vec<u64>, Vec<LootRollEntry>) {
    let (pending_players, auto_passed_players): (Vec<_>, Vec<_>) = group_players
        .iter()
        .copied()
        .partition(|player_id| can_equip.contains(player_id));
    let responses = auto_passed_players
        .into_iter()
        .map(auto_pass_entry)
        .collect();
    (pending_players, responses)
}

fn auto_pass_entry(player_id: u64) -> LootRollEntry {
    LootRollEntry {
        player_id,
        choice: LootRollChoice::Pass,
        roll_value: 0,
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
        eligible_quest_ids: &[u32],
        gold: u32,
    ) -> Self {
        let drops = table.roll(rolls, count_rolls, eligible_quest_ids);
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

// --- Loot sparkle ---

/// Marker indicating a corpse has loot available for a specific player.
///
/// The client uses this to render a sparkle/glow effect on the corpse.
/// Added when a mob dies and has loot; removed when the corpse is fully
/// looted or despawns.
///
/// In personal loot, each player may see different sparkles. In group loot,
/// all eligible players see the sparkle until loot is claimed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LootSparkle {
    /// Entity bits of the corpse.
    pub corpse: u64,
}

/// Determine which corpses should show a loot sparkle for a player.
///
/// A corpse sparkles if its loot window has available items or unclaimed gold.
pub fn sparkle_corpses(windows: &[(u64, &LootWindow)]) -> Vec<LootSparkle> {
    windows
        .iter()
        .filter(|(_, w)| !w.is_fully_looted())
        .map(|(entity, _)| LootSparkle { corpse: *entity })
        .collect()
}

// --- AoE looting ---

/// Default AoE loot radius in yards.
pub const AOE_LOOT_RADIUS: f32 = 50.0;

/// A nearby corpse eligible for AoE looting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LootableCorpse {
    /// Entity bits of the corpse.
    pub entity: u64,
    /// Distance from the player in yards.
    pub distance: f32,
}

/// Result of AoE looting: items and gold collected from multiple corpses.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AoeLootResult {
    /// Items collected from all corpses.
    pub items: Vec<LootDrop>,
    /// Total gold collected in copper.
    pub gold: u32,
    /// Entity bits of corpses that were fully looted (for despawn).
    pub fully_looted: Vec<u64>,
}

/// Perform AoE loot: collect all available items and gold from loot windows
/// of nearby corpses within `radius`.
///
/// Each window is looted completely (all items + gold). Returns the
/// aggregate result. Windows that become fully looted are tracked for
/// corpse despawn.
pub fn aoe_loot(
    windows: &mut [(u64, &mut LootWindow)],
    player_distances: &[(u64, f32)],
    radius: f32,
) -> AoeLootResult {
    let mut result = AoeLootResult::default();

    let in_range: std::collections::HashSet<u64> = player_distances
        .iter()
        .filter(|(_, dist)| *dist <= radius)
        .map(|(entity, _)| *entity)
        .collect();

    for (entity, window) in windows {
        if !in_range.contains(entity) {
            continue;
        }
        // Loot all available items
        for i in 0..window.slots.len() {
            if let Some(drop) = window.loot_item(i) {
                result.items.push(drop);
            }
        }
        // Loot gold
        if let Some(gold) = window.loot_gold() {
            result.gold += gold;
        }
        if window.is_fully_looted() {
            result.fully_looted.push(*entity);
        }
    }

    result
}

/// Creature type for gold drop scaling.
///
/// Ref: AzerothCore `creature_template.rank`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreatureRank {
    /// Normal mob — baseline gold.
    Normal,
    /// Elite mob — 3x gold.
    Elite,
    /// Rare mob — 2x gold.
    Rare,
    /// Rare elite — 5x gold.
    RareElite,
    /// Boss — 10x gold.
    Boss,
}

impl CreatureRank {
    /// Gold multiplier for this creature rank.
    pub fn gold_multiplier(self) -> f32 {
        match self {
            Self::Normal => 1.0,
            Self::Rare => 2.0,
            Self::Elite => 3.0,
            Self::RareElite => 5.0,
            Self::Boss => 10.0,
        }
    }
}

/// Calculate gold drop in copper from creature level and rank.
///
/// Base: `level² × 2` copper for Normal mobs, scaled by rank multiplier.
/// `roll` (0.0–1.0) adds ±30% variance.
///
/// Real values come from AzerothCore `creature_template.mingold/maxgold`;
/// this provides a reasonable default when those aren't available.
pub fn gold_drop(creature_level: u8, rank: CreatureRank, roll: f32) -> u32 {
    let base = creature_level as f32 * creature_level as f32 * 2.0;
    let scaled = base * rank.gold_multiplier();
    let variance = scaled * 0.3 * roll;
    (scaled + variance) as u32
}

#[cfg(test)]
#[path = "loot_tests.rs"]
mod tests;
