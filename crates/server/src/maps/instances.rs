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

#[path = "housing.rs"]
mod housing;

/// `Instance.UnloadDelay` default (worldserver.conf.dist): 30 minutes.
pub const INSTANCE_UNLOAD_DELAY_SECS: f64 = 1800.0;
/// `MIN_UNLOAD_DELAY`: an expired copy left empty unloads at once.
const MIN_UNLOAD_DELAY_SECS: f64 = 1.0;

/// World maps with local terrain and promoted TDB1210 content, plus the existing
/// Eastern Kingdoms, Kalimdor and Zephras realms. Other world maps remain refused.
/// See docs/world-population-counts.md for source/terrain/exclusion evidence.
pub const HOSTED_CONTINENTS: [u32; 46] = [
    0, 1, 369, 530, 571, 609, 646, 648, 654, 730, 732, 860, 861, 870, 974, 1064, 1116, 1220, 1265,
    1464, 1479, 1481, 1502, 1642, 1643, 1669, 1860, 1865, 2081, 2127, 2175, 2222, 2241, 2268, 2297,
    2374, 2444, 2454, 2552, 2570, 2601, 2694, 2738, 2771, 2785, 2991,
];

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
    pub raid_difficulty: super::RaidDifficulty,
    pub raid_group: bool,
}

impl Default for InstanceEntrant {
    fn default() -> Self {
        Self {
            character_id: 0,
            group: None,
            leader: 0,
            difficulty_id: 1,
            raid_difficulty: super::RaidDifficulty::default(),
            raid_group: false,
        }
    }
}

#[derive(Debug)]
struct InstanceCopy {
    difficulty: MapDifficulty,
    /// Admission is the match roster, independent of dungeon group/lock ownership.
    battleground: bool,
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
    battleground_by_character: HashMap<u64, InMap>,
    housing_copies: HashMap<(u32, u64), InMap>,
    housing_content_pending: HashSet<InMap>,
    housing_by_character: HashMap<u64, InMap>,
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
            battleground_by_character: HashMap::new(),
            housing_copies: HashMap::new(),
            housing_content_pending: HashSet::new(),
            housing_by_character: HashMap::new(),
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

    /// A companion group formed after solo admission inherits that owned copy.
    pub(crate) fn bind_group_instance(
        &mut self,
        group: GroupId,
        owner: u64,
        map: InMap,
    ) -> Result<(), String> {
        let copy = self
            .copies
            .get(&map)
            .ok_or("admitted follower copy is absent")?;
        if copy.owner != owner {
            return Err("admitted follower copy belongs to another character".into());
        }
        self.recent_by_group
            .insert((group, map.map_id), (owner, map.instance_id));
        Ok(())
    }

    /// The difficulty `entrant` enters `map_id` at, `None` when the map offers nothing
    /// it downscales to.
    fn entry_difficulty<'a>(
        store: &'a MapStore,
        map_id: u32,
        entrant: InstanceEntrant,
    ) -> Option<&'a MapDifficulty> {
        let requested = if store.map(map_id)?.instance_type == InstanceType::Raid {
            let default = store.default_map_difficulty(map_id)?;
            if store
                .difficulty_entries
                .get(&default.difficulty_id)?
                .is_legacy()
            {
                entrant.raid_difficulty.legacy
            } else {
                entrant.raid_difficulty.primary
            }
        } else {
            entrant.difficulty_id
        };
        store.downscaled_map_difficulty(map_id, requested)
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
        if matches!(
            store.map(map_id)?.instance_type,
            InstanceType::Battleground | InstanceType::Arena
        ) {
            return self.find_battleground_copy(map_id, entrant.character_id);
        }
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

    fn find_battleground_copy(&self, map_id: u32, character_id: u64) -> Option<InMap> {
        self.battleground_by_character
            .get(&character_id)
            .copied()
            .filter(|map| map.map_id == map_id && self.copies.contains_key(map))
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
            InstanceType::Raid if entry.expansion_id >= 11 && !entrant.raid_group => {
                Err(TransferAbortReason::NeedGroup)
            }
            InstanceType::Party | InstanceType::Raid => {
                self.create_dungeon(store, map_id, entrant, now)
            }
            InstanceType::Battleground | InstanceType::Arena => self
                .find_battleground_copy(map_id, entrant.character_id)
                .map(|map| ResolvedMap {
                    map,
                    created: false,
                })
                .ok_or(TransferAbortReason::MapNotAllowed),
            InstanceType::House | InstanceType::Neighborhood => {
                self.create_authorized_housing_map(entrant.character_id, map_id)
            }
            _ => Err(TransferAbortReason::MapNotAllowed),
        }
    }

    /// Create one match-owned map using its authored difficulty0 and team capacity.
    /// No dungeon locks, group binding or invented reservation-expiry timer.
    pub fn create_battleground_map(
        &mut self,
        store: &MapStore,
        map_id: u32,
        participants: &[u64],
        capacity: u32,
        now: Now,
    ) -> Result<InMap, TransferAbortReason> {
        if store.map(map_id).is_none_or(|entry| {
            !matches!(
                entry.instance_type,
                InstanceType::Battleground | InstanceType::Arena
            )
        }) || participants.is_empty()
            || participants.len() > capacity as usize
            || participants
                .iter()
                .any(|id| self.battleground_by_character.contains_key(id))
        {
            return Err(TransferAbortReason::MapNotAllowed);
        }
        let difficulty = store
            .map_difficulty(map_id, 0)
            .ok_or(TransferAbortReason::Difficulty(0))?;
        let owner = participants[0];
        let entrant = InstanceEntrant {
            character_id: owner,
            leader: owner,
            group: None,
            difficulty_id: 0,
            raid_difficulty: Default::default(),
            raid_group: false,
        };
        let map = InMap {
            map_id,
            instance_id: self.generate_instance_id(),
        };
        self.insert_copy(store, map, difficulty, entrant, owner, now);
        let copy = self.copies.get_mut(&map).expect("created copy");
        copy.battleground = true;
        copy.max_players = capacity;
        copy.unload_at = None;
        for character_id in participants {
            self.battleground_by_character.insert(*character_id, map);
        }
        Ok(map)
    }

    /// Reserve a whole incoming roster on an existing match-owned copy before invitation.
    /// Existing reservations include outstanding invitations and offline accepted members.
    pub fn reserve_battleground_players(
        &mut self,
        map: InMap,
        participants: &[u64],
    ) -> Result<(), TransferAbortReason> {
        let copy = self
            .copies
            .get(&map)
            .ok_or(TransferAbortReason::MapNotAllowed)?;
        let reserved = self
            .battleground_by_character
            .values()
            .filter(|&&assigned| assigned == map)
            .count();
        let conflicts = participants
            .iter()
            .any(|id| self.battleground_by_character.contains_key(id));
        if !copy.battleground
            || conflicts
            || reserved + participants.len() > copy.max_players as usize
        {
            return Err(TransferAbortReason::MapNotAllowed);
        }
        for &character in participants {
            self.battleground_by_character.insert(character, map);
        }
        Ok(())
    }

    pub fn unassign_battleground_player(&mut self, map: InMap, character_id: u64) {
        if self.battleground_by_character.get(&character_id) == Some(&map) {
            self.battleground_by_character.remove(&character_id);
        }
    }

    /// Explicit match close, not a fabricated battleground idle timer.
    pub fn close_battleground_map(&mut self, map: InMap) -> bool {
        let empty_match = self
            .copies
            .get(&map)
            .is_some_and(|copy| copy.battleground && copy.players.is_empty());
        let unassigned = self
            .battleground_by_character
            .values()
            .all(|reserved| *reserved != map);
        if empty_match && unassigned {
            self.copies.remove(&map);
            return true;
        }
        false
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
                battleground: false,
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
    /// Font activation converts the fresh Mythic copy into its authored Keystone difficulty.
    pub fn activate_challenge_difficulty(
        &mut self,
        map: InMap,
        difficulty: MapDifficulty,
    ) -> Result<(), String> {
        if self.completed_encounters(map) != 0
            || difficulty.difficulty_id != 8
            || difficulty.map_id != map.map_id
        {
            return Err("Challenge requires a fresh copy and authored difficulty8".into());
        }
        let copy = self
            .copies
            .get_mut(&map)
            .ok_or("Challenge copy is missing")?;
        if !matches!(copy.difficulty.difficulty_id, 23 | 8) {
            return Err("Challenge requires Mythic difficulty".into());
        }
        copy.difficulty = difficulty;
        Ok(())
    }

    /// The allocated copy's admission capacity, not current player occupancy.
    pub(crate) fn copy_capacity(&self, map: InMap) -> Option<u32> {
        self.copies.get(&map).map(|copy| copy.max_players)
    }

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
        if self.housing_property(map).is_some() {
            return (self.housing_by_character.get(&entrant.character_id) != Some(&map))
                .then_some(TransferAbortReason::MapNotAllowed);
        }
        let copy = self.copies.get(&map)?;
        if copy.battleground
            && self.battleground_by_character.get(&entrant.character_id) != Some(&map)
        {
            return Some(TransferAbortReason::MapNotAllowed);
        }
        if copy.players.len() as u32 >= copy.max_players {
            return Some(TransferAbortReason::MaxPlayers);
        }
        if copy.lock_owner.is_some()
            && let Some(reason) = self.locks.can_join(
                entrant.character_id,
                &copy.difficulty,
                map.instance_id,
                unix,
                copy.initial_data.completed_encounters,
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
    /// battlegrounds and scenarios are not hosted. Current-expansion raids require a raid group.
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
            InstanceType::Raid if entry.expansion_id >= 11 && !entrant.raid_group => {
                Some(TransferAbortReason::NeedGroup)
            }
            InstanceType::Party | InstanceType::Raid => {
                let Some(difficulty) = Self::entry_difficulty(store, map_id, entrant) else {
                    return Some(TransferAbortReason::Difficulty(entrant.difficulty_id));
                };
                if let Some(condition) = store.unmet_condition(difficulty.id, level) {
                    return Some(TransferAbortReason::Condition(condition.failure()));
                }
                self.find_instance(store, map_id, entrant, unix)
                    .and_then(|map| self.cannot_enter(map, entrant, unix))
            }
            InstanceType::Battleground | InstanceType::Arena => {
                match self.find_battleground_copy(map_id, entrant.character_id) {
                    Some(map) => self.cannot_enter(map, entrant, unix),
                    _ => Some(TransferAbortReason::MapNotAllowed),
                }
            }
            InstanceType::House | InstanceType::Neighborhood => self
                .authorized_housing_map(entrant.character_id, map_id)
                .map_or(Some(TransferAbortReason::MapNotAllowed), |map| {
                    self.cannot_enter(map, entrant, unix)
                }),
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
            if copy.battleground {
                return;
            }
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
            self.battleground_by_character
                .retain(|_, reserved| reserved != map);
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
            let at = copy
                .lock_owner
                .and_then(|owner| self.locks.lock(owner, copy.difficulty.lock_key()))
                .filter(|lock| lock.extended)
                .map_or(at, |lock| lock.effective_expiry(&copy.difficulty, now.unix));
            if at > now.unix {
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
