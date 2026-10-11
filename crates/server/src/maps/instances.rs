//! Instance copies of dungeon maps (TrinityCore `MapManager::CreateMap`,
//! `FindInstanceIdForPlayer`, `InstanceMap::CannotEnter`, `InstanceMap::RemovePlayerFromMap`,
//! `InstanceMap::Reset`).
//!
//! A copy is created at the entering group's (else player's) dungeon difficulty,
//! downscaled to one the map offers (`GetDownscaledMapDifficultyData`). A difficulty
//! without reset schedule has no instance lock: a group enters the instance it
//! created last (`Group::GetRecentInstanceId`), a player outside a group the one it
//! created last (`Player::GetRecentInstanceId`). With a reset schedule, the instance
//! owner's lock ([`InstanceLocks`]) names the copy; the owner is the player, or the
//! group's recent instance owner, else its leader (`Group::GetRecentInstanceOwner`).
//! With none, or once that copy unloaded, a new copy is created and remembered. An
//! instance with no players unloads after `Instance.UnloadDelay` (30 min), which
//! resets its content: the next entry creates a fresh copy, resuming a lock's saved
//! boss states. A copy whose lock reaches its reset expires (`Reset(Expire)`).

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use shared::protocol::TransferAbortReason;

use super::InMap;
use super::locks::{InstanceLocks, LockData, NormalInstanceLock, next_reset_time};
use super::store::{InstanceType, MapDifficulty, MapStore};
use crate::group::GroupId;

/// `Instance.UnloadDelay` default (worldserver.conf.dist): 30 minutes.
pub const INSTANCE_UNLOAD_DELAY_SECS: f64 = 1800.0;
/// `MIN_UNLOAD_DELAY`: an expired copy left empty unloads at once.
const MIN_UNLOAD_DELAY_SECS: f64 = 1.0;

/// The world maps the server hosts; other world maps are refused.
pub const HOSTED_CONTINENTS: [u32; 3] = [0, 1, 2991];

/// Who a player enters instances with: its character, its group and the group's
/// leader, and the dungeon difficulty it enters at (the group's in a group).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceEntrant {
    pub character_id: u64,
    pub group: Option<GroupId>,
    /// The group's leader; the player itself outside a group.
    pub leader: u64,
    /// `Group::GetDifficultyID` / `Player::GetDifficultyID` for dungeons.
    pub difficulty_id: u32,
}

#[derive(Debug)]
struct InstanceCopy {
    difficulty: MapDifficulty,
    max_players: u32,
    /// `InstanceMap::GetOwningGroup`: the group that created it.
    owning_group: Option<GroupId>,
    /// `i_instanceLock`'s owner: set for a difficulty with a reset schedule.
    lock_owner: Option<u64>,
    /// Run owner, including normal copies with no scheduled bind.
    owner: u64,
    /// A boss died under the lock (`!i_instanceLock->IsNew()`): no reset but by expiry.
    lock_used: bool,
    /// The boss states the copy was created with (`CreateInstanceData` → `Load`).
    initial_data: LockData,
    /// `i_instanceExpireEvent`: Unix seconds of the lock's reset.
    expires_at: Option<i64>,
    players: HashSet<Entity>,
    /// `m_unloadTimer` as an absolute time; `None` while players are inside.
    unload_at: Option<f64>,
}

#[derive(Resource, Debug)]
pub struct InstanceManager {
    next_instance_id: u32,
    copies: HashMap<InMap, InstanceCopy>,
    recent_by_character: HashMap<(u64, u32), u32>,
    /// `Group::m_recentInstances`: map → (instance owner, instance id).
    recent_by_group: HashMap<(GroupId, u32), (u64, u32)>,
    /// `sInstanceLockMgr`.
    pub locks: InstanceLocks,
    pub normal_locks: HashMap<u32, NormalInstanceLock>,
    /// A lock changed since persistence last saved them.
    pub locks_dirty: bool,
}

impl Default for InstanceManager {
    fn default() -> Self {
        Self {
            next_instance_id: 1,
            copies: HashMap::new(),
            recent_by_character: HashMap::new(),
            recent_by_group: HashMap::new(),
            locks: InstanceLocks::default(),
            normal_locks: HashMap::new(),
            locks_dirty: false,
        }
    }
}

/// A map copy `create_map` resolved, and whether it was just created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedMap {
    pub map: InMap,
    pub created: bool,
}

/// Times `InstanceManager` reads: `Time` elapsed seconds (unload timers) and Unix
/// seconds (lock expiry, `GameTime::GetSystemTime`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Now {
    pub elapsed: f64,
    pub unix: i64,
}

/// What `InstanceMap::Reset` did (`InstanceResetResult`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceResetResult {
    Success,
    NotEmpty,
    CannotReset,
}

impl InstanceManager {
    /// Locks loaded at startup; their instance ids are not handed out again
    /// (`MapManager::RegisterInstanceId`).
    pub fn with_locks(locks: InstanceLocks) -> Self {
        let next_instance_id = locks.instance_ids().max().map_or(1, |id| id + 1);
        Self {
            next_instance_id,
            locks,
            ..Default::default()
        }
    }

    pub fn with_locks_and_normal(locks: InstanceLocks, normal: Vec<NormalInstanceLock>) -> Self {
        let mut manager = Self::with_locks(locks);
        for lock in normal {
            manager.next_instance_id = manager.next_instance_id.max(lock.instance_id + 1);
            manager
                .recent_by_character
                .insert((lock.owner, lock.map_id), lock.instance_id);
            manager.normal_locks.insert(lock.instance_id, lock);
        }
        manager
    }

    /// The difficulty `entrant` enters `map_id` at, `None` when the map offers nothing
    /// it downscales to.
    fn entry_difficulty<'a>(
        store: &'a MapStore,
        map_id: u32,
        entrant: InstanceEntrant,
    ) -> Option<&'a MapDifficulty> {
        store.downscaled_map_difficulty(map_id, entrant.difficulty_id)
    }

    /// `Group::GetRecentInstanceOwner`, or the player outside a group.
    fn instance_owner(&self, map_id: u32, entrant: InstanceEntrant) -> u64 {
        match entrant.group {
            Some(group) => self
                .recent_by_group
                .get(&(group, map_id))
                .map_or(entrant.leader, |&(owner, _)| owner),
            None => entrant.character_id,
        }
    }

    fn recent_instance(&self, map_id: u32, entrant: InstanceEntrant) -> Option<u32> {
        match entrant.group {
            Some(group) => self
                .recent_by_group
                .get(&(group, map_id))
                .map(|&(_, instance_id)| instance_id),
            None => self
                .recent_by_character
                .get(&(entrant.character_id, map_id))
                .copied(),
        }
    }

    /// The instance id the owner's lock or the recent instance names, and the
    /// difficulty it is at (the lock's).
    fn bound_instance<'a>(
        &self,
        store: &'a MapStore,
        map_id: u32,
        entrant: InstanceEntrant,
        unix: i64,
    ) -> Option<(Option<u32>, &'a MapDifficulty, u64)> {
        let entries = Self::entry_difficulty(store, map_id, entrant)?;
        let owner = self.instance_owner(map_id, entrant);
        if let Some(lock) = self.locks.find_active_lock(owner, entries, unix) {
            // Reset difficulty to the one used in instance lock.
            let entries = store
                .downscaled_map_difficulty(map_id, lock.difficulty_id)
                .unwrap_or(entries);
            return Some((Some(lock.instance_id), entries, owner));
        }
        let recent = if entries.has_reset_schedule() {
            None
        } else {
            self.recent_instance(map_id, entrant).or_else(|| {
                self.normal_locks
                    .values()
                    .find(|lock| {
                        lock.owner == owner
                            && lock.map_id == map_id
                            && lock.difficulty_id == entries.difficulty_id
                    })
                    .map(|lock| lock.instance_id)
            })
        };
        Some((recent, entries, owner))
    }

    /// `MapManager::FindInstanceIdForPlayer`: the existing copy `entrant` would enter.
    pub fn find_instance(
        &self,
        store: &MapStore,
        map_id: u32,
        entrant: InstanceEntrant,
        unix: i64,
    ) -> Option<InMap> {
        let (instance_id, entries, owner) = self.bound_instance(store, map_id, entrant, unix)?;
        let map = InMap {
            map_id,
            instance_id: instance_id?,
        };
        let copy = self.copies.get(&map)?;
        // The instance id may be in use by another owner's copy for encounter locks.
        (entries.is_instance_id_bound() || copy.lock_owner.is_none_or(|o| o == owner))
            .then_some(map)
            .filter(|_| copy.difficulty.difficulty_id == entries.difficulty_id)
    }

    /// `MapManager::CreateMap` for a player: the continent itself, or the dungeon copy
    /// `entrant` enters, created when missing.
    pub fn create_map(
        &mut self,
        store: &MapStore,
        map_id: u32,
        entrant: InstanceEntrant,
        now: Now,
    ) -> Result<ResolvedMap, TransferAbortReason> {
        if HOSTED_CONTINENTS.contains(&map_id) {
            return Ok(ResolvedMap {
                map: InMap::continent(map_id),
                created: false,
            });
        }
        let entry = store
            .map(map_id)
            .ok_or(TransferAbortReason::MapNotAllowed)?;
        match entry.instance_type {
            InstanceType::Party => self.create_dungeon(store, map_id, entrant, now),
            _ => Err(TransferAbortReason::MapNotAllowed),
        }
    }

    fn create_dungeon(
        &mut self,
        store: &MapStore,
        map_id: u32,
        entrant: InstanceEntrant,
        now: Now,
    ) -> Result<ResolvedMap, TransferAbortReason> {
        let (bound_id, entries, owner) = self
            .bound_instance(store, map_id, entrant, now.unix)
            .ok_or(TransferAbortReason::Difficulty(entrant.difficulty_id))?;
        let entries = *entries;
        let locked = self
            .locks
            .find_active_lock(owner, &entries, now.unix)
            .is_some();
        let mut instance_id = bound_id.unwrap_or_else(|| self.generate_instance_id());
        if !locked {
            self.locks
                .create_for_new_instance(owner, &entries, instance_id, now.unix);
        }
        let mut map = InMap {
            map_id,
            instance_id,
        };
        // Deviation: a remembered copy at another difficulty (one a difficulty change
        // could not reset) is not entered; TrinityCore's recent instance id would be.
        if !locked
            && self
                .copies
                .get(&map)
                .is_some_and(|copy| copy.difficulty.difficulty_id != entries.difficulty_id)
        {
            instance_id = self.generate_instance_id();
            map.instance_id = instance_id;
        }
        // An encounter-lock id may already be in use by another owner's copy.
        if !entries.is_instance_id_bound()
            && entries.has_reset_schedule()
            && self
                .copies
                .get(&map)
                .is_some_and(|copy| copy.lock_owner != Some(owner))
        {
            instance_id = self.generate_instance_id();
            self.locks
                .set_instance_id(owner, &entries, instance_id, now.unix);
            map.instance_id = instance_id;
        }
        if self.copies.contains_key(&map) {
            return Ok(ResolvedMap {
                map,
                created: false,
            });
        }
        self.insert_copy(store, map, &entries, entrant, owner, now);
        match entrant.group {
            Some(group) => {
                self.recent_by_group
                    .insert((group, map_id), (owner, instance_id));
            }
            None => {
                self.recent_by_character
                    .insert((entrant.character_id, map_id), instance_id);
            }
        }
        info!(
            "Created instance {instance_id} of map {map_id} at difficulty {} for {entrant:?}",
            entries.difficulty_id
        );
        Ok(ResolvedMap { map, created: true })
    }

    /// `MapManager::CreateInstance` → `InstanceMap` + `CreateInstanceData`.
    fn insert_copy(
        &mut self,
        store: &MapStore,
        map: InMap,
        entries: &MapDifficulty,
        entrant: InstanceEntrant,
        owner: u64,
        now: Now,
    ) {
        let max_players = match entries.max_players {
            0 => store.map(map.map_id).map_or(0, |entry| entry.max_players),
            max => max,
        };
        let lock = self.locks.find_active_lock(owner, entries, now.unix);
        let lock_used = lock.is_some_and(|lock| !lock.is_new);
        let expires_at = lock.map(|lock| lock.expiry);
        // A grouped encounter-lock dungeon starts fresh unless it restores dungeon
        // progress (`IsRestoringDungeonState`, none of the hosted ones).
        let initial_data = if !entries.has_reset_schedule() {
            self.normal_locks
                .get(&map.instance_id)
                .filter(|lock| {
                    lock.owner == owner
                        && lock.map_id == map.map_id
                        && lock.difficulty_id == entries.difficulty_id
                })
                .map(|lock| lock.data.clone())
                .unwrap_or_default()
        } else if !entries.is_instance_id_bound() && entrant.group.is_some() {
            LockData::default()
        } else {
            self.locks
                .initialization_data(owner, entries, now.unix)
                .unwrap_or_default()
        };
        self.copies.insert(
            map,
            InstanceCopy {
                difficulty: *entries,
                max_players,
                owning_group: entrant.group,
                lock_owner: entries.has_reset_schedule().then_some(owner),
                owner,
                lock_used,
                initial_data,
                expires_at,
                players: HashSet::new(),
                // A new InstanceMap starts its unload timer (Map.cpp:2878).
                unload_at: Some(now.elapsed + INSTANCE_UNLOAD_DELAY_SECS),
            },
        );
    }

    fn generate_instance_id(&mut self) -> u32 {
        let id = self.next_instance_id;
        self.next_instance_id += 1;
        id
    }

    /// (map id, instance id) of every loaded copy with a lock (`InstanceLock::IsInUse`).
    pub fn lock_copies_in_use(&self) -> Vec<(u32, u32)> {
        self.copies
            .iter()
            .filter(|(_, copy)| copy.lock_owner.is_some())
            .map(|(map, _)| (map.map_id, map.instance_id))
            .collect()
    }

    /// The `MapDifficulty` row copy `map` was created at; `None` for continents.
    pub fn copy_difficulty(&self, map: InMap) -> Option<&MapDifficulty> {
        self.copies.get(&map).map(|copy| &copy.difficulty)
    }

    /// The `DONE` bosses copy `map` was created with (boss id bits).
    pub fn initial_done_bosses(&self, map: InMap) -> u32 {
        self.copies
            .get(&map)
            .map_or(0, |copy| copy.initial_data.done_bosses)
    }

    /// `InstanceMap::CannotEnter` for a player outside the copy: full, saved to another
    /// copy (`CanJoinInstanceLock`), or owned by a group the player is not in (both
    /// full and foreign are `TRANSFER_ABORT_MAX_PLAYERS`).
    pub fn cannot_enter(
        &self,
        map: InMap,
        entrant: InstanceEntrant,
        unix: i64,
    ) -> Option<TransferAbortReason> {
        let copy = self.copies.get(&map)?;
        if copy.players.len() as u32 >= copy.max_players {
            return Some(TransferAbortReason::MaxPlayers);
        }
        if copy.lock_owner.is_some()
            && let Some(reason) = self.locks.can_join(
                entrant.character_id,
                &copy.difficulty,
                map.instance_id,
                unix,
            )
        {
            return Some(reason);
        }
        if copy.owning_group.is_some() && copy.owning_group != entrant.group {
            return Some(TransferAbortReason::MaxPlayers);
        }
        None
    }

    /// `Map::PlayerCannotEnter` (Map.cpp:1807-1852), checked before a far teleport:
    /// the map must offer a difficulty the entrant's downscales to, a player at `level`
    /// must meet its level conditions (`Player::Satisfy`), and the copy the entrant is
    /// bound to must admit it. World maps other than the hosted continents,
    /// raids, battlegrounds and scenarios are not hosted.
    pub fn player_cannot_enter(
        &self,
        store: &MapStore,
        map_id: u32,
        entrant: InstanceEntrant,
        level: u8,
        unix: i64,
    ) -> Option<TransferAbortReason> {
        if HOSTED_CONTINENTS.contains(&map_id) {
            return None;
        }
        let Some(entry) = store.map(map_id) else {
            return Some(TransferAbortReason::MapNotAllowed);
        };
        match entry.instance_type {
            InstanceType::Party => {
                let Some(difficulty) = Self::entry_difficulty(store, map_id, entrant) else {
                    return Some(TransferAbortReason::Difficulty(entrant.difficulty_id));
                };
                if let Some(condition) = store.unmet_condition(difficulty.id, level) {
                    return Some(TransferAbortReason::Condition(condition.failure()));
                }
                self.find_instance(store, map_id, entrant, unix)
                    .and_then(|map| self.cannot_enter(map, entrant, unix))
            }
            _ => Some(TransferAbortReason::MapNotAllowed),
        }
    }

    /// `Map::AddPlayerToMap`: the copy stops its unload timer.
    pub fn add_player(&mut self, map: InMap, player: Entity) {
        if let Some(copy) = self.copies.get_mut(&map) {
            copy.players.insert(player);
            copy.unload_at = None;
        }
    }

    /// `InstanceMap::RemovePlayerFromMap`: the last player out starts the unload timer,
    /// at once when the copy's lock has expired.
    pub fn remove_player(&mut self, map: InMap, player: Entity, now: Now) {
        let locks = &self.locks;
        if let Some(copy) = self.copies.get_mut(&map)
            && copy.players.remove(&player)
            && copy.players.is_empty()
        {
            let expired = copy
                .lock_owner
                .and_then(|owner| locks.lock(owner, copy.difficulty.lock_key()))
                .is_some_and(|lock| lock.is_expired(now.unix));
            let delay = if expired {
                MIN_UNLOAD_DELAY_SECS
            } else {
                INSTANCE_UNLOAD_DELAY_SECS
            };
            copy.unload_at = Some(now.elapsed + delay);
        }
    }

    /// Copies whose unload timer ran out (`Map::CanUnload`), forgotten here; their
    /// entities are despawned by the caller.
    pub fn take_expired(&mut self, elapsed: f64) -> Vec<InMap> {
        let expired: Vec<InMap> = self
            .copies
            .iter()
            .filter(|(_, copy)| copy.unload_at.is_some_and(|at| at <= elapsed))
            .map(|(&map, _)| map)
            .collect();
        for map in &expired {
            self.copies.remove(map);
            if self.normal_locks.remove(&map.instance_id).is_some() {
                self.locks_dirty = true;
            }
        }
        expired
    }

    /// `InstanceMap::Update`'s expire event: copies whose lock reset passed are reset
    /// (`Reset(Expire)`) and scheduled for the next reset. Returns the copies with
    /// players inside, whose players are told the lock expired.
    pub fn expire_locked_copies(&mut self, now: Now) -> Vec<InMap> {
        let mut occupied = Vec::new();
        for (&map, copy) in self.copies.iter_mut() {
            let Some(at) = copy.expires_at else {
                continue;
            };
            if at >= now.unix {
                continue;
            }
            copy.expires_at = Some(next_reset_time(copy.difficulty.reset_interval, now.unix));
            if copy.players.is_empty() {
                copy.unload_at = Some(now.elapsed + MIN_UNLOAD_DELAY_SECS);
            } else {
                occupied.push(map);
            }
        }
        occupied
    }

    /// `InstanceMap::Reset` for a manual reset or a difficulty change.
    fn reset(&mut self, map: InMap, now: Now) -> Option<InstanceResetResult> {
        if !self.copies.contains_key(&map) && self.normal_locks.remove(&map.instance_id).is_some() {
            self.locks_dirty = true;
            return Some(InstanceResetResult::Success);
        }
        let copy = self.copies.get_mut(&map)?;
        if copy.lock_owner.is_some() && copy.lock_used {
            return Some(InstanceResetResult::CannotReset);
        }
        if !copy.players.is_empty() {
            return Some(InstanceResetResult::NotEmpty);
        }
        copy.unload_at = Some(now.elapsed + MIN_UNLOAD_DELAY_SECS);
        if self.normal_locks.remove(&map.instance_id).is_some() {
            self.locks_dirty = true;
        }
        Some(InstanceResetResult::Success)
    }

    /// `Player::ResetInstances` / `Group::ResetInstances` (for `ResetInstances` the
    /// entrant is the leader): each remembered copy is reset; the ones that reset, and
    /// on a difficulty change the occupied and locked ones too, are forgotten. Returns
    /// the (map id, result) of each copy tried.
    pub fn reset_instances(
        &mut self,
        entrant: InstanceEntrant,
        on_difficulty_change: bool,
        now: Now,
    ) -> Vec<(u32, InstanceResetResult)> {
        let remembered: Vec<(u32, u32)> = match entrant.group {
            Some(group) => self
                .recent_by_group
                .iter()
                .filter(|((g, _), _)| *g == group)
                .map(|(&(_, map_id), &(_, instance_id))| (map_id, instance_id))
                .collect(),
            None => self
                .recent_by_character
                .iter()
                .filter(|((character, _), _)| *character == entrant.character_id)
                .map(|(&(_, map_id), &instance_id)| (map_id, instance_id))
                .collect(),
        };
        let mut results = Vec::new();
        for (map_id, instance_id) in remembered {
            let map = InMap {
                map_id,
                instance_id,
            };
            let Some(result) = self.reset(map, now) else {
                continue;
            };
            let forget = match result {
                InstanceResetResult::Success => true,
                InstanceResetResult::NotEmpty => on_difficulty_change,
                // Group::ResetInstances forgets it; Player::ResetInstances keeps it.
                InstanceResetResult::CannotReset => entrant.group.is_some(),
            };
            if forget {
                match entrant.group {
                    Some(group) => self.recent_by_group.remove(&(group, map_id)),
                    None => self
                        .recent_by_character
                        .remove(&(entrant.character_id, map_id))
                        .map(|id| (entrant.character_id, id)),
                };
            }
            results.push((map_id, result));
        }
        results.sort_by_key(|&(map_id, _)| map_id);
        results
    }

    pub fn completed_encounters(&self, map: InMap) -> u32 {
        self.copies
            .get(&map)
            .map_or(0, |copy| copy.initial_data.completed_encounters)
    }

    /// `InstanceMap::UpdateInstanceLock` for a boss of copy `map` becoming `DONE`:
    /// the shared state (instance-id bound) and each player's lock record the kill.
    /// Returns the players saved for the first time (`InstanceSaveCreated`).
    pub fn update_instance_lock(
        &mut self,
        map: InMap,
        players: &[(Entity, u64)],
        done_bosses: u32,
        encounter_bit: Option<u32>,
        unix: i64,
    ) -> Vec<Entity> {
        let Some(copy) = self.copies.get_mut(&map) else {
            return Vec::new();
        };
        copy.initial_data.done_bosses = done_bosses;
        if let Some(bit) = encounter_bit {
            copy.initial_data.completed_encounters |= 1 << bit;
        }
        // Normal dungeons have no scheduled lock, but still track their copy's progress.
        if copy.lock_owner.is_none() {
            self.normal_locks.insert(
                map.instance_id,
                NormalInstanceLock {
                    map_id: map.map_id,
                    difficulty_id: copy.difficulty.difficulty_id,
                    instance_id: map.instance_id,
                    owner: copy.owner,
                    data: copy.initial_data.clone(),
                },
            );
            self.locks_dirty = true;
            return Vec::new();
        }
        copy.lock_used = true;
        let update = super::locks::LockUpdate {
            instance_id: map.instance_id,
            done_bosses,
            instance_completed_encounters: copy.initial_data.completed_encounters,
            completed_encounter_bit: encounter_bit,
        };
        let entries = copy.difficulty;
        if entries.is_instance_id_bound() {
            self.locks.update_shared(&update);
        }
        let mut saved = Vec::new();
        for &(player, character) in players {
            if self
                .locks
                .update_for_player(character, &entries, &update, unix)
            {
                saved.push(player);
            }
        }
        self.locks_dirty = true;
        saved
    }
}
