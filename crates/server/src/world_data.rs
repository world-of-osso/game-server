use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags, Row, params};
use std::collections::HashMap;
use std::path::Path;

pub struct CreatureTemplate {
    pub entry: u32,
    pub name: String,
    pub min_level: u8,
    pub max_level: u8,
    pub faction: u16,
    pub npc_flags: u32,
    pub speed_walk: f32,
    pub health_modifier: f32,
    /// `rank`: 1 elite, 2 rare elite (Retail `CreatureClassifications`).
    pub rank: u8,
    /// `exp`: health scaling expansion.
    pub expansion: u8,
    /// `ExperienceModifier` (TrinityCore `ModExperience`).
    pub experience_modifier: f32,
    /// Critters and `CREATURE_FLAG_EXTRA_NO_XP` give no experience.
    pub gives_experience: bool,
    pub models: Vec<CreatureModelInfo>,
}

pub struct CreatureModelInfo {
    pub display_id: u32,
}

pub struct CreatureSpawn {
    pub guid: u32,
    pub template_entry: u32,
    pub map: u16,
    pub zone_id: u16,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub wander_distance: f32,
    pub movement_type: u8,
}

pub struct WaypointPath {
    pub points: Vec<WaypointPoint>,
}

pub struct WaypointPoint {
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub delay: u32,
}

#[derive(Resource)]
pub struct WorldData {
    pub templates: HashMap<u32, CreatureTemplate>,
    pub spawns: Vec<CreatureSpawn>,
    pub spawns_by_map: HashMap<u16, Vec<usize>>,
    pub waypoints: HashMap<u32, WaypointPath>,
}

pub fn load_world_data(path: &Path) -> WorldData {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("Failed to open world.db");

    let mut templates = load_templates(&conn);
    load_models(&conn, &mut templates);
    let spawns = load_spawns(&conn);
    let spawns_by_map = build_spawns_by_map(&spawns);
    let waypoints = load_waypoints(&conn);

    info!(
        "WorldData loaded: {} templates, {} spawns, {} maps, {} waypoint paths",
        templates.len(),
        spawns.len(),
        spawns_by_map.len(),
        waypoints.len(),
    );

    WorldData {
        templates,
        spawns,
        spawns_by_map,
        waypoints,
    }
}

const CREATURE_TYPE_CRITTER: i64 = 8;
const CREATURE_FLAG_EXTRA_NO_XP: i64 = 0x40;

fn template_from_row(row: &Row) -> rusqlite::Result<CreatureTemplate> {
    Ok(CreatureTemplate {
        entry: row.get::<_, i64>(0)? as u32,
        min_level: row.get::<_, i64>(1)? as u8,
        max_level: row.get::<_, i64>(2)? as u8,
        faction: row.get::<_, i64>(3)? as u16,
        npc_flags: row.get::<_, i64>(4)? as u32,
        speed_walk: row.get(5)?,
        health_modifier: row.get(6)?,
        name: row.get(7)?,
        rank: row.get::<_, i64>(8)? as u8,
        expansion: row.get::<_, i64>(9)? as u8,
        experience_modifier: row.get(10)?,
        gives_experience: row.get::<_, i64>(11)? != CREATURE_TYPE_CRITTER
            && row.get::<_, i64>(12)? & CREATURE_FLAG_EXTRA_NO_XP == 0,
        models: Vec::new(),
    })
}

fn load_templates(conn: &Connection) -> HashMap<u32, CreatureTemplate> {
    let mut stmt = conn
        .prepare(
            "SELECT entry, minlevel, maxlevel, faction, npcflag, speed_walk, HealthModifier, name,
                    rank, exp, ExperienceModifier, type, flags_extra
             FROM creature_template",
        )
        .expect("Failed to prepare creature_template query");

    let rows = stmt
        .query_map(params![], template_from_row)
        .expect("Failed to query creature_template");

    let mut map = HashMap::new();
    for row in rows {
        let t = row.expect("Failed to read creature_template row");
        map.insert(t.entry, t);
    }
    map
}

fn load_models(conn: &Connection, templates: &mut HashMap<u32, CreatureTemplate>) {
    let mut stmt = conn
        .prepare(
            "SELECT CreatureID, CreatureDisplayID, DisplayScale, Probability
             FROM creature_template_model
             ORDER BY CreatureID, Idx",
        )
        .expect("Failed to prepare creature_template_model query");

    let rows = stmt
        .query_map(params![], |row| {
            Ok((
                row.get::<_, i64>(0)? as u32,
                CreatureModelInfo {
                    display_id: row.get::<_, i64>(1)? as u32,
                },
            ))
        })
        .expect("Failed to query creature_template_model");

    for row in rows {
        let (creature_id, model) = row.expect("Failed to read creature_template_model row");
        if let Some(template) = templates.get_mut(&creature_id) {
            template.models.push(model);
        }
    }
}

fn spawn_from_row(row: &Row) -> rusqlite::Result<CreatureSpawn> {
    Ok(CreatureSpawn {
        guid: row.get::<_, i64>(0)? as u32,
        template_entry: row.get::<_, i64>(1)? as u32,
        map: row.get::<_, i64>(2)? as u16,
        zone_id: row.get::<_, i64>(3)? as u16,
        position_x: row.get(4)?,
        position_y: row.get(5)?,
        position_z: row.get(6)?,
        wander_distance: row.get(7)?,
        movement_type: row.get::<_, i64>(8)? as u8,
        orientation: row.get(9)?,
    })
}

fn load_spawns(conn: &Connection) -> Vec<CreatureSpawn> {
    // No events are activated yet. Negative memberships spawn while inactive.
    let mut stmt = conn
        .prepare(
            "SELECT guid, id1, map, zoneId, position_x, position_y, position_z,
                    wander_distance, MovementType, orientation
             FROM creature
             WHERE NOT EXISTS (
                 SELECT 1 FROM game_event_creature event WHERE event.guid = creature.guid
             ) OR EXISTS (
                 SELECT 1 FROM game_event_creature event
                 WHERE event.guid = creature.guid AND event.eventEntry < 0
             )",
        )
        .expect("Failed to prepare creature query; import game_event_creature into world.db");

    let rows = stmt
        .query_map(params![], spawn_from_row)
        .expect("Failed to query creature");

    rows.map(|r| r.expect("Failed to read creature row"))
        .collect()
}

fn build_spawns_by_map(spawns: &[CreatureSpawn]) -> HashMap<u16, Vec<usize>> {
    let mut map: HashMap<u16, Vec<usize>> = HashMap::new();
    for (idx, spawn) in spawns.iter().enumerate() {
        map.entry(spawn.map).or_default().push(idx);
    }
    map
}

fn waypoint_point_from_row(row: &Row) -> rusqlite::Result<(u32, WaypointPoint)> {
    Ok((
        row.get::<_, i64>(0)? as u32,
        WaypointPoint {
            position_x: row.get(2)?,
            position_y: row.get(3)?,
            position_z: row.get(4)?,
            delay: row.get::<_, i64>(5)? as u32,
        },
    ))
}

fn load_waypoints(conn: &Connection) -> HashMap<u32, WaypointPath> {
    let mut stmt = conn
        .prepare(
            "SELECT id, point, position_x, position_y, position_z, delay
             FROM waypoint_data
             ORDER BY id, point",
        )
        .expect("Failed to prepare waypoint_data query");

    let rows = stmt
        .query_map(params![], waypoint_point_from_row)
        .expect("Failed to query waypoint_data");

    let mut map: HashMap<u32, WaypointPath> = HashMap::new();
    for row in rows {
        let (guid, point) = row.expect("Failed to read waypoint_data row");
        map.entry(guid)
            .or_insert_with(|| WaypointPath { points: Vec::new() })
            .points
            .push(point);
    }
    map
}

fn startup_load_world_data(mut commands: Commands) {
    let path = Path::new("data/world.db");
    let data = load_world_data(path);
    commands.insert_resource(data);
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("Failed to open world.db");
    let templates = crate::faction::load_faction_templates(&conn)
        .expect("Failed to load faction_template; run scripts/db2.py import");
    info!("Loaded {} faction templates", templates.0.len());
    commands.insert_resource(templates);
    let gossip =
        crate::npc_interaction::load_gossip_catalog(&conn).expect("Failed to load gossip menus");
    commands.insert_resource(gossip);
}

pub struct WorldDataPlugin;

impl Plugin for WorldDataPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup_load_world_data);
    }
}
