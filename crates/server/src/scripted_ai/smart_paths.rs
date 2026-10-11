//! TrinityCore a352b1fa waypoint_path/node data used by SmartAI::StartPath.
use rusqlite::Connection;
use shared::components::Position;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug)]
pub(super) struct WaypointNode {
    pub id: u32,
    pub position: Position,
}

#[derive(Debug)]
pub(super) struct WaypointPath {
    pub walk: bool,
    pub nodes: Vec<WaypointNode>,
}

pub(super) struct EscortPath {
    pub path_id: u32,
    pub path: Arc<WaypointPath>,
    pub index: usize,
    pub pause_ms: Option<u32>,
    pub reached: bool,
}

pub(super) fn load_waypoint_paths(
    conn: &Connection,
) -> rusqlite::Result<HashMap<u32, Arc<WaypointPath>>> {
    let mut query = conn.prepare(
        "SELECT p.PathId,p.MoveType,n.NodeId,n.PositionX,n.PositionY,n.PositionZ
         FROM tdb_waypoint_path p JOIN tdb_waypoint_path_node n USING(PathId)
         WHERE p.PathId IN (SELECT action_param2 FROM tdb_smart_scripts WHERE action_type=53)
         ORDER BY p.PathId,n.NodeId",
    )?;
    let rows = query.query_map([], |row| {
        let wow: (f32, f32, f32) = (row.get(3)?, row.get(4)?, row.get(5)?);
        let (x, y, z) = crate::terrain_height::wow_to_bevy(wow.0, wow.1, wow.2);
        Ok((
            row.get::<_, u32>(0)?,
            row.get::<_, u32>(1)? == 0,
            WaypointNode {
                id: row.get(2)?,
                position: Position { x, y, z },
            },
        ))
    })?;
    let mut paths: HashMap<u32, WaypointPath> = HashMap::new();
    for row in rows {
        let (id, walk, node) = row?;
        paths
            .entry(id)
            .or_insert_with(|| WaypointPath {
                walk,
                nodes: Vec::new(),
            })
            .nodes
            .push(node);
    }
    Ok(paths
        .into_iter()
        .map(|(id, path)| (id, Arc::new(path)))
        .collect())
}
