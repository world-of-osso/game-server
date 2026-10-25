//! Copy-scoped objective snapshots; no achievement metadata is pushed on entry.
use super::*;
use shared::protocol::{
    ACHIEVEMENT_NAME_MAX_BYTES, DungeonEncounterProgress, DungeonProgress, InstanceChannel,
};

#[derive(Component)]
struct LastDungeonMap(InMap);

pub(super) fn send_entry_progress(world: &mut World) {
    let entries: Vec<_> = world
        .query_filtered::<(Entity, &PlayerLink, Ref<InWorld>, Option<&LastDungeonMap>), With<Authenticated>>()
        .iter(world)
        .filter_map(|(connection, link, in_world, last)| {
            let map = *world.get::<InMap>(link.0)?;
            let changed = last.is_none_or(|last| last.0 != map);
            (in_world.is_added() || changed).then_some((connection, map))
        })
        .collect();
    let maps: BTreeSet<_> = entries.iter().map(|(_, map)| *map).collect();
    for map in maps {
        send_map_progress(world, map);
    }
    for (connection, map) in entries {
        world.entity_mut(connection).insert(LastDungeonMap(map));
    }
}

fn dungeon_snapshot(world: &World, map: InMap) -> Option<DungeonProgress> {
    let store = world.get_resource::<crate::maps::MapStore>()?;
    let instances = world.get_resource::<crate::maps::InstanceManager>()?;
    let copy = instances.copy_difficulty(map);
    let difficulty_id = copy.map_or(0, |entry| entry.difficulty_id);
    let rows = match copy {
        Some(entry) => store.encounters_for_map(map.map_id, entry.difficulty_id),
        None => Vec::new(),
    };
    let mask = instances.completed_encounters(map);
    let encounters = rows
        .into_iter()
        .map(|row| {
            DungeonEncounterProgress {
                encounter_id: row.id,
                name: catalog::bound_text(&row.name, ACHIEVEMENT_NAME_MAX_BYTES),
                defeated: mask & (1 << row.bit) != 0,
                // Flags have no verified optional bit in the pinned source. Preserve them.
                optional: None,
                flags: row.flags,
            }
        })
        .collect();
    Some(DungeonProgress {
        map_id: map.map_id,
        instance_id: map.instance_id,
        difficulty_id,
        encounters,
    })
}

pub(super) fn send_map_progress(world: &mut World, map: InMap) {
    let Some(snapshot) = dungeon_snapshot(world, map) else {
        return;
    };
    let connections: Vec<_> = world
        .query_filtered::<(Entity, &PlayerLink), (With<Authenticated>, With<InWorld>)>()
        .iter(world)
        .filter(|(_, link)| world.get::<InMap>(link.0) == Some(&map))
        .map(|(connection, _)| connection)
        .collect();
    for connection in connections {
        if let Some(mut sender) = world.get_mut::<MessageSender<DungeonProgress>>(connection) {
            sender.send::<InstanceChannel>(snapshot.clone());
        }
    }
}
