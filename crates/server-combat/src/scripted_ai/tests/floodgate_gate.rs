use super::*;
use shared::protocol::GameObjectCollision;

#[test]
fn swampface_done_opens_dam_and_only_its_collision_proxy() {
    let (mut h, _, map) = bound_boss_at_difficulty(226396, 2773, 2, 205, None);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let spawns = crate::game_objects::load_game_object_spawns(&conn).unwrap();
    let other = InMap {
        instance_id: map.instance_id + 1,
        ..map
    };
    let source = |guid| {
        spawns
            .0
            .iter()
            .find(|spawn| spawn.map_id == 2773 && spawn.guid == guid)
            .expect("authored Floodgate object")
    };
    let world = h.app.world_mut();
    let gate = world
        .spawn((
            crate::game_objects::game_object_bundle(source(30001612)),
            map,
        ))
        .id();
    let proxy = world
        .spawn((
            crate::game_objects::game_object_bundle(source(30001653)),
            map,
        ))
        .id();
    let earlier_proxy = world
        .spawn((
            crate::game_objects::game_object_bundle(source(30001630)),
            map,
        ))
        .id();
    let other_proxy = world
        .spawn((
            crate::game_objects::game_object_bundle(source(30001653)),
            other,
        ))
        .id();
    crate::scripted_ai::door_data::update_doors(world, map);
    for object in [gate, proxy, earlier_proxy, other_proxy] {
        assert_eq!(
            world.get::<GameObjectCollision>(object),
            Some(&GameObjectCollision(true))
        );
    }
    let boss = instance_script::boss_id_for_encounter(world, map, 3053).unwrap();
    // The fixture's nearby player may already have pulled Swampface.
    if instance_script::get_boss_state(world, map, boss) == Some(EncounterState::NotStarted) {
        assert!(instance_script::set_boss_state(
            world,
            map,
            boss,
            EncounterState::InProgress
        ));
    }
    assert_eq!(
        instance_script::get_boss_state(world, map, boss),
        Some(EncounterState::InProgress)
    );
    assert_eq!(
        world.get::<GameObjectCollision>(gate),
        Some(&GameObjectCollision(true))
    );
    assert_eq!(
        world.get::<GameObjectCollision>(proxy),
        Some(&GameObjectCollision(true))
    );
    assert!(instance_script::set_boss_state(
        world,
        map,
        boss,
        EncounterState::Done
    ));
    for object in [gate, proxy] {
        assert_eq!(
            world.get::<GameObjectCollision>(object),
            Some(&GameObjectCollision(false))
        );
        assert_eq!(
            world
                .get::<crate::game_objects::GameObjectState>(object)
                .unwrap()
                .0,
            0
        );
    }
    for object in [earlier_proxy, other_proxy] {
        assert_eq!(
            world.get::<GameObjectCollision>(object),
            Some(&GameObjectCollision(true))
        );
    }
    let recreated = world
        .spawn((
            crate::game_objects::game_object_bundle(source(30001653)),
            map,
        ))
        .id();
    crate::scripted_ai::door_data::update_doors(world, map);
    assert_eq!(
        world.get::<GameObjectCollision>(recreated),
        Some(&GameObjectCollision(false))
    );
}
