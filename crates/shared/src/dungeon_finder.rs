//! Dungeon finder: role-based matchmaking queue.
//!
//! Ref: AzerothCore `LFGMgr.cpp`.

use serde::{Deserialize, Serialize};

use crate::group::GroupRole;

/// A player or group queued for a dungeon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueueEntry {
    /// Player entity bits (leader if group).
    pub leader: u64,
    /// All member entity bits (1 for solo, up to 5 for group).
    pub members: Vec<u64>,
    /// Roles each member can fill. Index matches `members`.
    pub roles: Vec<GroupRole>,
    /// Dungeon IDs the group is queued for.
    pub dungeon_ids: Vec<u32>,
    /// Server timestamp when queued.
    pub queued_at: u64,
}

/// Why queuing failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueError {
    /// Already in the queue.
    AlreadyQueued,
    /// No dungeons selected.
    NoDungeons,
    /// No roles selected.
    NoRoles,
    /// Player has a deserter penalty.
    Deserter,
}

/// The dungeon finder queue.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DungeonQueue {
    pub entries: Vec<QueueEntry>,
}

impl DungeonQueue {
    /// Add a solo player to the queue.
    pub fn queue_solo(
        &mut self,
        player: u64,
        role: GroupRole,
        dungeon_ids: Vec<u32>,
        now: u64,
    ) -> Result<(), QueueError> {
        self.queue_group(player, vec![player], vec![role], dungeon_ids, now)
    }

    /// Add a group to the queue.
    pub fn queue_group(
        &mut self,
        leader: u64,
        members: Vec<u64>,
        roles: Vec<GroupRole>,
        dungeon_ids: Vec<u32>,
        now: u64,
    ) -> Result<(), QueueError> {
        if dungeon_ids.is_empty() {
            return Err(QueueError::NoDungeons);
        }
        if roles.is_empty() {
            return Err(QueueError::NoRoles);
        }
        if self.is_queued(leader) {
            return Err(QueueError::AlreadyQueued);
        }
        self.entries.push(QueueEntry {
            leader,
            members,
            roles,
            dungeon_ids,
            queued_at: now,
        });
        Ok(())
    }

    /// Remove a player/group from the queue.
    pub fn dequeue(&mut self, leader: u64) -> bool {
        let had = self.entries.iter().any(|e| e.leader == leader);
        self.entries.retain(|e| e.leader != leader);
        had
    }

    /// Whether a player is already queued (as leader or member).
    pub fn is_queued(&self, player: u64) -> bool {
        self.entries.iter().any(|e| e.members.contains(&player))
    }

    /// Number of entries in the queue.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Average wait time estimate based on queue entries (seconds since queued).
    pub fn average_wait(&self, now: u64) -> f32 {
        if self.entries.is_empty() {
            return 0.0;
        }
        let total: u64 = self
            .entries
            .iter()
            .map(|e| now.saturating_sub(e.queued_at))
            .sum();
        total as f32 / self.entries.len() as f32
    }
    /// Try to form a 1T+1H+3D group from queued entries with overlapping dungeons.
    ///
    /// Returns the matched entries (removed from queue) and the dungeon ID,
    /// or `None` if no valid group can be formed.
    pub fn try_match(&mut self) -> Option<MatchResult> {
        // Find a common dungeon across candidates
        let all_dungeons: Vec<u32> = self
            .entries
            .iter()
            .flat_map(|e| e.dungeon_ids.iter().copied())
            .collect();

        for dungeon_id in &all_dungeons {
            let candidates: Vec<usize> = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| e.dungeon_ids.contains(dungeon_id))
                .map(|(i, _)| i)
                .collect();

            if let Some(group) = find_composition(&self.entries, &candidates) {
                let result = build_match(&self.entries, &group, *dungeon_id);
                // Remove matched entries (reverse order to preserve indices)
                let mut to_remove: Vec<usize> = group.iter().map(|&(idx, _)| idx).collect();
                to_remove.sort_unstable();
                to_remove.dedup();
                for idx in to_remove.into_iter().rev() {
                    self.entries.remove(idx);
                }
                return Some(result);
            }
        }
        None
    }
}

/// A successful dungeon match.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchResult {
    pub dungeon_id: u32,
    pub tank: u64,
    pub healer: u64,
    pub dps: Vec<u64>,
}

// --- Dungeon definitions and teleport ---

/// A dungeon instance definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DungeonDef {
    pub id: u32,
    pub name: String,
    pub map_id: u16,
    /// Entrance position where players are teleported.
    pub entrance_x: f32,
    pub entrance_y: f32,
    pub entrance_z: f32,
    pub min_level: u8,
    pub max_level: u8,
}

/// A teleport order for a player entering a dungeon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TeleportOrder {
    pub player: u64,
    pub map_id: u16,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Generate teleport orders for all players in a match.
///
/// Returns one `TeleportOrder` per player (tank, healer, 3 DPS).
/// The server applies these to move players to the dungeon entrance.
pub fn teleport_orders(match_result: &MatchResult, dungeon: &DungeonDef) -> Vec<TeleportOrder> {
    let all_players = std::iter::once(match_result.tank)
        .chain(std::iter::once(match_result.healer))
        .chain(match_result.dps.iter().copied());

    all_players
        .map(|player| TeleportOrder {
            player,
            map_id: dungeon.map_id,
            x: dungeon.entrance_x,
            y: dungeon.entrance_y,
            z: dungeon.entrance_z,
        })
        .collect()
}

// --- Instance lockouts ---

/// Instance difficulty / reset period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockoutType {
    /// Heroic dungeon: resets daily.
    HeroicDaily,
    /// Mythic dungeon or raid: resets weekly.
    MythicWeekly,
}

/// Seconds per day.
const DAY_SECS: u64 = 86400;
/// Seconds per week.
const WEEK_SECS: u64 = 604800;

/// A single instance lockout for a player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceLockout {
    pub dungeon_id: u32,
    pub lockout_type: LockoutType,
    /// Timestamp when the lockout was acquired.
    pub acquired_at: u64,
}

impl InstanceLockout {
    /// Timestamp of the next reset for this lockout.
    pub fn reset_at(&self) -> u64 {
        let period = match self.lockout_type {
            LockoutType::HeroicDaily => DAY_SECS,
            LockoutType::MythicWeekly => WEEK_SECS,
        };
        let periods = self.acquired_at / period;
        (periods + 1) * period
    }

    /// Whether this lockout has expired.
    pub fn is_expired(&self, now: u64) -> bool {
        now >= self.reset_at()
    }
}

/// Per-player instance lockout tracker.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PlayerLockouts {
    pub lockouts: Vec<InstanceLockout>,
}

impl PlayerLockouts {
    /// Record completing an instance.
    pub fn add(&mut self, dungeon_id: u32, lockout_type: LockoutType, now: u64) {
        // Remove any existing lockout for this dungeon
        self.lockouts.retain(|l| l.dungeon_id != dungeon_id);
        self.lockouts.push(InstanceLockout {
            dungeon_id,
            lockout_type,
            acquired_at: now,
        });
    }

    /// Whether a player is locked out of a dungeon.
    pub fn is_locked(&self, dungeon_id: u32, now: u64) -> bool {
        self.lockouts
            .iter()
            .any(|l| l.dungeon_id == dungeon_id && !l.is_expired(now))
    }

    /// Remove expired lockouts.
    pub fn cleanup(&mut self, now: u64) {
        self.lockouts.retain(|l| !l.is_expired(now));
    }

    /// Get the reset time for a specific dungeon lockout (if active).
    pub fn reset_time(&self, dungeon_id: u32, now: u64) -> Option<u64> {
        self.lockouts
            .iter()
            .find(|l| l.dungeon_id == dungeon_id && !l.is_expired(now))
            .map(|l| l.reset_at())
    }
}

// --- Satchel rewards ---

/// A satchel bonus for queuing as an in-demand role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SatchelReward {
    /// Gold bonus in copper.
    pub gold: u32,
    /// Bonus item (e.g. Satchel of Exotic Mysteries).
    pub item_id: u32,
}

/// Default satchel reward.
pub const DEFAULT_SATCHEL: SatchelReward = SatchelReward {
    gold: 500_000,  // 50g
    item_id: 54516, // Satchel of Exotic Mysteries
};

/// Determine which roles are in demand based on current queue composition.
///
/// A role is "in demand" if the queue has significantly fewer of that role
/// than needed. Standard ratio: 1T:1H:3D per group → tanks and healers
/// are in demand when their count per DPS is below threshold.
pub fn in_demand_roles(queue: &DungeonQueue) -> Vec<GroupRole> {
    let mut tanks = 0u32;
    let mut healers = 0u32;
    let mut dps = 0u32;

    for entry in &queue.entries {
        for role in &entry.roles {
            match role {
                GroupRole::Tank => tanks += 1,
                GroupRole::Healer => healers += 1,
                GroupRole::Dps => dps += 1,
            }
        }
    }

    let mut demand = Vec::new();
    // Need 1 tank per 3 DPS; in demand if ratio is worse
    if dps > 0 && tanks * 3 < dps {
        demand.push(GroupRole::Tank);
    }
    // Need 1 healer per 3 DPS
    if dps > 0 && healers * 3 < dps {
        demand.push(GroupRole::Healer);
    }
    // If queue is empty, both tank and healer are in demand
    if queue.is_empty() {
        demand.push(GroupRole::Tank);
        demand.push(GroupRole::Healer);
    }
    demand
}

/// Check if a player's role qualifies for a satchel reward.
pub fn satchel_for_role(role: GroupRole, demand: &[GroupRole]) -> Option<SatchelReward> {
    if demand.contains(&role) {
        Some(DEFAULT_SATCHEL)
    } else {
        None
    }
}

// --- Deserter penalty ---

/// Deserter debuff duration in seconds (30 minutes).
pub const DESERTER_DURATION: u64 = 1800;

/// Tracks deserter penalties for players who leave dungeons early.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DeserterTracker {
    /// (player_entity, expires_at_timestamp) pairs.
    entries: Vec<(u64, u64)>,
}

impl DeserterTracker {
    /// Apply a deserter penalty to a player.
    pub fn apply(&mut self, player: u64, now: u64) {
        let expires = now + DESERTER_DURATION;
        if let Some(entry) = self.entries.iter_mut().find(|(p, _)| *p == player) {
            entry.1 = expires;
        } else {
            self.entries.push((player, expires));
        }
    }

    /// Whether a player has an active deserter penalty.
    pub fn is_deserter(&self, player: u64, now: u64) -> bool {
        self.entries
            .iter()
            .any(|(p, exp)| *p == player && now < *exp)
    }

    /// Remaining deserter time in seconds (0 if not active).
    pub fn remaining(&self, player: u64, now: u64) -> u64 {
        self.entries
            .iter()
            .find(|(p, _)| *p == player)
            .map_or(0, |(_, exp)| exp.saturating_sub(now))
    }

    /// Remove expired penalties.
    pub fn cleanup(&mut self, now: u64) {
        self.entries.retain(|(_, exp)| now < *exp);
    }
}

/// (queue_entry_index, member_index_within_entry)
type Slot = (usize, usize);

/// Try to find 1T+1H+3D from candidates for a specific dungeon.
fn find_composition(entries: &[QueueEntry], candidates: &[usize]) -> Option<Vec<Slot>> {
    let mut tank: Option<Slot> = None;
    let mut healer: Option<Slot> = None;
    let mut dps: Vec<Slot> = Vec::new();

    for &idx in candidates {
        let entry = &entries[idx];
        for (mi, role) in entry.roles.iter().enumerate() {
            let slot = (idx, mi);
            match role {
                GroupRole::Tank if tank.is_none() => tank = Some(slot),
                GroupRole::Healer if healer.is_none() => healer = Some(slot),
                GroupRole::Dps if dps.len() < 3 => dps.push(slot),
                // If tank/healer already filled, DPS can absorb extras
                GroupRole::Tank | GroupRole::Healer if dps.len() < 3 => {}
                _ => {}
            }
        }
    }

    let (Some(t), Some(h)) = (tank, healer) else {
        return None;
    };
    if dps.len() != 3 {
        return None;
    }
    let mut group = vec![t, h];
    group.extend(dps);
    Some(group)
}

fn build_match(entries: &[QueueEntry], group: &[Slot], dungeon_id: u32) -> MatchResult {
    let tank = entries[group[0].0].members[group[0].1];
    let healer = entries[group[1].0].members[group[1].1];
    let dps = group[2..]
        .iter()
        .map(|&(ei, mi)| entries[ei].members[mi])
        .collect();
    MatchResult {
        dungeon_id,
        tank,
        healer,
        dps,
    }
}

#[cfg(test)]
#[path = "dungeon_finder_tests.rs"]
mod tests;
