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
        let total: u64 = self.entries.iter().map(|e| now.saturating_sub(e.queued_at)).sum();
        total as f32 / self.entries.len() as f32
    }
    /// Try to form a 1T+1H+3D group from queued entries with overlapping dungeons.
    ///
    /// Returns the matched entries (removed from queue) and the dungeon ID,
    /// or `None` if no valid group can be formed.
    pub fn try_match(&mut self) -> Option<MatchResult> {
        // Find a common dungeon across candidates
        let all_dungeons: Vec<u32> = self.entries
            .iter()
            .flat_map(|e| e.dungeon_ids.iter().copied())
            .collect();

        for dungeon_id in &all_dungeons {
            let candidates: Vec<usize> = self.entries
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
pub fn teleport_orders(
    match_result: &MatchResult,
    dungeon: &DungeonDef,
) -> Vec<TeleportOrder> {
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
        self.entries.iter().any(|(p, exp)| *p == player && now < *exp)
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
    let dps = group[2..].iter().map(|&(ei, mi)| entries[ei].members[mi]).collect();
    MatchResult { dungeon_id, tank, healer, dps }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_solo() {
        let mut q = DungeonQueue::default();
        assert!(q.queue_solo(1, GroupRole::Tank, vec![100], 1000).is_ok());
        assert_eq!(q.len(), 1);
        assert!(q.is_queued(1));
    }

    #[test]
    fn queue_group() {
        let mut q = DungeonQueue::default();
        q.queue_group(
            1, vec![1, 2, 3],
            vec![GroupRole::Tank, GroupRole::Healer, GroupRole::Dps],
            vec![100], 1000,
        ).unwrap();
        assert!(q.is_queued(1));
        assert!(q.is_queued(2));
        assert!(q.is_queued(3));
    }

    #[test]
    fn queue_already_queued() {
        let mut q = DungeonQueue::default();
        q.queue_solo(1, GroupRole::Dps, vec![100], 1000).unwrap();
        assert_eq!(
            q.queue_solo(1, GroupRole::Dps, vec![100], 1000),
            Err(QueueError::AlreadyQueued)
        );
    }

    #[test]
    fn queue_no_dungeons() {
        let mut q = DungeonQueue::default();
        assert_eq!(
            q.queue_solo(1, GroupRole::Dps, vec![], 1000),
            Err(QueueError::NoDungeons)
        );
    }

    #[test]
    fn dequeue() {
        let mut q = DungeonQueue::default();
        q.queue_solo(1, GroupRole::Tank, vec![100], 1000).unwrap();
        assert!(q.dequeue(1));
        assert!(!q.is_queued(1));
        assert!(q.is_empty());
    }

    #[test]
    fn dequeue_nonexistent() {
        let mut q = DungeonQueue::default();
        assert!(!q.dequeue(99));
    }

    #[test]
    fn average_wait_time() {
        let mut q = DungeonQueue::default();
        q.queue_solo(1, GroupRole::Dps, vec![100], 100).unwrap();
        q.queue_solo(2, GroupRole::Dps, vec![100], 200).unwrap();
        let avg = q.average_wait(400);
        // Player 1: 300s, Player 2: 200s → avg 250
        assert!((avg - 250.0).abs() < 0.01);
    }

    // --- Matchmaking tests ---

    fn queue_5_players(q: &mut DungeonQueue) {
        q.queue_solo(1, GroupRole::Tank, vec![100], 0).unwrap();
        q.queue_solo(2, GroupRole::Healer, vec![100], 0).unwrap();
        q.queue_solo(3, GroupRole::Dps, vec![100], 0).unwrap();
        q.queue_solo(4, GroupRole::Dps, vec![100], 0).unwrap();
        q.queue_solo(5, GroupRole::Dps, vec![100], 0).unwrap();
    }

    #[test]
    fn match_1t_1h_3d() {
        let mut q = DungeonQueue::default();
        queue_5_players(&mut q);
        let m = q.try_match().unwrap();
        assert_eq!(m.dungeon_id, 100);
        assert_eq!(m.tank, 1);
        assert_eq!(m.healer, 2);
        assert_eq!(m.dps.len(), 3);
        assert!(q.is_empty()); // all removed
    }

    #[test]
    fn no_match_missing_tank() {
        let mut q = DungeonQueue::default();
        q.queue_solo(1, GroupRole::Healer, vec![100], 0).unwrap();
        q.queue_solo(2, GroupRole::Dps, vec![100], 0).unwrap();
        q.queue_solo(3, GroupRole::Dps, vec![100], 0).unwrap();
        q.queue_solo(4, GroupRole::Dps, vec![100], 0).unwrap();
        assert!(q.try_match().is_none());
    }

    #[test]
    fn no_match_missing_healer() {
        let mut q = DungeonQueue::default();
        q.queue_solo(1, GroupRole::Tank, vec![100], 0).unwrap();
        q.queue_solo(2, GroupRole::Dps, vec![100], 0).unwrap();
        q.queue_solo(3, GroupRole::Dps, vec![100], 0).unwrap();
        q.queue_solo(4, GroupRole::Dps, vec![100], 0).unwrap();
        assert!(q.try_match().is_none());
    }

    #[test]
    fn no_match_different_dungeons() {
        let mut q = DungeonQueue::default();
        q.queue_solo(1, GroupRole::Tank, vec![100], 0).unwrap();
        q.queue_solo(2, GroupRole::Healer, vec![200], 0).unwrap(); // different dungeon
        q.queue_solo(3, GroupRole::Dps, vec![100], 0).unwrap();
        q.queue_solo(4, GroupRole::Dps, vec![100], 0).unwrap();
        q.queue_solo(5, GroupRole::Dps, vec![100], 0).unwrap();
        assert!(q.try_match().is_none());
    }

    #[test]
    fn match_removes_from_queue() {
        let mut q = DungeonQueue::default();
        queue_5_players(&mut q);
        q.queue_solo(6, GroupRole::Dps, vec![200], 0).unwrap();
        q.try_match();
        assert_eq!(q.len(), 1); // player 6 remains
        assert!(q.is_queued(6));
    }

    #[test]
    fn match_overlapping_dungeons() {
        let mut q = DungeonQueue::default();
        q.queue_solo(1, GroupRole::Tank, vec![100, 200], 0).unwrap();
        q.queue_solo(2, GroupRole::Healer, vec![200, 300], 0).unwrap();
        q.queue_solo(3, GroupRole::Dps, vec![100, 200], 0).unwrap();
        q.queue_solo(4, GroupRole::Dps, vec![200], 0).unwrap();
        q.queue_solo(5, GroupRole::Dps, vec![200, 300], 0).unwrap();
        let m = q.try_match().unwrap();
        // All overlap on dungeon 200
        assert_eq!(m.dungeon_id, 200);
    }

    // --- Teleport tests ---

    fn test_dungeon() -> DungeonDef {
        DungeonDef {
            id: 100,
            name: "Deadmines".into(),
            map_id: 36,
            entrance_x: -11208.0,
            entrance_y: 1673.0,
            entrance_z: 24.0,
            min_level: 15,
            max_level: 21,
        }
    }

    #[test]
    fn teleport_all_5_players() {
        let m = MatchResult {
            dungeon_id: 100,
            tank: 1,
            healer: 2,
            dps: vec![3, 4, 5],
        };
        let dungeon = test_dungeon();
        let orders = teleport_orders(&m, &dungeon);
        assert_eq!(orders.len(), 5);
        for order in &orders {
            assert_eq!(order.map_id, 36);
            assert_eq!(order.x, -11208.0);
        }
        let players: Vec<u64> = orders.iter().map(|o| o.player).collect();
        assert!(players.contains(&1));
        assert!(players.contains(&2));
        assert!(players.contains(&5));
    }

    #[test]
    fn dungeon_def_level_range() {
        let d = test_dungeon();
        assert_eq!(d.min_level, 15);
        assert_eq!(d.max_level, 21);
    }

    // --- Deserter tests ---

    #[test]
    fn deserter_applied_and_checked() {
        let mut tracker = DeserterTracker::default();
        tracker.apply(1, 10000);
        assert!(tracker.is_deserter(1, 10000));
        assert!(tracker.is_deserter(1, 11799)); // 1799s later, still active
        assert!(!tracker.is_deserter(1, 11800)); // 1800s = expired
    }

    #[test]
    fn deserter_remaining() {
        let mut tracker = DeserterTracker::default();
        tracker.apply(1, 10000);
        assert_eq!(tracker.remaining(1, 10000), 1800);
        assert_eq!(tracker.remaining(1, 10500), 1300);
        assert_eq!(tracker.remaining(1, 12000), 0);
    }

    #[test]
    fn deserter_not_active_for_others() {
        let mut tracker = DeserterTracker::default();
        tracker.apply(1, 10000);
        assert!(!tracker.is_deserter(2, 10000));
    }

    #[test]
    fn deserter_cleanup() {
        let mut tracker = DeserterTracker::default();
        tracker.apply(1, 10000);
        tracker.apply(2, 11000);
        tracker.cleanup(11800); // player 1 expired, player 2 still active
        assert!(!tracker.is_deserter(1, 11800));
        assert!(tracker.is_deserter(2, 11800));
    }

    #[test]
    fn deserter_refresh_extends() {
        let mut tracker = DeserterTracker::default();
        tracker.apply(1, 10000); // expires 11800
        tracker.apply(1, 11000); // refreshes to 12800
        assert!(tracker.is_deserter(1, 12000));
    }
}
