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
    housing: Option<Res<'w, crate::housing::HousingAvailability>>,
    database: Option<Res<'w, crate::persistence::GameDatabase>>,
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
        self.restore_housing_admission(map_id, character_id)?;
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

    fn restore_housing_admission(&mut self, map_id: u32, character_id: u64) -> Option<()> {
        if self
            .store
            .map(map_id)
            .is_some_and(|entry| entry.instance_type == super::store::InstanceType::House)
        {
            let database = self.database.as_ref()?;
            let location = match crate::persistence::housing_store::load_house_location(
                &database.db,
                character_id,
                self.groups.as_deref(),
            ) {
                Ok(location) => location,
                Err(message) => {
                    error!("Restore character {character_id} visited house: {message}");
                    return None;
                }
            };
            self.instances
                .authorize_housing_entry(&self.store, map_id, location.house_id, character_id)
                .ok()?;
            return Some(());
        }
        let housing = self
            .store
            .map(map_id)
            .is_some_and(|entry| entry.instance_type == super::store::InstanceType::Neighborhood);
        if !housing {
            return Some(());
        }
        let database = self.database.as_ref()?;
        let location = match crate::persistence::housing_store::load_neighborhood_location(
            &database.db,
            character_id,
        ) {
            Ok(Some(location)) if location.map_id == map_id => location,
            Ok(_) => {
                warn!(
                    "Character {character_id} has no persisted admission to neighborhood map {map_id}"
                );
                return None;
            }
            Err(message) => {
                error!("Restore character {character_id} housing admission: {message}");
                return None;
            }
        };
        self.instances
            .authorize_housing_entry(
                &self.store,
                map_id,
                u64::from(location.neighborhood_id),
                character_id,
            )
            .ok()?;
        Some(())
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
        super::transfer::queue_map_content(
            commands,
            self.world_data.as_deref(),
            self.objects.as_deref(),
            content,
        );
        if let Some(housing) = &self.housing {
            crate::housing::spawn_neighborhood_objects(commands, housing, resolved.map);
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
