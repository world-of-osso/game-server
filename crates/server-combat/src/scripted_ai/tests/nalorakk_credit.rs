use super::*;

#[test]
fn nalorakk_water_door_opens_after_hoardmonger_in_its_copy() {
    use crate::game_objects::{GO_STATE_ACTIVE, GO_STATE_READY, GameObjectState};
    use shared::protocol::GameObjectCollision;

    let (mut h, _, map) = bound_boss_at_difficulty(241812, 2825, 0, 205, None);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let spawns = crate::game_objects::load_game_object_spawns(&conn).unwrap();
    let source = spawns
        .0
        .iter()
        .find(|spawn| spawn.info.entry == 531150 && spawn.map_id == 2825)
        .unwrap();
    let other_map = InMap {
        instance_id: map.instance_id + 1,
        ..map
    };
    let door = h
        .app
        .world_mut()
        .spawn((crate::game_objects::game_object_bundle(source), map))
        .id();
    let other = h
        .app
        .world_mut()
        .spawn((crate::game_objects::game_object_bundle(source), other_map))
        .id();
    crate::scripted_ai::door_data::update_doors(h.app.world_mut(), map);
    assert_eq!(
        h.app.world().get::<GameObjectState>(door),
        Some(&GameObjectState(GO_STATE_READY))
    );
    assert_eq!(
        h.app.world().get::<GameObjectCollision>(door),
        Some(&GameObjectCollision(true))
    );
    let world = h.app.world_mut();
    let boss = instance_script::boss_id_for_encounter(world, map, 3207).unwrap();
    assert!(instance_script::set_boss_state(
        world,
        map,
        boss,
        EncounterState::Done
    ));
    assert_eq!(
        world.get::<GameObjectState>(door),
        Some(&GameObjectState(GO_STATE_ACTIVE))
    );
    assert_eq!(
        world.get::<GameObjectCollision>(door),
        Some(&GameObjectCollision(false))
    );
    assert_eq!(
        world.get::<GameObjectCollision>(other),
        Some(&GameObjectCollision(true))
    );
    let recreated = world
        .spawn((crate::game_objects::game_object_bundle(source), map))
        .id();
    crate::scripted_ai::door_data::update_doors(world, map);
    assert_eq!(
        world.get::<GameObjectState>(recreated),
        Some(&GameObjectState(GO_STATE_ACTIVE))
    );
    assert_eq!(
        world.get::<GameObjectCollision>(recreated),
        Some(&GameObjectCollision(false))
    );
}

#[test]
fn nalorakk_cave_door_opens_after_sentinel_in_its_copy() {
    use crate::game_objects::{GO_STATE_ACTIVE, GO_STATE_READY, GameObjectState};
    use shared::protocol::GameObjectCollision;

    let (mut h, _, map) = bound_boss_at_difficulty(244100, 2825, 1, 205, None);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let spawns = crate::game_objects::load_game_object_spawns(&conn).unwrap();
    let source = spawns
        .0
        .iter()
        .find(|spawn| spawn.info.entry == 570512 && spawn.map_id == 2825)
        .unwrap();
    let other_map = InMap {
        instance_id: map.instance_id + 1,
        ..map
    };
    let door = h
        .app
        .world_mut()
        .spawn((crate::game_objects::game_object_bundle(source), map))
        .id();
    let other = h
        .app
        .world_mut()
        .spawn((crate::game_objects::game_object_bundle(source), other_map))
        .id();
    crate::scripted_ai::door_data::update_doors(h.app.world_mut(), map);
    assert_eq!(
        h.app.world().get::<GameObjectState>(door),
        Some(&GameObjectState(GO_STATE_READY))
    );
    assert_eq!(
        h.app.world().get::<GameObjectCollision>(door),
        Some(&GameObjectCollision(true))
    );
    let world = h.app.world_mut();
    let boss = instance_script::boss_id_for_encounter(world, map, 3208).unwrap();
    if instance_script::get_boss_state(world, map, boss) == Some(EncounterState::InProgress) {
        assert!(instance_script::set_boss_state(
            world,
            map,
            boss,
            EncounterState::NotStarted
        ));
    }
    assert!(instance_script::set_boss_state(
        world,
        map,
        boss,
        EncounterState::InProgress
    ));
    assert_eq!(
        world.get::<GameObjectCollision>(door),
        Some(&GameObjectCollision(true))
    );
    assert!(instance_script::set_boss_state(
        world,
        map,
        boss,
        EncounterState::Done
    ));
    assert_eq!(
        world.get::<GameObjectState>(door),
        Some(&GameObjectState(GO_STATE_ACTIVE))
    );
    assert_eq!(
        world.get::<GameObjectCollision>(door),
        Some(&GameObjectCollision(false))
    );
    assert_eq!(
        world.get::<GameObjectCollision>(other),
        Some(&GameObjectCollision(true))
    );
    let recreated = world
        .spawn((crate::game_objects::game_object_bundle(source), map))
        .id();
    crate::scripted_ai::door_data::update_doors(world, map);
    assert_eq!(
        world.get::<GameObjectState>(recreated),
        Some(&GameObjectState(GO_STATE_ACTIVE))
    );
    assert_eq!(
        world.get::<GameObjectCollision>(recreated),
        Some(&GameObjectCollision(false))
    );
}

#[test]
fn hoardmonger_unblocked_attack_enters_combat_without_regeneration() {
    use shared::components::{CombatStatus, UnitFlags};
    let (mut h, boss, _) = bound_boss_at_difficulty(241812, 2825, 0, 205, None);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    h.app
        .insert_resource(crate::scripted_ai::model_reach::load_model_reaches(&conn).unwrap());
    let player = h.player();
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(shared::components::UnitLevel(90));
    let pos = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: pos.x + 3.0,
        ..pos
    });
    assert!(h.app.world().get::<ThreatTable>(boss).is_none());
    h.app.world_mut().get_mut::<UnitFlags>(boss).unwrap().0 &=
        !(UnitFlags::IMMUNE_TO_PC | UnitFlags::IMMUNE_TO_NPC);
    h.advance(0.2);
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(crate::combat::AutoAttack::new(boss));
    h.advance(15.0);
    let threat = h
        .app
        .world()
        .get::<ThreatTable>(boss)
        .unwrap()
        .threat_for(player.to_bits());
    assert!(
        threat > 0.0,
        "melee must produce threat: {threat}; boss health {:?} flags {:?} position {:?} state {:?}; player position {:?} attack {:?}; reach {:?}; events {:?}",
        h.health(boss),
        h.app.world().get::<UnitFlags>(boss),
        h.app.world().get::<Position>(boss),
        h.app.world().get::<crate::creature_ai::CreatureState>(boss),
        h.app.world().get::<Position>(player),
        h.app.world().get::<crate::combat::AutoAttack>(player),
        crate::scripted_ai::model_reach::melee_range(h.app.world(), player, boss),
        h.inbox(0).log
    );
    assert_eq!(
        h.app.world().get::<CombatStatus>(boss),
        Some(&CombatStatus(true))
    );
    let damaged = h.app.world().get::<Health>(boss).unwrap().current;
    assert!(damaged < h.app.world().get::<Health>(boss).unwrap().max);
    h.advance(5.0);
    assert!(
        h.app.world().get::<Health>(boss).unwrap().current <= damaged,
        "boss regenerated while in combat"
    );
}

#[test]
fn nalorakk_credit_lethal_damage_completes_encounter_3209() {
    let (_directory, path) = prepare_encounter_database("nalorakk");
    let source = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let maps = crate::maps::store::load_map_store(&source).unwrap();
    assert_eq!(maps.raid_creature_encounters.get(&(2825, 246404)), Some(&2));
    let (mut h, boss, map) = bound_boss_at_difficulty(246404, 2825, 2, 205, None);
    h.app.insert_resource(maps);
    let pos = *h.app.world().get::<Position>(boss).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: pos.x + 3.0,
        ..pos
    });
    h.engage(boss);
    h.advance(0.1);
    h.hit_pct(boss, 1_000.0);
    h.advance(0.1);
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 2),
        Some(EncounterState::Done)
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map)
            & 4,
        4
    );
    assert!(
        h.inbox(0)
            .ends
            .iter()
            .any(|end| end.encounter_id == 3209 && end.success)
    );
}
