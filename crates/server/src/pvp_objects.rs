//! Authored templates for scripted BG objects without fabricated static placements.
use crate::game_objects::GameObjectBounds;
use crate::maps::InMap;
use bevy::prelude::*;
use lightyear::prelude::*;
use rusqlite::Connection;
use shared::protocol::GameObjectInfo;
use std::collections::BTreeMap;

#[derive(Resource, Debug, Clone, Default)]
pub(crate) struct BattlegroundObjectTemplates(BTreeMap<u32, (GameObjectInfo, GameObjectBounds)>);

pub(crate) fn load_battleground_objects(
    conn: &Connection,
) -> rusqlite::Result<BattlegroundObjectTemplates> {
    let mut query=conn.prepare("SELECT t.entry,t.type,t.displayId,t.name,t.size,d.GeoBox_0,d.GeoBox_1,d.GeoBox_2,d.GeoBox_3,d.GeoBox_4,d.GeoBox_5 FROM tdb_gameobject_template t JOIN game_object_display_info d ON d.ID=t.displayId WHERE t.entry IN (272471,281306,281307,422412)")?;
    let rows = query.query_map([], |row| {
        let scale = row.get::<_, f64>(4)? as f32;
        let geo = |i| row.get::<_, f64>(i).map(|value| value as f32);
        let info = GameObjectInfo {
            entry: row.get(0)?,
            go_type: row.get(1)?,
            display_id: row.get(2)?,
            name: row.get(3)?,
            scale,
        };
        let bounds = GameObjectBounds {
            min: Vec3::new(geo(5)?, geo(6)?, geo(7)?),
            max: Vec3::new(geo(8)?, geo(9)?, geo(10)?),
            scale,
            orientation: 0.0,
        };
        Ok((info.entry, (info, bounds)))
    })?;
    Ok(BattlegroundObjectTemplates(
        rows.collect::<rusqlite::Result<_>>()?,
    ))
}

pub(super) fn require_scripted_objects(world: &World, entries: &[u32]) -> Result<(), String> {
    let templates = world
        .get_resource::<BattlegroundObjectTemplates>()
        .ok_or("scripted battleground object catalog missing")?;
    for entry in entries {
        if !templates.0.contains_key(entry) {
            return Err(format!(
                "source battleground object template/display {entry} missing"
            ));
        }
    }
    Ok(())
}

pub(super) fn spawn_scripted_object(
    world: &mut World,
    map: InMap,
    entry: u32,
    at: Vec3,
    orientation: f32,
) -> Entity {
    let (info, bounds) = world
        .resource::<BattlegroundObjectTemplates>()
        .0
        .get(&entry)
        .expect("validated scripted BG template/display")
        .clone();
    let (x, y, z) = crate::terrain_height::wow_to_bevy(at.x, at.y, at.z);
    world
        .spawn((
            shared::components::Position { x, y, z },
            shared::components::Rotation {
                x: 0.0,
                y: orientation + std::f32::consts::FRAC_PI_2,
                z: 0.0,
            },
            info,
            // Script summons use the source default GO_STATE_READY; publish that
            // state explicitly, just as ordinary authored GO spawns do.
            shared::protocol::GameObjectCollision(true),
            shared::protocol::GameObjectVisible(true),
            GameObjectBounds {
                orientation,
                ..bounds
            },
            map,
            Replicate::to_clients(NetworkTarget::All),
            crate::interest::NetworkVisibility::default(),
        ))
        .id()
}
