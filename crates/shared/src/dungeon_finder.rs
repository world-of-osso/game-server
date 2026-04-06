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
}
