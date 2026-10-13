//! Authored templates for scripted BG objects without fabricated static placements.
use crate::game_objects::GameObjectBounds;
use crate::maps::InMap;
use bevy::prelude::*;
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
    let spawn = crate::game_objects::GameObjectSpawn {
        guid: 0, // Runtime summon: no static content_gameobject GUID.
        map_id: map.map_id,
        info,
        position: at,
        orientation,
        initial_state: crate::game_objects::GO_STATE_READY,
        bounds: GameObjectBounds {
            orientation,
            ..bounds
        },
        spawn_difficulties: vec![0],
        chair: None,
        quest_object: None,
        script_name: String::new(),
    };
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, world);
    let object = crate::game_objects::spawn_game_object(&mut commands, &spawn, map);
    queue.apply(world);
    object
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripted_azerite_emits_ready_native_object_state() {
        let mut world = World::new();
        let info = GameObjectInfo {
            entry: 281307,
            go_type: 42,
            display_id: 47685,
            name: "Azerite".into(),
            scale: 1.1,
        };
        let bounds = GameObjectBounds {
            min: Vec3::ZERO,
            max: Vec3::ONE,
            scale: 1.1,
            orientation: 0.0,
        };
        world.insert_resource(BattlegroundObjectTemplates(BTreeMap::from([(
            info.entry,
            (info.clone(), bounds),
        )])));
        let map = InMap {
            map_id: 1803,
            instance_id: 9,
        };
        let at = Vec3::new(1343.72, 2919.95, 32.87);
        let object = spawn_scripted_object(&mut world, map, info.entry, at, 0.5);
        let entity = world.entity(object);
        assert_eq!(entity.get::<GameObjectInfo>(), Some(&info));
        assert_eq!(entity.get::<InMap>(), Some(&map));
        assert_eq!(
            entity
                .get::<shared::protocol::GameObjectCollision>()
                .map(|state| state.0),
            Some(true),
            "native object snapshot requires explicit READY collision state"
        );
        assert_eq!(
            entity
                .get::<shared::protocol::GameObjectVisible>()
                .map(|state| state.0),
            Some(true)
        );
        assert_eq!(crate::game_objects::wow_position(entity.get().unwrap()), at);
    }
}
