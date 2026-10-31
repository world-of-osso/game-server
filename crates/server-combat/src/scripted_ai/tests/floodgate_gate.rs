use super::*;
use shared::protocol::GameObjectCollision;

#[test]
fn geezle_releases_near_player_and_melee_starts_encounter_3054() {
    let (mut h, boss, map) = bound_boss_at_difficulty(226404, 2773, 3, 205, None);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    h.app
        .insert_resource(crate::scripted_ai::model_reach::load_model_reaches(&conn).unwrap());
    let other_map = InMap {
        instance_id: map.instance_id + 1,
        ..map
    };
    let (other_boss, _) = spawn_authored_actor(&mut h.app, &conn, 226404, other_map, 205);
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    let player = h.player();
    let place = |h: &mut Harness, dx: f32| {
        h.app.world_mut().entity_mut(player).insert(Position {
            x: origin.x + dx,
            ..origin
        });
    };
    let immunity = UnitFlags::IMMUNE_TO_PC | UnitFlags::IMMUNE_TO_NPC;
    place(&mut h, 23.0);
    h.advance(0.2);
    let initial = h.app.world().get::<UnitFlags>(boss).unwrap().0;
    assert_eq!(initial & immunity, immunity, "23 yd is outside the release");

    place(&mut h, 21.0);
    h.advance(0.2);
    let released = h.app.world().get::<UnitFlags>(boss).unwrap().0;
    assert_eq!(released & immunity, 0, "21 yd releases Geezle");
    assert_eq!(released & !immunity, initial & !immunity);
    assert_eq!(
        h.app.world().get::<UnitFlags>(other_boss).unwrap().0 & immunity,
        immunity,
        "another copy stays protected"
    );

    h.app
        .world_mut()
        .entity_mut(player)
        .insert(shared::components::UnitLevel(80));
    place(&mut h, 3.0);
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(crate::combat::AutoAttack::new(boss));
    h.advance(10.0);
    let health = h.app.world().get::<Health>(boss).unwrap();
    assert!(health.current < health.max, "melee damages Geezle");
    let world = h.app.world_mut();
    let encounter = instance_script::boss_id_for_encounter(world, map, 3054).unwrap();
    assert_eq!(
        instance_script::get_boss_state(world, map, encounter),
        Some(EncounterState::InProgress)
    );
}

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
