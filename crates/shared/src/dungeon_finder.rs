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
}
