//! World game objects from world.db `content_gameobject` (docs/wiki/systems/game-objects.md).
//!
//! Guild Vaults (`GAMEOBJECT_TYPE_GUILD_BANK`) and Mailboxes
//! (`GAMEOBJECT_TYPE_MAILBOX`) on map 0 spawn for now; other types and maps become
//! spawns by widening `SPAWN_QUERY`. A spawned object
//! replicates `Position`, `Rotation` and `GameObjectInfo`, and keeps its model box
//! (Retail `GameObjectDisplayInfo.GeoBox`) for the interaction reach check.
//!
//! Reach is AzerothCore `GameObject::IsAtInteractDistance`: the player must be inside
//! the object's rotated model box grown by the interaction distance on every side.
//! The distance is the blizzlike 5 yd (AzerothCore
//! `GameObject::GetInteractionDistance` notes 5.0 as blizzlike and uses 10.0).
//! Game objects have no per-tick systems, so grid activation does not apply.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use lightyear::prelude::*;
use rusqlite::{Connection, Row};
use shared::components::{Position, Rotation};
use shared::protocol::{GAMEOBJECT_TYPE_GUILD_BANK, GAMEOBJECT_TYPE_MAILBOX, GameObjectInfo};

use crate::interest::NetworkVisibility;
use crate::terrain_height::{bevy_to_wow, wow_to_bevy};

/// Retail interaction distance around the model box (yd).
pub(crate) const GAME_OBJECT_INTERACT_DISTANCE: f32 = 5.0;

const SPAWN_QUERY: &str = "SELECT g.guid, g.id, g.position_x, g.position_y, g.position_z, \
     g.orientation, t.type, t.displayId, t.name, t.size, \
     d.GeoBox_0, d.GeoBox_1, d.GeoBox_2, d.GeoBox_3, d.GeoBox_4, d.GeoBox_5 \
     FROM content_gameobject g \
     JOIN content_gameobject_template t ON t.entry = g.id \
     JOIN game_object_display_info d ON d.ID = t.displayId \
     WHERE g.map = 0 AND t.type IN (?1, ?2) ORDER BY g.guid";

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectSpawn {
    pub guid: u32,
    pub info: GameObjectInfo,
    /// WoW coordinates.
    pub position: Vec3,
    pub orientation: f32,
    pub bounds: GameObjectBounds,
}

/// Model box in object space (WoW axes, unscaled) plus the spawn's facing.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct GameObjectBounds {
    pub min: Vec3,
    pub max: Vec3,
    pub scale: f32,
    pub orientation: f32,
}

impl GameObjectBounds {
    /// Whether `point` (WoW coordinates) is within `radius` of the box of an object
    /// standing at `origin`.
    pub fn reaches(&self, origin: Vec3, point: Vec3, radius: f32) -> bool {
        let local = Quat::from_rotation_z(-self.orientation) * (point - origin);
        let min = self.min * self.scale - Vec3::splat(radius);
        let max = self.max * self.scale + Vec3::splat(radius);
        local.cmpge(min).all() && local.cmple(max).all()
    }
}

#[derive(Resource, Debug, Default, Clone)]
pub struct GameObjectSpawns(pub Vec<GameObjectSpawn>);

fn spawn_from_row(row: &Row) -> rusqlite::Result<GameObjectSpawn> {
    let scale: f64 = row.get(9)?;
    let geo = |index: usize| -> rusqlite::Result<f32> { Ok(row.get::<_, f64>(index)? as f32) };
    Ok(GameObjectSpawn {
        guid: row.get(0)?,
        info: GameObjectInfo {
            entry: row.get(1)?,
            go_type: row.get(6)?,
            display_id: row.get(7)?,
            name: row.get(8)?,
            scale: scale as f32,
        },
        position: Vec3::new(
            row.get::<_, f64>(2)? as f32,
            row.get::<_, f64>(3)? as f32,
            row.get::<_, f64>(4)? as f32,
        ),
        orientation: row.get::<_, f64>(5)? as f32,
        bounds: GameObjectBounds {
            min: Vec3::new(geo(10)?, geo(11)?, geo(12)?),
            max: Vec3::new(geo(13)?, geo(14)?, geo(15)?),
            scale: scale as f32,
            orientation: row.get::<_, f64>(5)? as f32,
        },
    })
}

pub fn load_game_object_spawns(conn: &Connection) -> rusqlite::Result<GameObjectSpawns> {
    let mut statement = conn.prepare(SPAWN_QUERY)?;
    let rows = statement.query_map(
        [GAMEOBJECT_TYPE_GUILD_BANK, GAMEOBJECT_TYPE_MAILBOX],
        spawn_from_row,
    )?;
    Ok(GameObjectSpawns(rows.collect::<rusqlite::Result<_>>()?))
}

/// Server-space (`wow_to_bevy`) position of a WoW coordinate.
fn bevy_position(wow: Vec3) -> Position {
    let (x, y, z) = wow_to_bevy(wow.x, wow.y, wow.z);
    Position { x, y, z }
}

/// WoW coordinates of a server `Position`.
pub(crate) fn wow_position(position: &Position) -> Vec3 {
    let (x, y, z) = bevy_to_wow(position.x, position.y, position.z);
    Vec3::new(x, y, z)
}

pub(crate) fn game_object_bundle(
    spawn: &GameObjectSpawn,
) -> (Position, Rotation, GameObjectInfo, GameObjectBounds) {
    (
        bevy_position(spawn.position),
        // Same facing convention as creature spawns (`zones::creature_bundle`).
        Rotation {
            x: 0.0,
            y: spawn.orientation + FRAC_PI_2,
            z: 0.0,
        },
        spawn.info.clone(),
        spawn.bounds,
    )
}

fn spawn_game_objects(mut commands: Commands, spawns: Res<GameObjectSpawns>) {
    for spawn in &spawns.0 {
        commands.spawn((
            game_object_bundle(spawn),
            Replicate::to_clients(NetworkTarget::All),
            NetworkVisibility::default(),
        ));
    }
    info!("Spawned {} game objects on map 0", spawns.0.len());
}

pub struct GameObjectPlugin;

impl Plugin for GameObjectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameObjectSpawns>()
            .add_systems(PostStartup, spawn_game_objects);
    }
}

#[cfg(test)]
#[path = "game_objects_tests.rs"]
mod tests;
