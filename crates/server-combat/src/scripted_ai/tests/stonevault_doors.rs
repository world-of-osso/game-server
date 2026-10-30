//! TC 15e0ed76 instance_the_stonevault.cpp doorData.
use super::*;
use shared::protocol::GameObjectCollision;

#[test]
fn stonevault_edna_done_opens_both_path_doors_only_in_its_copy() {
    let (mut h, _, map) = bound_boss_at_difficulty(210108, 2652, 0, 205, None);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let spawns = crate::game_objects::load_game_object_spawns(&conn).unwrap();
    let other = InMap {
        instance_id: map.instance_id + 1,
        ..map
    };
    let mut doors = Vec::new();
    for entry in [440235, 440239] {
        let source = spawns
            .0
            .iter()
            .find(|spawn| spawn.map_id == 2652 && spawn.info.entry == entry)
            .unwrap();
        let first = h
            .app
            .world_mut()
            .spawn((crate::game_objects::game_object_bundle(source), map))
            .id();
        let second = h
            .app
            .world_mut()
            .spawn((crate::game_objects::game_object_bundle(source), other))
            .id();
        doors.push((source, first, second));
    }
    crate::scripted_ai::door_data::update_doors(h.app.world_mut(), map);
    for (_, first, second) in &doors {
        assert_eq!(
            h.app.world().get::<GameObjectCollision>(*first),
            Some(&GameObjectCollision(true))
        );
        assert_eq!(
            h.app.world().get::<GameObjectCollision>(*second),
            Some(&GameObjectCollision(true))
        );
    }
    let world = h.app.world_mut();
    let boss = instance_script::boss_id_for_encounter(world, map, 2854).unwrap();
    assert!(instance_script::set_boss_state(
        world,
        map,
        boss,
        EncounterState::Done
    ));
    for (source, first, second) in doors {
        assert_eq!(
            world.get::<GameObjectCollision>(first),
            Some(&GameObjectCollision(false))
        );
        assert_eq!(
            world.get::<GameObjectCollision>(second),
            Some(&GameObjectCollision(true))
        );
        let recreated = world
            .spawn((crate::game_objects::game_object_bundle(source), map))
            .id();
        crate::scripted_ai::door_data::update_doors(world, map);
        assert_eq!(
            world.get::<GameObjectCollision>(recreated),
            Some(&GameObjectCollision(false))
        );
    }
}
