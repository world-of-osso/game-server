use rusqlite::{Connection, OpenFlags};
use std::path::Path;

#[path = "../followers/pack_identity.rs"]
mod pack_identity;

pub struct Spawn {
    pub guid: u32,
    pub entry: u32,
    pub pack: u32,
    pub position: [f32; 3],
}

#[derive(Default)]
pub struct Scene {
    pub polygons: Vec<Vec<[f32; 3]>>,
    pub spawns: Vec<Spawn>,
}

pub fn read_scene(database: &Path, navmesh: &Path) -> Result<Scene, String> {
    let conn = Connection::open_with_flags(database, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("{}: {error}", database.display()))?;
    let mut query = conn
        .prepare(
            "SELECT c.guid,c.id1,c.position_x,c.position_y,c.position_z,f.leaderGUID
         FROM content_creature c
         LEFT JOIN content_creature_formations f ON f.memberGUID=c.guid AND EXISTS (
             SELECT 1 FROM content_creature_formations leader
             WHERE leader.memberGUID=f.leaderGUID AND leader.leaderGUID=f.leaderGUID)
         WHERE c.map=34 ORDER BY c.guid",
        )
        .map_err(|error| format!("Read map34 source spawns: {error}"))?;
    let spawns = query
        .query_map([], |row| {
            let guid = row.get(0)?;
            let leader: Option<u32> = row.get(5)?;
            let x: f32 = row.get(2)?;
            let y: f32 = row.get(3)?;
            let z: f32 = row.get(4)?;
            Ok(Spawn {
                guid,
                entry: row.get(1)?,
                pack: pack_identity::pack_guid(guid, leader),
                position: [x, z, -y],
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(Scene {
        polygons: navigation::read_walkable_polygons(navmesh)?,
        spawns,
    })
}
