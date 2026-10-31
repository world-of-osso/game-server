use super::*;
use shared::protocol::GameObjectCollision;

#[test]
fn avanoxx_done_opens_spike_wall_only_in_its_instance() {
    let (mut h, _, map) = bound_boss_at_difficulty(213179, 2660, 2, 205, None);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let spawns = crate::game_objects::load_game_object_spawns(&conn).unwrap();
    let other = InMap {
        instance_id: map.instance_id + 1,
        ..map
    };
    let mut walls = Vec::new();
    for entry in [455376, 455377, 455378, 467576] {
        let source = spawns
            .0
            .iter()
            .find(|spawn| spawn.map_id == 2660 && spawn.info.entry == entry)
            .expect("authored Ara-Kara barrier");
        let wall = h
            .app
            .world_mut()
            .spawn((crate::game_objects::game_object_bundle(source), map))
            .id();
        let other_wall = h
            .app
            .world_mut()
            .spawn((crate::game_objects::game_object_bundle(source), other))
            .id();
        walls.push((source, wall, other_wall));
    }
    let anub_source = spawns
        .0
        .iter()
        .find(|spawn| spawn.map_id == 2660 && spawn.info.entry == 455375)
        .expect("authored Anub'zekt wall");
    let anub_wall = h
        .app
        .world_mut()
        .spawn((crate::game_objects::game_object_bundle(anub_source), map))
        .id();
    let other_anub_wall = h
        .app
        .world_mut()
        .spawn((crate::game_objects::game_object_bundle(anub_source), other))
        .id();
    crate::scripted_ai::door_data::update_doors(h.app.world_mut(), map);
    assert_eq!(
        h.app.world().get::<GameObjectCollision>(anub_wall),
        Some(&GameObjectCollision(true))
    );
    for (_, wall, other_wall) in &walls {
        for object in [*wall, *other_wall] {
            assert_eq!(
                h.app.world().get::<GameObjectCollision>(object),
                Some(&GameObjectCollision(true))
            );
        }
    }
    let world = h.app.world_mut();
    let boss = instance_script::boss_id_for_encounter(world, map, 2926).unwrap();
    assert!(instance_script::set_boss_state(
        world,
        map,
        boss,
        EncounterState::Done
    ));
    assert_eq!(
        world.get::<GameObjectCollision>(anub_wall),
        Some(&GameObjectCollision(true)),
        "Avanoxx must not open Anub'zekt's wall"
    );
    for (source, wall, other_wall) in walls {
        assert_eq!(
            world.get::<GameObjectCollision>(wall),
            Some(&GameObjectCollision(false))
        );
        assert_eq!(
            world.get::<GameObjectCollision>(other_wall),
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
    let anub = instance_script::boss_id_for_encounter(world, map, 2906).unwrap();
    assert!(instance_script::set_boss_state(
        world,
        map,
        anub,
        EncounterState::Done
    ));
    assert_eq!(
        world.get::<GameObjectCollision>(anub_wall),
        Some(&GameObjectCollision(false))
    );
    assert_eq!(
        world.get::<GameObjectCollision>(other_anub_wall),
        Some(&GameObjectCollision(true))
    );
    let recreated = world
        .spawn((crate::game_objects::game_object_bundle(anub_source), map))
        .id();
    crate::scripted_ai::door_data::update_doors(world, map);
    assert_eq!(
        world.get::<GameObjectCollision>(recreated),
        Some(&GameObjectCollision(false))
    );
}

#[test]
fn avanoxx_requires_three_distinct_attendant_deaths_then_waits_in_its_copy() {
    let (mut h, boss, map) = bound_boss_at_difficulty(213179, 2660, 2, 205, None);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let native =
        crate::spell_cast::test_harness::load_native_creature_world(&[217531, 218324, 217533]);
    {
        let mut data = h.app.world_mut().resource_mut::<WorldData>();
        data.templates.extend(native.templates);
        data.difficulty_templates
            .extend(native.difficulty_templates);
    }
    let other_map = InMap {
        instance_id: map.instance_id + 1,
        ..map
    };
    let (other_boss, _) = spawn_authored_actor(&mut h.app, &conn, 213179, other_map, 205);
    let (other_attendant, _) = spawn_authored_actor(&mut h.app, &conn, 217533, other_map, 205);
    let actors: Vec<_> = [217531, 218324, 217533]
        .into_iter()
        .map(|entry| spawn_authored_actor(&mut h.app, &conn, entry, map, 205).0)
        .collect();
    let (duplicate, _) = spawn_authored_actor(&mut h.app, &conn, 217531, map, 205);
    let blocked = UnitFlags::IMMUNE_TO_PC | UnitFlags::NOT_SELECTABLE;
    let kill = |h: &mut Harness, actor| {
        let damage = h.health(actor).max * 2.0;
        crate::combat::deal_direct_damage(h.app.world_mut(), actor, actor, damage);
        h.advance(0.1);
        assert_eq!(h.health(actor).current, 0.0);
    };
    kill(&mut h, other_attendant);
    kill(&mut h, actors[0]);
    kill(&mut h, duplicate);
    kill(&mut h, actors[1]);
    h.advance(18.0);
    assert_eq!(
        h.app.world().get::<UnitFlags>(boss).unwrap().0 & blocked,
        blocked
    );
    kill(&mut h, actors[2]);
    h.advance(17.0);
    assert_eq!(
        h.app.world().get::<UnitFlags>(boss).unwrap().0 & blocked,
        blocked
    );
    h.advance(0.5);
    assert_eq!(h.app.world().get::<UnitFlags>(boss).unwrap().0 & blocked, 0);
    assert_eq!(
        h.app.world().get::<UnitFlags>(other_boss).unwrap().0 & blocked,
        blocked
    );
    let (respawned, _) = spawn_authored_actor(&mut h.app, &conn, 213179, map, 205);
    h.advance(0.1);
    assert_eq!(
        h.app.world().get::<UnitFlags>(respawned).unwrap().0 & blocked,
        0
    );
}
