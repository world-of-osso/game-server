use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags, Row, params};
use std::collections::HashMap;
use std::path::Path;

pub struct CreatureTemplate {
    pub entry: u32,
    pub name: String,
    pub subname: Option<String>,
    pub min_level: u8,
    pub max_level: u8,
    pub faction: u16,
    pub npc_flags: u32,
    pub speed_walk: f32,
    pub speed_run: f32,
    pub scale: f32,
    pub rank: u8,
    pub unit_class: u8,
    pub unit_flags: u32,
    pub creature_type: u8,
    pub health_modifier: f32,
    pub mana_modifier: f32,
    pub ai_name: String,
    pub movement_type: u8,
    pub flags_extra: u32,
    pub models: Vec<CreatureModelInfo>,
}

pub struct CreatureModelInfo {
    pub display_id: u32,
    pub display_scale: f32,
    pub probability: f32,
}

pub struct CreatureSpawn {
    pub guid: u32,
    pub template_entry: u32,
    pub map: u16,
    pub zone_id: u16,
    pub area_id: u16,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub respawn_time_secs: u32,
    pub wander_distance: f32,
    pub movement_type: u8,
}

pub struct WaypointPath {
    pub creature_guid: u32,
    pub points: Vec<WaypointPoint>,
}

pub struct WaypointPoint {
    pub point: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: Option<f32>,
    pub delay: u32,
    pub move_type: u8,
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

fn template_from_row(row: &Row) -> rusqlite::Result<CreatureTemplate> {
    Ok(CreatureTemplate {
        entry: row.get::<_, i64>(0)? as u32,
        name: row.get(1)?,
        subname: row.get(2)?,
        min_level: row.get::<_, i64>(3)? as u8,
        max_level: row.get::<_, i64>(4)? as u8,
        faction: row.get::<_, i64>(5)? as u16,
        npc_flags: row.get::<_, i64>(6)? as u32,
        speed_walk: row.get(7)?,
        speed_run: row.get(8)?,
        scale: row.get(9)?,
        rank: row.get::<_, i64>(10)? as u8,
        unit_class: row.get::<_, i64>(11)? as u8,
        unit_flags: row.get::<_, i64>(12)? as u32,
        creature_type: row.get::<_, i64>(13)? as u8,
        health_modifier: row.get(14)?,
        mana_modifier: row.get(15)?,
        ai_name: row.get(16)?,
        movement_type: row.get::<_, i64>(17)? as u8,
        flags_extra: row.get::<_, i64>(18)? as u32,
        models: Vec::new(),
    })
}

fn load_templates(conn: &Connection) -> HashMap<u32, CreatureTemplate> {
    let mut stmt = conn
        .prepare(
            "SELECT entry, name, subname, minlevel, maxlevel, faction, npcflag,
                    speed_walk, speed_run, scale, rank, unit_class, unit_flags,
                    type, HealthModifier, ManaModifier, AIName, MovementType, flags_extra
             FROM creature_template",
        )
        .expect("Failed to prepare creature_template query");

    let rows = stmt
        .query_map(params![], |row| template_from_row(row))
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
                    display_scale: row.get(2)?,
                    probability: row.get(3)?,
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
        area_id: row.get::<_, i64>(4)? as u16,
        position_x: row.get(5)?,
        position_y: row.get(6)?,
        position_z: row.get(7)?,
        orientation: row.get(8)?,
        respawn_time_secs: row.get::<_, i64>(9)? as u32,
        wander_distance: row.get(10)?,
        movement_type: row.get::<_, i64>(11)? as u8,
    })
}

fn load_spawns(conn: &Connection) -> Vec<CreatureSpawn> {
    let mut stmt = conn
        .prepare(
            "SELECT guid, id1, map, zoneId, areaId,
                    position_x, position_y, position_z, orientation,
                    spawntimesecs, wander_distance, MovementType
             FROM creature",
        )
        .expect("Failed to prepare creature query");

    let rows = stmt
        .query_map(params![], |row| spawn_from_row(row))
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
            point: row.get::<_, i64>(1)? as u32,
            position_x: row.get(2)?,
            position_y: row.get(3)?,
            position_z: row.get(4)?,
            orientation: row.get(5)?,
            delay: row.get::<_, i64>(6)? as u32,
            move_type: row.get::<_, i64>(7)? as u8,
        },
    ))
}

fn load_waypoints(conn: &Connection) -> HashMap<u32, WaypointPath> {
    let mut stmt = conn
        .prepare(
            "SELECT id, point, position_x, position_y, position_z,
                    orientation, delay, move_type
             FROM waypoint_data
             ORDER BY id, point",
        )
        .expect("Failed to prepare waypoint_data query");

    let rows = stmt
        .query_map(params![], |row| waypoint_point_from_row(row))
        .expect("Failed to query waypoint_data");

    let mut map: HashMap<u32, WaypointPath> = HashMap::new();
    for row in rows {
        let (guid, point) = row.expect("Failed to read waypoint_data row");
        map.entry(guid)
            .or_insert_with(|| WaypointPath {
                creature_guid: guid,
                points: Vec::new(),
            })
            .points
            .push(point);
    }
    map
}

fn startup_load_world_data(mut commands: Commands) {
    let data = load_world_data(std::path::Path::new("data/world.db"));
    commands.insert_resource(data);
}

pub struct WorldDataPlugin;

impl Plugin for WorldDataPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup_load_world_data);
    }
}
