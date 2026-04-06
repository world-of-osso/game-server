//! Per-character instance save/lockout tracking.
//!
//! Ref: AzerothCore `InstanceSaveMgr.cpp`, `InstancePlayerBind`.

use serde::{Deserialize, Serialize};

use crate::instance::{Difficulty, ResetTimer};

/// A character's saved binding to a specific instance.
///
/// Created when a boss is killed in a lockout-tracked difficulty.
/// Prevents re-entering a fresh instance until the lockout expires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceSave {
    /// Map ID of the instance.
    pub map_id: u16,
    /// Difficulty the lockout applies to.
    pub difficulty: Difficulty,
    /// The specific instance ID the character is bound to.
    pub instance_id: u32,
    /// Bitmask of completed boss encounters (bit N = encounter N).
    pub completed_encounters: u32,
    /// Timestamp when the lockout was first acquired.
    pub acquired_at: u64,
    /// Reset timer inherited from the difficulty config.
    pub reset_timer: ResetTimer,
}

impl InstanceSave {
    /// Timestamp when this lockout expires.
    pub fn expires_at(&self) -> Option<u64> {
        self.reset_timer.next_reset(self.acquired_at)
    }

    /// Whether this lockout has expired.
    pub fn is_expired(&self, now: u64) -> bool {
        self.expires_at().is_some_and(|t| now >= t)
    }

    /// Record a boss kill (encounter index 0-31).
    pub fn complete_encounter(&mut self, encounter_index: u8) {
        self.completed_encounters |= 1 << encounter_index;
    }

    /// Whether a specific encounter has been completed.
    pub fn is_encounter_done(&self, encounter_index: u8) -> bool {
        self.completed_encounters & (1 << encounter_index) != 0
    }

    /// Number of completed encounters.
    pub fn completed_count(&self) -> u32 {
        self.completed_encounters.count_ones()
    }
}

/// Per-character instance lockout tracker.
///
/// Each character has their own lockouts independent of group.
/// A character locked to heroic Shadowfang Keep cannot enter a fresh
/// heroic Shadowfang Keep until the daily reset, but can enter normal.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CharacterLockouts {
    saves: Vec<InstanceSave>,
}

impl CharacterLockouts {
    /// Bind a character to an instance after a boss kill.
    ///
    /// If already bound to the same map+difficulty, updates the encounter mask.
    /// Otherwise creates a new save.
    pub fn bind(
        &mut self,
        map_id: u16,
        difficulty: Difficulty,
        instance_id: u32,
        encounter_index: u8,
        reset_timer: ResetTimer,
        now: u64,
    ) {
        if let Some(save) = self.find_mut(map_id, difficulty) {
            save.complete_encounter(encounter_index);
            return;
        }
        let mut save = InstanceSave {
            map_id,
            difficulty,
            instance_id,
            completed_encounters: 0,
            acquired_at: now,
            reset_timer,
        };
        save.complete_encounter(encounter_index);
        self.saves.push(save);
    }

    /// Check if a character is locked out of a map at a given difficulty.
    pub fn is_locked(&self, map_id: u16, difficulty: Difficulty, now: u64) -> bool {
        self.saves
            .iter()
            .any(|s| s.map_id == map_id && s.difficulty == difficulty && !s.is_expired(now))
    }

    /// Get the active save for a map+difficulty, if any.
    pub fn find(&self, map_id: u16, difficulty: Difficulty) -> Option<&InstanceSave> {
        self.saves
            .iter()
            .find(|s| s.map_id == map_id && s.difficulty == difficulty)
    }

    fn find_mut(&mut self, map_id: u16, difficulty: Difficulty) -> Option<&mut InstanceSave> {
        self.saves
            .iter_mut()
            .find(|s| s.map_id == map_id && s.difficulty == difficulty)
    }

    /// Get the instance ID a character is bound to for a map+difficulty.
    ///
    /// Used to rejoin the same instance after disconnect/relog.
    pub fn bound_instance(&self, map_id: u16, difficulty: Difficulty, now: u64) -> Option<u32> {
        self.find(map_id, difficulty)
            .filter(|s| !s.is_expired(now))
            .map(|s| s.instance_id)
    }

    /// Remove all expired lockouts.
    pub fn cleanup(&mut self, now: u64) {
        self.saves.retain(|s| !s.is_expired(now));
    }

    /// All active (non-expired) lockouts.
    pub fn active_lockouts(&self, now: u64) -> Vec<&InstanceSave> {
        self.saves.iter().filter(|s| !s.is_expired(now)).collect()
    }

    /// Next reset time across all active lockouts, or `None` if no lockouts.
    pub fn next_reset(&self, now: u64) -> Option<u64> {
        self.saves
            .iter()
            .filter(|s| !s.is_expired(now))
            .filter_map(|s| s.expires_at())
            .min()
    }

    /// Total number of saves (including expired).
    pub fn len(&self) -> usize {
        self.saves.len()
    }

    /// Whether the character has no saves.
    pub fn is_empty(&self) -> bool {
        self.saves.is_empty()
    }
}

#[cfg(test)]
#[path = "instance_lockout_tests.rs"]
mod tests;
