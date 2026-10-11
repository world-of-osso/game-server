//! The map copy a character enters the world on (TrinityCore `Player::LoadFromDB`,
//! Player.cpp:18480-18545).
//!
//! The saved map's copy is created for the player (`MapManager::CreateMap`). A dungeon
//! that refuses it, or one that cannot be created, sends it to the dungeon's go-back
//! trigger target (`GetGoBackTrigger`); a dungeon copy created anew (the one it logged
//! out in has reset) puts it at the dungeon's entrance (`GetMapEntranceTrigger`).

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use shared::components::Position;

use super::instances::{InstanceEntrant, Now, ResolvedMap};
use super::{InMap, InstanceManager, MapStore, WorldLocation};
use crate::game_objects::GameObjectSpawns;
use crate::group::GroupRegistry;
use crate::world_data::WorldData;

#[derive(SystemParam)]
pub(crate) struct LoginMaps<'w> {
    store: Res<'w, MapStore>,
    instances: ResMut<'w, InstanceManager>,
    groups: Option<Res<'w, GroupRegistry>>,
    time: Res<'w, Time>,
    clock: Option<Res<'w, super::ServerClock>>,
    world_data: Option<Res<'w, WorldData>>,
    objects: Option<Res<'w, GameObjectSpawns>>,
}

/// Where a character enters the world.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LoginPlace {
    pub map: InMap,
    pub position: Position,
    /// The map's lowercase `Map.db2` Directory, for `LoadTerrain`.
    pub directory: String,
}

impl LoginMaps<'_> {
    /// The copy and position for `character_id` at dungeon difficulty `difficulty_id`,
    /// saved on `map_id` at `position`; `None` when neither the saved map nor its
    /// go-back location can take the player.
    pub fn resolve(
        &mut self,
        commands: &mut Commands,
        character_id: u64,
        difficulty_id: u32,
        raid_difficulty: super::RaidDifficulty,
        map_id: u32,
        position: Position,
    ) -> Option<LoginPlace> {
        let entrant: InstanceEntrant = super::transfer::entrant_for(
            self.groups.as_deref(),
            character_id,
            difficulty_id,
            raid_difficulty,
        );
        match self.enter(map_id, entrant) {
            Some(resolved) => {
                let position = match (resolved.created, self.is_dungeon(map_id)) {
                    (true, true) => self
                        .store
                        .entrance_location(map_id)
                        .map_or(position, |entrance| entrance.bevy_position()),
                    _ => position,
                };
                self.spawn_if_created(commands, resolved);
                self.place(resolved.map, position)
            }
            None => {
                let back: WorldLocation = self.store.go_back_location(map_id)?;
                warn!(
                    "Character {character_id} cannot enter map {map_id}; moved to map {}",
                    back.map_id
                );
                let resolved = self.enter(back.map_id, entrant)?;
                self.spawn_if_created(commands, resolved);
                self.place(resolved.map, back.bevy_position())
            }
        }
    }

    fn is_dungeon(&self, map_id: u32) -> bool {
        self.store
            .map(map_id)
            .is_some_and(|entry| entry.is_dungeon())
    }

    fn enter(&mut self, map_id: u32, entrant: InstanceEntrant) -> Option<ResolvedMap> {
        let now = Now {
            elapsed: self.time.elapsed_secs_f64(),
            unix: self.clock.as_deref().copied().unwrap_or_default().unix(),
        };
        let resolved = self
            .instances
            .create_map(&self.store, map_id, entrant, now)
            .ok()?;
        self.instances
            .cannot_enter(resolved.map, entrant, now.unix)
            .is_none()
            .then_some(resolved)
    }

    fn spawn_if_created(&self, commands: &mut Commands, resolved: ResolvedMap) {
        if !resolved.created {
            return;
        }
        let content = super::transfer::map_content(&self.instances, &self.store, resolved.map);
        if let Some(objects) = &self.objects {
            crate::game_objects::spawn_map_game_objects(
                commands,
                objects,
                resolved.map,
                content.difficulty_id,
            );
        }
        if let Some(world_data) = &self.world_data {
            crate::zones::spawn_map_creatures(commands, world_data, content, true);
        }
    }

    fn place(&self, map: InMap, position: Position) -> Option<LoginPlace> {
        let Some(entry) = self.store.map(map.map_id) else {
            error!("Map {} has no Map.db2 entry", map.map_id);
            return None;
        };
        Some(LoginPlace {
            map,
            position,
            directory: entry.directory.clone(),
        })
    }
}
