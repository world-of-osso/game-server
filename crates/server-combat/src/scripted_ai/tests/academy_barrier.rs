use super::*;
use shared::protocol::GameObjectCollision;

#[test]
fn academy_bramble_opens_after_three_encounters_only_in_their_copy() {
    let (mut h, _, map) = bound_boss_at_difficulty(194181, 2526, 0, 205, None);
    let connection =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let spawns = crate::game_objects::load_game_object_spawns(&connection).unwrap();
    let source = spawns
        .0
        .iter()
        .find(|spawn| spawn.info.entry == 382475)
        .unwrap();
    assert_eq!(source.map_id, 2526);
    let other_map = InMap {
        instance_id: map.instance_id + 1,
        ..map
    };
    let door = h
        .app
        .world_mut()
        .spawn((crate::game_objects::game_object_bundle(source), map))
        .id();
    let other_door = h
        .app
        .world_mut()
        .spawn((crate::game_objects::game_object_bundle(source), other_map))
        .id();
    crate::scripted_ai::door_data::update_doors(h.app.world_mut(), map);
    assert_eq!(
        h.app.world().get::<GameObjectCollision>(door),
        Some(&GameObjectCollision(true))
    );
    for (index, encounter) in [2563, 2564, 2562].into_iter().enumerate() {
        let world = h.app.world_mut();
        let boss = instance_script::boss_id_for_encounter(world, map, encounter).unwrap();
        assert!(instance_script::set_boss_state(
            world,
            map,
            boss,
            EncounterState::Done
        ));
        assert_eq!(
            world.get::<GameObjectCollision>(door),
            Some(&GameObjectCollision(index != 2))
        );
        assert_eq!(
            world.get::<GameObjectCollision>(other_door),
            Some(&GameObjectCollision(true))
        );
    }
    let recreated = h
        .app
        .world_mut()
        .spawn((crate::game_objects::game_object_bundle(source), map))
        .id();
    crate::scripted_ai::door_data::update_doors(h.app.world_mut(), map);
    assert_eq!(
        h.app.world().get::<GameObjectCollision>(recreated),
        Some(&GameObjectCollision(false))
    );
}
