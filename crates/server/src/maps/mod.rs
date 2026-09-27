//! Maps and map copies (docs/specs/instances.md).
//!
//! Every positioned world entity carries [`InMap`]: the map and copy it is in
//! (TrinityCore `WorldObject::GetMapId` / `GetInstanceId`). Visibility, aggro and the
//! other proximity rules only relate entities of one `InMap`. Continents (maps 0 and 1)
//! are copy 0; each dungeon copy has its own instance id (`instances`). Players move
//! between maps through [`teleport_to`] (`transfer`) and area trigger teleports
//! (`area_triggers`).

mod area_triggers;
pub(crate) mod graveyard;
pub(crate) mod instances;
mod login;
pub(crate) mod persistence;
pub(crate) mod store;
pub(crate) mod transfer;

use bevy::prelude::*;
use shared::components::{Player, Position};

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
            .add_observer(track_player_added_to_map)
            .add_observer(track_player_removed_from_map)
            .add_systems(Update, transfer::receive_world_port_acks)
            .add_systems(
                FixedUpdate,
                (
                    area_triggers::detect_area_trigger_teleports
                        .after(crate::networking::apply_terrain_gravity),
                    unload_expired_instances,
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
    mut instances: ResMut<InstanceManager>,
) {
    if let Ok(&map) = players.get(trigger.entity) {
        instances.remove_player(map, trigger.entity, time.elapsed_secs_f64());
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
        crate::scripted_ai::instance_script::unload_instance_scripts(world, &expired);
    });
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
