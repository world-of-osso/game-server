//! World game objects from world.db `content_gameobject` (docs/wiki/systems/game-objects.md).
//!
//! Guild Vaults (`GAMEOBJECT_TYPE_GUILD_BANK`), Mailboxes
//! (`GAMEOBJECT_TYPE_MAILBOX`), meeting stones (`GAMEOBJECT_TYPE_MEETINGSTONE`), chairs
//! (`GAMEOBJECT_TYPE_CHAIR`, `chair`) and the
//! non-usable decoration (`GAMEOBJECT_TYPE_GENERIC`) and spell foci
//! (`GAMEOBJECT_TYPE_SPELL_FOCUS`: fires, bonfires) spawn for now, into each copy of their
//! map (`maps`). Instance copies also render every model-bearing authored object
//! type, including doors and chests; this does not add their interaction/scripts.
//! Summoning portals spawn at runtime (`meeting_stone::ritual`). A spawned object
//! replicates `Position`, `Rotation` and `GameObjectInfo`, and keeps its model box
//! (Retail `GameObjectDisplayInfo.GeoBox`) for the interaction reach check.
//!
//! Reach is AzerothCore `GameObject::IsAtInteractDistance`: the player must be inside
//! the object's rotated model box grown by the interaction distance on every side.
//! The distance is the blizzlike 5 yd (AzerothCore
//! `GameObject::GetInteractionDistance` notes 5.0 as blizzlike and uses 10.0).
//! Static objects stay outside grid activation; summoned lifetimes and bomb fuses tick with spells.

pub(crate) mod collision;
pub(crate) mod cooking_focus;
mod cooking_traps;
pub(crate) mod identity_conditions;
pub(crate) mod locks;
pub(crate) mod quest_object;
pub(crate) mod seaforium_bombs;
pub(crate) mod spellcaster;
pub(crate) mod summoned;

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use lightyear::prelude::*;
use rusqlite::{Connection, Row};
use shared::components::{Position, Rotation};
use shared::protocol::{
    GAMEOBJECT_TYPE_CHAIR, GAMEOBJECT_TYPE_GENERIC, GAMEOBJECT_TYPE_GUILD_BANK,
    GAMEOBJECT_TYPE_MAILBOX, GAMEOBJECT_TYPE_MEETINGSTONE, GAMEOBJECT_TYPE_SPELL_FOCUS,
    GameObjectInfo,
};

use crate::interest::NetworkVisibility;
use crate::terrain_height::{bevy_to_wow, wow_to_bevy};

pub(crate) const GAMEOBJECT_TYPE_BUTTON: u8 = 1;
pub(crate) const GAMEOBJECT_TYPE_CHEST: u8 = 3;
pub(crate) const GAMEOBJECT_TYPE_TRAP: u8 = 6;
pub(crate) const GAMEOBJECT_TYPE_GOOBER: u8 = 10;

/// Retail interaction distance around the model box (yd).
pub(crate) const GAME_OBJECT_INTERACT_DISTANCE: f32 = 5.0;

const SPAWN_QUERY: &str = "SELECT g.guid, g.id, g.position_x, g.position_y, g.position_z, \
     g.orientation, t.type, t.displayId, t.name, t.size, \
     d.GeoBox_0, d.GeoBox_1, d.GeoBox_2, d.GeoBox_3, d.GeoBox_4, d.GeoBox_5, g.map, \
     g.spawnDifficulties, t.Data0, t.Data1, t.Data3, t.Data5, t.Data6, g.spawntimesecs, g.state, \
     t.ScriptName, t.Data10, t.Data23 \
     FROM content_gameobject g \
     JOIN content_gameobject_template t ON t.entry = g.id \
     JOIN game_object_display_info d ON d.ID = t.displayId \
     WHERE g.spawnDifficulties <> '0' OR t.type IN (?1, ?2, ?3, ?4, ?5, ?6, 3, 10, 50) \
     OR (g.map IN (SELECT map_id FROM arena_map) AND t.type IN (0,6,35))";

/// Battleground objective spawns use difficulty 0; the shared rules name each one.
fn battleground_objective_clause() -> String {
    shared::battleground::all_templates()
        .iter()
        .flat_map(|template| {
            shared::battleground::objective_object_entries(template.map_id)
                .into_iter()
                .map(move |entry| format!(" OR (g.map = {} AND g.id = {entry})", template.map_id))
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectSpawn {
    pub guid: u32,
    pub map_id: u32,
    pub info: GameObjectInfo,
    /// WoW coordinates.
    pub position: Vec3,
    pub orientation: f32,
    /// Authored gameobject.state: GO_STATE_READY1 has model collision.
    pub initial_state: u8,
    pub bounds: GameObjectBounds,
    /// `gameobject.spawnDifficulties`: the copy difficulties it spawns in (0 on a
    /// continent).
    pub spawn_difficulties: Vec<u32>,
    /// A `GAMEOBJECT_TYPE_CHAIR`'s seats.
    pub chair: Option<crate::chair::Chair>,
    pub quest_object: Option<quest_object::QuestObject>,
    /// `gameobject_template.ScriptName`; empty when none.
    pub script_name: String,
}

/// A ported `GameObjectAI` registered for the template's ScriptName.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct GameObjectScript(pub String);

/// `GO_FLAG_IN_USE` with `GO_STATE_ACTIVE` after `UseDoorOrButton`. The client refuses
/// to use an in-use object, so the server applies that gate itself.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectInUse;

/// Authoritative `GameObjectData::State`. Native animation selection is not yet
/// driven by this field; model collision is replicated separately.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GameObjectState(pub u8);

pub(crate) const GO_STATE_ACTIVE: u8 = 0;
pub(crate) const GO_STATE_DESTROYED: u8 = 2;
pub(crate) const GO_STATE_READY: u8 = 1;

/// TC `GameObject::SetGoState`: store the state and enable model collision only
/// in READY. No scripted GO AI/state-type callbacks are bound to the liquid pool.
pub(crate) fn set_go_state(world: &mut World, object: Entity, state: u8) {
    world.entity_mut(object).insert((
        GameObjectState(state),
        shared::protocol::GameObjectCollision(state == GO_STATE_READY),
    ));
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

fn spawn_from_row(row: &Row, locks: &locks::LockDb) -> rusqlite::Result<GameObjectSpawn> {
    let scale: f64 = row.get(9)?;
    let geo = |index: usize| -> rusqlite::Result<f32> { Ok(row.get::<_, f64>(index)? as f32) };
    Ok(GameObjectSpawn {
        guid: row.get(0)?,
        map_id: row.get(16)?,
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
        initial_state: row.get(24)?,
        bounds: GameObjectBounds {
            min: Vec3::new(geo(10)?, geo(11)?, geo(12)?),
            max: Vec3::new(geo(13)?, geo(14)?, geo(15)?),
            scale: scale as f32,
            orientation: row.get::<_, f64>(5)? as f32,
        },
        chair: (row.get::<_, u8>(6)? == GAMEOBJECT_TYPE_CHAIR)
            .then(|| -> rusqlite::Result<_> {
                Ok(crate::chair::Chair::new(row.get(18)?, row.get(19)?))
            })
            .transpose()?,
        quest_object: quest_object::QuestObject::from_row(row, locks)?,
        script_name: row.get::<_, Option<String>>(25)?.unwrap_or_default(),
        spawn_difficulties: row
            .get::<_, String>(17)?
            .split(',')
            .map(|token| {
                token
                    .trim()
                    .parse()
                    .unwrap_or_else(|_| panic!("bad spawnDifficulties in gameobject {}", token))
            })
            .collect(),
    })
}

pub fn load_game_object_spawns(conn: &Connection) -> rusqlite::Result<GameObjectSpawns> {
    let locks = locks::load_locks(conn.path().ok_or(rusqlite::Error::InvalidQuery)?)?;
    let query = format!(
        "{SPAWN_QUERY}{} ORDER BY g.guid",
        battleground_objective_clause()
    );
    let mut statement = conn.prepare(&query)?;
    let rows = statement.query_map(
        rusqlite::params![
            GAMEOBJECT_TYPE_GUILD_BANK,
            GAMEOBJECT_TYPE_MAILBOX,
            GAMEOBJECT_TYPE_MEETINGSTONE,
            GAMEOBJECT_TYPE_GENERIC,
            GAMEOBJECT_TYPE_SPELL_FOCUS,
            GAMEOBJECT_TYPE_CHAIR,
        ],
        |row| spawn_from_row(row, &locks),
    )?;
    Ok(GameObjectSpawns(rows.collect::<rusqlite::Result<_>>()?))
}

/// Server-space (`wow_to_bevy`) position of a WoW coordinate.
pub(crate) fn bevy_position(wow: Vec3) -> Position {
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
) -> (
    Position,
    Rotation,
    GameObjectInfo,
    GameObjectBounds,
    shared::protocol::GameObjectCollision,
    GameObjectState,
) {
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
        shared::protocol::GameObjectCollision(spawn.initial_state == GO_STATE_READY),
        GameObjectState(spawn.initial_state),
    )
}

fn spawn_continent_game_objects(mut commands: Commands, spawns: Res<GameObjectSpawns>) {
    for map_id in crate::maps::HOSTED_CONTINENTS {
        let count = spawn_map_game_objects(
            &mut commands,
            &spawns,
            crate::maps::InMap::continent(map_id),
            0,
            None,
        );
        info!("Spawned {count} game objects on map {map_id}");
    }
}

/// Spawn the game objects of `map`'s map spawned at its `difficulty_id` into that copy;
/// members of condition-driven spawn groups are left to their battleground runtime.
pub(crate) fn spawn_map_game_objects(
    commands: &mut Commands,
    spawns: &GameObjectSpawns,
    map: crate::maps::InMap,
    difficulty_id: u32,
    spawn_groups: Option<&crate::spawn_groups::SpawnGroups>,
) -> usize {
    let population = crate::followers::population_difficulty(difficulty_id);
    let spawns = spawns.0.iter().filter(|spawn| {
        spawn.map_id == map.map_id
            && spawn.spawn_difficulties.contains(&population)
            && !spawn_groups.is_some_and(|groups| groups.is_object_member(spawn.guid))
    });
    let mut count = 0;
    for spawn in spawns {
        if spawn.info.go_type == shared::protocol::GAMEOBJECT_TYPE_GATHERING_NODE {
            let spawn = spawn.clone();
            commands.queue(move |world: &mut World| {
                crate::gathering::spawn_selected_node(world, spawn, map)
            });
            count += 1;
            continue;
        }
        spawn_game_object(commands, spawn, map);
        count += 1;
    }
    count
}

/// One replicated non-gathering game object of `spawn` in `map`'s copy.
pub(crate) fn spawn_game_object(
    commands: &mut Commands,
    spawn: &GameObjectSpawn,
    map: crate::maps::InMap,
) -> Entity {
    let mut object = commands.spawn((
        game_object_bundle(spawn),
        shared::protocol::GameObjectVisible(true),
        map,
        Replicate::to_clients(NetworkTarget::All),
        NetworkVisibility::default(),
    ));
    if let Some(quest_object) = &spawn.quest_object {
        object.insert(quest_object.clone());
    }
    if let Some(chair) = &spawn.chair {
        object.insert(chair.clone());
    }
    if crate::scripted_ai::is_game_object_script(&spawn.script_name) {
        object.insert(GameObjectScript(spawn.script_name.clone()));
    }
    object.id()
}

pub struct GameObjectPlugin;

impl Plugin for GameObjectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameObjectSpawns>()
            .add_plugins((
                crate::gathering::GatheringPlugin,
                collision::CollisionIndexPlugin,
            ))
            .add_systems(
                FixedUpdate,
                collision::sync_live_objects
                    .before(crate::networking::process_player_inputs)
                    .before(crate::networking::apply_terrain_gravity)
                    .before(crate::knockback::update_knockbacks),
            )
            .add_systems(
                PostStartup,
                spawn_continent_game_objects.after(crate::zones::spawn_continent_creatures),
            );
    }
}

#[cfg(test)]
#[path = "game_objects_tests.rs"]
mod tests;
