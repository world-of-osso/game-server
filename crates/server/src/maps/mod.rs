//! Maps and map copies (docs/specs/instances.md).
//!
//! Every positioned world entity carries [`InMap`]: the map and copy it is in
//! (TrinityCore `WorldObject::GetMapId` / `GetInstanceId`). Visibility, aggro and the
//! other proximity rules only relate entities of one `InMap`. Continents (maps 0 and 1)
//! are copy 0; each dungeon copy has its own instance id (`instances`). Players move
//! between maps through [`teleport_to`] (`transfer`) and area trigger teleports
//! (`area_triggers`).

mod area_triggers;
pub(crate) mod difficulty;
pub(crate) mod graveyard;
pub(crate) mod instances;
pub(crate) mod locks;
mod login;
pub(crate) mod persistence;
pub(crate) mod store;
pub(crate) mod transfer;

use bevy::prelude::*;
use shared::components::{Player, Position};

pub(crate) use difficulty::{DungeonDifficulty, ServerClock};
pub(crate) use instances::{HOSTED_CONTINENTS, InstanceManager, NPC_APPEARANCE_MAPS};
pub(crate) use login::LoginMaps;
pub(crate) use store::MapStore;
pub(crate) use transfer::{MapTransfer, TeleportOptions, teleport_to};

/// The map copy an entity is in.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InMap {
    pub map_id: u32,
    pub instance_id: u32,
}

impl InMap {
    /// A world map (continent): its only copy is instance 0.
    pub const fn continent(map_id: u32) -> Self {
        Self {
            map_id,
            instance_id: 0,
        }
    }
}

/// A point on a map in WoW coordinates, orientation in radians (TrinityCore
/// `WorldLocation`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldLocation {
    pub map_id: u32,
    pub position: Vec3,
    pub orientation: f32,
}

impl WorldLocation {
    /// The server (Bevy-space) position of this location.
    pub fn bevy_position(&self) -> Position {
        let (x, y, z) =
            crate::terrain_height::wow_to_bevy(self.position.x, self.position.y, self.position.z);
        Position { x, y, z }
    }
}

/// The map id a player is saved on: its map, or the destination of the transfer it is
/// in (TrinityCore saves `m_teleport_dest` before the world port ack).
pub(crate) fn saved_map_id(in_map: Option<&InMap>, transfer: Option<&MapTransfer>) -> u32 {
    match (in_map, transfer) {
        (Some(in_map), _) => in_map.map_id,
        (None, Some(transfer)) => transfer.map_id,
        (None, None) => panic!("player is neither on a map nor in a map transfer"),
    }
}

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InstanceManager>()
            .init_resource::<ServerClock>()
            .init_resource::<difficulty::DifficultyChanges>()
            .add_observer(track_player_added_to_map)
            .add_observer(track_player_removed_from_map)
            .add_systems(PostStartup, load_instance_locks)
            .add_systems(
                Update,
                (
                    transfer::receive_world_port_acks,
                    difficulty::receive_set_dungeon_difficulty,
                    difficulty::apply_difficulty_changes,
                    difficulty::send_dungeon_difficulty,
                    difficulty::send_world_server_info,
                    difficulty::receive_request_raid_info,
                    difficulty::receive_reset_instances,
                    difficulty::receive_set_saved_instance_extend,
                    save_instance_locks,
                )
                    .chain(),
            )
            .add_systems(
                FixedUpdate,
                (
                    area_triggers::detect_area_trigger_teleports
                        .after(crate::networking::apply_terrain_gravity),
                    (difficulty::expire_instance_locks, unload_expired_instances).chain(),
                    // complete_transfer runs in Update: the next interest update shows
                    // the arrival its destination.
                    transfer::announce_world_arrivals.after(crate::interest::InterestUpdate),
                ),
            );
    }
}

/// `Map::AddPlayerToMap`: a player entering a dungeon copy holds it loaded.
fn track_player_added_to_map(
    trigger: On<Insert, InMap>,
    players: Query<&InMap, With<Player>>,
    mut instances: ResMut<InstanceManager>,
) {
    if let Ok(&map) = players.get(trigger.entity) {
        instances.add_player(map, trigger.entity);
    }
}

/// `Map::RemovePlayerFromMap`: the map is left by a transfer, logout or despawn.
fn track_player_removed_from_map(
    trigger: On<Discard, InMap>,
    players: Query<&InMap, With<Player>>,
    time: Res<Time>,
    clock: Option<Res<ServerClock>>,
    mut instances: ResMut<InstanceManager>,
) {
    if let Ok(&map) = players.get(trigger.entity) {
        let now = instances::Now {
            elapsed: time.elapsed_secs_f64(),
            unix: clock.map_or_else(|| ServerClock::default().unix(), |clock| clock.unix()),
        };
        instances.remove_player(map, trigger.entity, now);
    }
}

/// `InstanceLockMgr::Load`: the saved locks, their instance ids reserved.
fn load_instance_locks(
    db: Option<Res<crate::persistence::GameDatabase>>,
    mut instances: ResMut<InstanceManager>,
) {
    let Some(db) = db else {
        return;
    };
    let saved = persistence::load_instance_locks(&db.db);
    let count: usize = saved.locks.values().map(|locks| locks.len()).sum();
    *instances = InstanceManager::with_locks(locks::InstanceLocks::from_saved(saved));
    info!("Loaded {count} instance locks");
}

/// Lock changes are written as they happen (TrinityCore commits each in its
/// transaction).
fn save_instance_locks(
    db: Option<Res<crate::persistence::GameDatabase>>,
    mut instances: ResMut<InstanceManager>,
) {
    if !instances.locks_dirty {
        return;
    }
    instances.locks_dirty = false;
    if let Some(db) = db {
        persistence::save_instance_locks(&db.db, &instances.locks.saved());
    }
}

/// `MapManager::Update` → `Map::CanUnload`: an instance empty for the unload delay is
/// removed with everything in it; the next entry creates a fresh copy.
fn unload_expired_instances(
    time: Res<Time>,
    mut instances: ResMut<InstanceManager>,
    entities: Query<(Entity, &InMap)>,
    mut commands: Commands,
) {
    let expired = instances.take_expired(time.elapsed_secs_f64());
    if expired.is_empty() {
        return;
    }
    for (entity, map) in &entities {
        if expired.contains(map) {
            commands.entity(entity).despawn();
        }
    }
    info!("Unloaded instances {expired:?}");
    commands.queue(move |world: &mut World| {
        if let Some(mut pools) = world.get_resource_mut::<crate::spawn_pools::PoolSpawns>() {
            pools.0.retain(|map, _| !expired.contains(map));
        }
        crate::scripted_ai::instance_script::unload_instance_scripts(world, &expired);
    });
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
