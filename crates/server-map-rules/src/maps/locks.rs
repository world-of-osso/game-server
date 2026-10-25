//! Instance locks (TrinityCore `InstanceLockMgr`, game/Instances/InstanceLockMgr.cpp).
//!
//! A dungeon difficulty with a reset schedule (`MapDifficulty.ResetInterval` daily or
//! weekly) locks players to what they killed there. Locks are per character and keyed
//! by (`MapID`, `LockID`), so the difficulties of a map sharing a `LockID` share one
//! lock. Creating a copy for an owner who has no lock gives the owner a *temporary*
//! lock (`CreateInstanceLockForNewInstance`); the first boss killed while a player is
//! inside makes that player's lock permanent (`UpdateInstanceLockForPlayer`), and the
//! lock then expires at the next reset (`GetNextResetTime`), or one period later when
//! extended.
//!
//! Instance-id-bound locks (`IsInstanceIdBound`: no encounter-lock flag) tie the player
//! to one copy: its shared boss state lives by instance id and a new copy of the same
//! id resumes it. Encounter locks (Mythic, `UseLootBasedLockInsteadOfInstanceLock`)
//! only record the killed encounters.
//!
//! Times are Unix seconds. The reset hour and weekday are Retail's (US: Tuesday
//! 15:00 UTC), in place of TrinityCore's `ResetSchedule.Hour` / `WeekDay` config.

use std::collections::HashMap;

use bevy::prelude::*;
use shared::protocol::{InstanceLockInfo, TransferAbortReason};

use super::store::{MapDifficulty, ResetInterval};

/// Retail's reset hour (UTC).
pub const RESET_HOUR_UTC: i64 = 15;
/// Retail's weekly reset day, `tm_wday` numbering (0 Sunday): Tuesday.
pub const RESET_WEEK_DAY: i64 = 2;
const DAY_SECS: i64 = 86_400;
/// 1970-01-01 was a Thursday.
const EPOCH_WEEK_DAY: i64 = 4;

/// `InstanceLockKey`: (`MapID`, `LockID`).
pub type LockKey = (u32, u32);

/// `InstanceLockData`: the saved instance state (`InstanceScript::GetSaveData` boss
/// states) and the completed encounters.
#[derive(Debug, Clone, Default, PartialEq, Eq, bitcode::Encode, bitcode::Decode)]
pub struct LockData {
    /// Boss ids (`SetBossNumber` index) whose state is `DONE`.
    pub done_bosses: u32,
    /// `CompletedEncountersMask`: `DungeonEncounter.Bit` of each killed boss.
    pub completed_encounters: u32,
}

/// Progress checkpoint of a normal copy. No scheduled bind or client lock entry;
/// it resumes the owner's run and is deleted on manual reset or copy unload.
#[derive(Debug, Clone, PartialEq, Eq, bitcode::Encode, bitcode::Decode)]
pub struct NormalInstanceLock {
    pub map_id: u32,
    pub difficulty_id: u32,
    pub instance_id: u32,
    pub owner: u64,
    pub data: LockData,
}

/// `InstanceLock` of one character.
#[derive(Debug, Clone, PartialEq, Eq, bitcode::Encode, bitcode::Decode)]
pub struct InstanceLock {
    pub map_id: u32,
    pub difficulty_id: u32,
    pub instance_id: u32,
    /// Unix seconds.
    pub expiry: i64,
    pub extended: bool,
    pub data: LockData,
    /// `_isNew`: created for a new copy, no boss killed under it yet (only temporary locks,
    /// which are never saved).
    pub is_new: bool,
}

impl InstanceLock {
    pub fn is_expired(&self, now: i64) -> bool {
        self.expiry <= now
    }

    /// `InstanceLock::GetEffectiveExpiryTime`.
    pub fn effective_expiry(&self, entries: &MapDifficulty, now: i64) -> i64 {
        if !self.extended {
            return self.expiry;
        }
        if self.is_expired(now) {
            return next_reset_time(entries.reset_interval, now);
        }
        self.expiry + entries.raid_duration_secs()
    }
}

/// A boss kill to record (`InstanceLockUpdateEvent` of `UpdateBossStateSaveDataEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockUpdate {
    pub instance_id: u32,
    /// The copy's `DONE` bosses after the kill (`UpdateBossStateSaveData`).
    pub done_bosses: u32,
    /// The copy's completed encounters after the kill.
    pub instance_completed_encounters: u32,
    /// `DungeonEncounter.Bit` of the killed boss.
    pub completed_encounter_bit: Option<u32>,
}

/// `InstanceLockMgr`.
#[derive(Resource, Debug, Default)]
pub struct InstanceLocks {
    /// `_instanceLocksByPlayer`.
    locks: HashMap<u64, HashMap<LockKey, InstanceLock>>,
    /// `_temporaryInstanceLocksByPlayer`: locks of copies no boss was killed in yet.
    temporary: HashMap<u64, HashMap<LockKey, InstanceLock>>,
    /// `SharedInstanceLockData` by instance id: the state every player's instance-id
    /// bound lock of that copy resumes.
    shared: HashMap<u32, LockData>,
}

impl InstanceLocks {
    /// Locks loaded from persistence (`InstanceLockMgr::Load`).
    pub fn from_saved(saved: SavedLocks) -> Self {
        Self {
            locks: saved.locks,
            temporary: HashMap::new(),
            shared: saved.shared,
        }
    }

    /// What persistence keeps: permanent locks and the shared state they reference.
    pub fn saved(&self) -> SavedLocks {
        SavedLocks {
            locks: self.locks.clone(),
            shared: self.shared.clone(),
        }
    }

    /// Every instance id a saved lock references (`MapManager::RegisterInstanceId`).
    pub fn instance_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.locks
            .values()
            .flat_map(|locks| locks.values().map(|lock| lock.instance_id))
    }

    /// `FindActiveInstanceLock(guid, entries, ignoreTemporary, ignoreExpired)`.
    fn find_active(
        &self,
        character: u64,
        entries: &MapDifficulty,
        ignore_temporary: bool,
        ignore_expired: bool,
        now: i64,
    ) -> Option<&InstanceLock> {
        if !entries.has_reset_schedule() {
            return None;
        }
        let key = entries.lock_key();
        let lock = self.locks.get(&character).and_then(|locks| locks.get(&key));
        if let Some(lock) = lock
            && (!lock.is_expired(now) || lock.extended || !ignore_expired)
        {
            return Some(lock);
        }
        if ignore_temporary {
            return None;
        }
        self.temporary
            .get(&character)
            .and_then(|locks| locks.get(&key))
    }

    /// `InstanceLockMgr::FindActiveInstanceLock(guid, entries)`: the permanent lock unless
    /// expired and not extended, else the temporary one.
    pub fn find_active_lock(
        &self,
        character: u64,
        entries: &MapDifficulty,
        now: i64,
    ) -> Option<&InstanceLock> {
        self.find_active(character, entries, false, true, now)
    }

    /// `InstanceLockMgr::CreateInstanceLockForNewInstance`: `None` without a reset schedule.
    pub fn create_for_new_instance(
        &mut self,
        owner: u64,
        entries: &MapDifficulty,
        instance_id: u32,
        now: i64,
    ) -> Option<&InstanceLock> {
        if !entries.has_reset_schedule() {
            return None;
        }
        if entries.is_instance_id_bound() {
            self.shared.insert(instance_id, LockData::default());
        }
        let lock = InstanceLock {
            map_id: entries.map_id,
            difficulty_id: entries.difficulty_id,
            instance_id,
            expiry: next_reset_time(entries.reset_interval, now),
            extended: false,
            data: LockData::default(),
            is_new: true,
        };
        let temporary = self.temporary.entry(owner).or_default();
        temporary.insert(entries.lock_key(), lock);
        temporary.get(&entries.lock_key())
    }

    /// `InstanceLock::SetInstanceId` on `owner`'s active lock: its instance id was taken
    /// by another owner's copy (encounter locks), so it names a new copy.
    pub fn set_instance_id(
        &mut self,
        owner: u64,
        entries: &MapDifficulty,
        instance_id: u32,
        now: i64,
    ) {
        let key = entries.lock_key();
        let permanent = self.find_active(owner, entries, true, true, now).is_some();
        let locks = if permanent {
            self.locks.get_mut(&owner)
        } else {
            self.temporary.get_mut(&owner)
        };
        if let Some(lock) = locks.and_then(|locks| locks.get_mut(&key)) {
            lock.instance_id = instance_id;
        }
    }

    /// The state a copy created under `owner`'s lock starts with
    /// (`GetInstanceInitializationData`: the shared state of an instance-id-bound lock,
    /// the lock's own otherwise); `None` for a new lock.
    pub fn initialization_data(
        &self,
        owner: u64,
        entries: &MapDifficulty,
        now: i64,
    ) -> Option<LockData> {
        let lock = self.find_active_lock(owner, entries, now)?;
        if lock.is_new {
            return None;
        }
        if entries.is_instance_id_bound() {
            return self.shared.get(&lock.instance_id).cloned();
        }
        Some(lock.data.clone())
    }

    /// `InstanceLockMgr::CanJoinInstanceLock`: a player saved to another copy of an
    /// instance-id-bound difficulty may not enter this one.
    pub fn can_join(
        &self,
        character: u64,
        entries: &MapDifficulty,
        instance_id: u32,
        now: i64,
        instance_completed_encounters: u32,
    ) -> Option<TransferAbortReason> {
        let lock = self.find_active_lock(character, entries, now)?;
        if entries.flexible_locking {
            return (lock.data.completed_encounters & !instance_completed_encounters != 0)
                .then_some(TransferAbortReason::AlreadyCompletedEncounter);
        }
        (!entries.is_using_encounter_locks() && !lock.is_new && lock.instance_id != instance_id)
            .then_some(TransferAbortReason::LockedToDifferentInstance)
    }

    /// `InstanceLockMgr::UpdateSharedInstanceLock`.
    pub fn update_shared(&mut self, update: &LockUpdate) {
        let shared = self.shared.entry(update.instance_id).or_default();
        shared.done_bosses = update.done_bosses;
        if let Some(bit) = update.completed_encounter_bit {
            shared.completed_encounters |= 1 << bit;
        }
    }

    /// `InstanceLockMgr::UpdateInstanceLockForPlayer`: the player's lock with the kill
    /// recorded; true when it is a new lock for the player (`InstanceSaveCreated`).
    pub fn update_for_player(
        &mut self,
        character: u64,
        entries: &MapDifficulty,
        update: &LockUpdate,
        now: i64,
    ) -> bool {
        let key = entries.lock_key();
        let previous = self.find_active_lock(character, entries, now);
        let is_new_lock = previous.is_none_or(|lock| lock.is_new || lock.is_expired(now));
        let permanent = self
            .find_active(character, entries, true, true, now)
            .is_some();
        if !permanent {
            // The temporary lock of the copy, else a fresh lock, replaces an expired one.
            let lock = self.take_temporary(character, key).unwrap_or(InstanceLock {
                map_id: entries.map_id,
                difficulty_id: entries.difficulty_id,
                instance_id: update.instance_id,
                expiry: next_reset_time(entries.reset_interval, now),
                extended: false,
                data: LockData::default(),
                is_new: false,
            });
            self.locks.entry(character).or_default().insert(key, lock);
        }
        let lock = self
            .locks
            .get_mut(&character)
            .and_then(|locks| locks.get_mut(&key))
            .expect("the player's lock was just inserted or found");
        lock.instance_id = update.instance_id;
        lock.difficulty_id = entries.difficulty_id;
        lock.is_new = false;
        lock.data.done_bosses = update.done_bosses;
        if let Some(bit) = update.completed_encounter_bit {
            lock.data.completed_encounters |= 1 << bit;
        }
        // Synchronize the copy's completed encounters into the player's for the UI.
        if !entries.is_using_encounter_locks() {
            lock.data.completed_encounters |= update.instance_completed_encounters;
        }
        if lock.is_expired(now) {
            lock.expiry = next_reset_time(entries.reset_interval, now);
            lock.extended = false;
        }
        is_new_lock
    }

    fn take_temporary(&mut self, character: u64, key: LockKey) -> Option<InstanceLock> {
        let locks = self.temporary.get_mut(&character)?;
        let lock = locks.remove(&key);
        if locks.is_empty() {
            self.temporary.remove(&character);
        }
        lock
    }

    /// `InstanceLockMgr::UpdateInstanceLockExtensionForPlayer`: (old, new) effective
    /// expiry, `None` without a lock.
    pub fn set_extended(
        &mut self,
        character: u64,
        entries: &MapDifficulty,
        extended: bool,
        now: i64,
    ) -> Option<(i64, i64)> {
        let lock = self
            .locks
            .get_mut(&character)?
            .get_mut(&entries.lock_key())?;
        let old = lock.effective_expiry(entries, now);
        lock.extended = extended;
        Some((old, lock.effective_expiry(entries, now)))
    }

    /// `InstanceLockMgr::ResetInstanceLocksForPlayer` for GM commands: each unexpired lock
    /// not in use (`in_use`) of `map_id` (all maps when `None`) expires at the last reset.
    pub fn reset_for_player(
        &mut self,
        character: u64,
        map_id: Option<u32>,
        in_use: impl Fn(&InstanceLock) -> bool,
        entries_of: impl Fn(&InstanceLock) -> Option<MapDifficulty>,
        now: i64,
    ) -> Vec<LockKey> {
        let Some(locks) = self.locks.get_mut(&character) else {
            return Vec::new();
        };
        let mut reset = Vec::new();
        for (key, lock) in locks.iter_mut() {
            if in_use(lock) || map_id.is_some_and(|map| map != lock.map_id) || lock.is_expired(now)
            {
                continue;
            }
            let Some(entries) = entries_of(lock) else {
                continue;
            };
            lock.expiry =
                next_reset_time(entries.reset_interval, now) - entries.raid_duration_secs();
            lock.extended = false;
            reset.push(*key);
        }
        reset
    }

    /// `Player::SendRaidInfo`: every permanent lock of the player, current and expired.
    pub fn raid_info(
        &self,
        character: u64,
        entries_of: impl Fn(&InstanceLock) -> Option<MapDifficulty>,
        now: i64,
    ) -> Vec<InstanceLockInfo> {
        let Some(locks) = self.locks.get(&character) else {
            return Vec::new();
        };
        let mut infos: Vec<InstanceLockInfo> = locks
            .values()
            .filter_map(|lock| {
                let entries = entries_of(lock)?;
                Some(InstanceLockInfo {
                    map_id: lock.map_id,
                    difficulty_id: lock.difficulty_id,
                    instance_id: lock.instance_id,
                    time_remaining_secs: (lock.effective_expiry(&entries, now) - now).max(0) as u32,
                    completed_mask: lock.data.completed_encounters,
                    locked: !lock.is_expired(now),
                    extended: lock.extended,
                })
            })
            .collect();
        infos.sort_by_key(|info| (info.map_id, info.difficulty_id));
        infos
    }

    /// The permanent lock of `character` on `key` (tests, admin output).
    pub fn lock(&self, character: u64, key: LockKey) -> Option<&InstanceLock> {
        self.locks.get(&character)?.get(&key)
    }
}

/// Permanent locks and shared instance states, as persisted.
#[derive(Debug, Clone, Default, PartialEq, Eq, bitcode::Encode, bitcode::Decode)]
pub struct SavedLocks {
    pub locks: HashMap<u64, HashMap<LockKey, InstanceLock>>,
    pub shared: HashMap<u32, LockData>,
}

/// `InstanceLockMgr::GetNextResetTime` (InstanceLockMgr.cpp:534-566) at Retail's reset
/// hour: the next daily reset, or the next weekly reset day, strictly after the hour of
/// `now` began. A difficulty without a schedule gets `now`.
pub fn next_reset_time(interval: ResetInterval, now: i64) -> i64 {
    let day_start = now.div_euclid(DAY_SECS) * DAY_SECS;
    let hour = now.rem_euclid(DAY_SECS) / 3600;
    let reset_today = day_start + RESET_HOUR_UTC * 3600;
    match interval {
        ResetInterval::Anytime => now,
        ResetInterval::Daily => {
            if hour >= RESET_HOUR_UTC {
                reset_today + DAY_SECS
            } else {
                reset_today
            }
        }
        ResetInterval::Weekly => {
            let week_day = (now.div_euclid(DAY_SECS) + EPOCH_WEEK_DAY).rem_euclid(7);
            let mut days = RESET_WEEK_DAY - week_day;
            if week_day > RESET_WEEK_DAY || (week_day == RESET_WEEK_DAY && hour >= RESET_HOUR_UTC) {
                days += 7;
            }
            reset_today + days * DAY_SECS
        }
    }
}

#[cfg(test)]
#[path = "locks_tests.rs"]
mod tests;
