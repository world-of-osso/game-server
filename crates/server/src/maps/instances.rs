//! Instance copies of dungeon maps (TrinityCore `MapManager::CreateMap`,
//! `FindInstanceIdForPlayer`, `InstanceMap::CannotEnter`, `InstanceMap::RemovePlayerFromMap`).
//!
//! A normal dungeon (a `MapDifficulty` without reset schedule) has no instance lock:
//! a group enters the instance it created last (`Group::GetRecentInstanceId`), a
//! player outside a group the one it created last (`Player::GetRecentInstanceId`);
//! with none, or once that copy unloaded, a new copy is created and remembered. An
//! instance with no players unloads after `Instance.UnloadDelay` (30 min), which
//! resets its content: the next entry creates a fresh copy.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use shared::protocol::TransferAbortReason;

use super::InMap;
use super::store::{InstanceType, MapStore};
use crate::group::GroupId;

/// `Instance.UnloadDelay` default (worldserver.conf.dist): 30 minutes.
pub const INSTANCE_UNLOAD_DELAY_SECS: f64 = 1800.0;

/// Maps whose NPCs the client has appearance data for (game-engine
/// data/cache/npc_appearance.sqlite covers maps 0 and 1): the world maps the server
/// hosts, and the maps whose creatures spawn. A dungeon outside it opens empty.
pub const NPC_APPEARANCE_MAPS: [u32; 2] = [0, 1];

/// Who a player enters instances with: its character and its group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceEntrant {
    pub character_id: u64,
    pub group: Option<GroupId>,
}

#[derive(Debug)]
struct InstanceCopy {
    max_players: u32,
    /// `InstanceMap::GetOwningGroup`: the group that created it.
    owning_group: Option<GroupId>,
    players: HashSet<Entity>,
    /// `m_unloadTimer` as an absolute time; `None` while players are inside.
    unload_at: Option<f64>,
}

#[derive(Resource, Debug)]
pub struct InstanceManager {
    next_instance_id: u32,
    copies: HashMap<InMap, InstanceCopy>,
    recent_by_character: HashMap<(u64, u32), u32>,
    recent_by_group: HashMap<(GroupId, u32), u32>,
}

impl Default for InstanceManager {
    fn default() -> Self {
        Self {
            next_instance_id: 1,
            copies: HashMap::new(),
            recent_by_character: HashMap::new(),
            recent_by_group: HashMap::new(),
        }
    }
}

/// A map copy `create_map` resolved, and whether it was just created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedMap {
    pub map: InMap,
    pub created: bool,
}

impl InstanceManager {
    fn recent_instance(&self, map_id: u32, entrant: InstanceEntrant) -> Option<u32> {
        match entrant.group {
            Some(group) => self.recent_by_group.get(&(group, map_id)).copied(),
            None => self
                .recent_by_character
                .get(&(entrant.character_id, map_id))
                .copied(),
        }
    }

    /// `MapManager::FindInstanceIdForPlayer`: the existing copy `entrant` would enter.
    pub fn find_instance(&self, map_id: u32, entrant: InstanceEntrant) -> Option<InMap> {
        let instance_id = self.recent_instance(map_id, entrant)?;
        let map = InMap {
            map_id,
            instance_id,
        };
        self.copies.contains_key(&map).then_some(map)
    }

    /// `MapManager::CreateMap` for a player: the continent itself, or the dungeon copy
    /// `entrant` enters, created when missing.
    pub fn create_map(
        &mut self,
        store: &MapStore,
        map_id: u32,
        entrant: InstanceEntrant,
        now: f64,
    ) -> Result<ResolvedMap, TransferAbortReason> {
        if NPC_APPEARANCE_MAPS.contains(&map_id) {
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
        now: f64,
    ) -> Result<ResolvedMap, TransferAbortReason> {
        let difficulty = store
            .dungeon_difficulty(map_id)
            .ok_or(TransferAbortReason::NotFound)?;
        if difficulty.reset_interval != 0 {
            // Instance locks (dungeons with a reset schedule) are not implemented.
            return Err(TransferAbortReason::MapNotAllowed);
        }
        if let Some(map) = self.find_instance(map_id, entrant) {
            return Ok(ResolvedMap {
                map,
                created: false,
            });
        }
        let instance_id = self
            .recent_instance(map_id, entrant)
            .unwrap_or_else(|| self.generate_instance_id());
        let map = InMap {
            map_id,
            instance_id,
        };
        let max_players = match difficulty.max_players {
            0 => store.map(map_id).map_or(0, |entry| entry.max_players),
            max => max,
        };
        self.copies.insert(
            map,
            InstanceCopy {
                max_players,
                owning_group: entrant.group,
                players: HashSet::new(),
                // A new InstanceMap starts its unload timer (Map.cpp:2878).
                unload_at: Some(now + INSTANCE_UNLOAD_DELAY_SECS),
            },
        );
        match entrant.group {
            Some(group) => self.recent_by_group.insert((group, map_id), instance_id),
            None => self
                .recent_by_character
                .insert((entrant.character_id, map_id), instance_id),
        };
        info!(
            "Created instance {instance_id} of map {map_id} for {:?}",
            entrant
        );
        Ok(ResolvedMap { map, created: true })
    }

    fn generate_instance_id(&mut self) -> u32 {
        let id = self.next_instance_id;
        self.next_instance_id += 1;
        id
    }

    /// `InstanceMap::CannotEnter` for a player outside the copy: full, or owned by a
    /// group the player is not in (both `TRANSFER_ABORT_MAX_PLAYERS`).
    pub fn cannot_enter(
        &self,
        map: InMap,
        entrant: InstanceEntrant,
    ) -> Option<TransferAbortReason> {
        let copy = self.copies.get(&map)?;
        if copy.players.len() as u32 >= copy.max_players {
            return Some(TransferAbortReason::MaxPlayers);
        }
        if copy.owning_group.is_some() && copy.owning_group != entrant.group {
            return Some(TransferAbortReason::MaxPlayers);
        }
        None
    }

    /// `Map::PlayerCannotEnter` (Map.cpp:1807-1852), checked before a far teleport:
    /// the copy `entrant` is bound to must admit it. World maps without NPC data, raids,
    /// battlegrounds and scenarios are not hosted.
    pub fn player_cannot_enter(
        &self,
        store: &MapStore,
        map_id: u32,
        entrant: InstanceEntrant,
    ) -> Option<TransferAbortReason> {
        if NPC_APPEARANCE_MAPS.contains(&map_id) {
            return None;
        }
        let Some(entry) = store.map(map_id) else {
            return Some(TransferAbortReason::MapNotAllowed);
        };
        match entry.instance_type {
            InstanceType::Party => {
                if store.dungeon_difficulty(map_id).is_none() {
                    return Some(TransferAbortReason::NotFound);
                }
                self.find_instance(map_id, entrant)
                    .and_then(|map| self.cannot_enter(map, entrant))
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

    /// `InstanceMap::RemovePlayerFromMap`: the last player out starts the unload timer.
    pub fn remove_player(&mut self, map: InMap, player: Entity, now: f64) {
        if let Some(copy) = self.copies.get_mut(&map)
            && copy.players.remove(&player)
            && copy.players.is_empty()
        {
            copy.unload_at = Some(now + INSTANCE_UNLOAD_DELAY_SECS);
        }
    }

    /// Copies whose unload timer ran out (`Map::CanUnload`), forgotten here; their
    /// entities are despawned by the caller.
    pub fn take_expired(&mut self, now: f64) -> Vec<InMap> {
        let expired: Vec<InMap> = self
            .copies
            .iter()
            .filter(|(_, copy)| copy.unload_at.is_some_and(|at| at <= now))
            .map(|(&map, _)| map)
            .collect();
        for map in &expired {
            self.copies.remove(map);
        }
        expired
    }
}
