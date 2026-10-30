//! Current LittleWigs2777a9d; project policy LOST_DWARVES_SURRENDER_AT_10PCT_LUA.
use super::*;

const MAP: u32 = 2451;
const DWARVES: [u32; 3] = [184581, 184582, 184580];

fn dwarf_copy(entry: u32, difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(entry, MAP, 0, difficulty, None);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    let wdt = h.app.world().resource::<MapStore>().maps[&MAP].wdt_file_data_id;
    h.app.insert_resource(crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        std::collections::HashMap::from([(MAP, wdt)]),
    ));
    (h, boss, map)
}

fn cast_count(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|g| g.caster == boss.to_bits() && g.spell_id == spell)
        .count()
}

#[test]
fn encounter_coverage_lost_dwarves_native_cleave_damage_qualifies() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = dwarf_copy(DWARVES[0], difficulty);
        let player = h.player();
        h.engage(boss);
        h.advance(0.05);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, 369563, Some(player), false)
            .expect("native369563");
        h.advance(4.5);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|e| e.kind == CombatLogKind::Damage
                    && e.amount > 0
                    && e.spell_id == Some(369563)
                    && e.source == Some(boss.to_bits())
                    && e.target == Some(player.to_bits())),
            "actual cone damage difficulty{difficulty}"
        );
    }
}

#[test]
fn encounter_coverage_lost_dwarves_first_cleave_uses_current_lua_clock() {
    let (mut h, boss, _) = dwarf_copy(DWARVES[0], 205);
    h.engage(boss);
    h.advance(12.0);
    assert_eq!(
        cast_count(&h, boss, 369563),
        1,
        "current Lua8.5s first Cleave"
    );
}

#[test]
fn encounter_coverage_lost_dwarves_overkill_surrenders_alive_and_all_three_grant_credit() {
    for difficulty in [1, 205] {
        let (mut h, baelog, map) = dwarf_copy(DWARVES[0], difficulty);
        let conn =
            Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let (eric, _) = spawn_authored_actor(&mut h.app, &conn, DWARVES[1], map, difficulty);
        let (olaf, _) = spawn_authored_actor(&mut h.app, &conn, DWARVES[2], map, difficulty);
        for (index, actor) in [baelog, eric, olaf].into_iter().enumerate() {
            h.engage(actor);
            h.hit_pct(actor, 200.0);
            h.advance(0.1);
            assert!(
                h.app
                    .world()
                    .get::<crate::combat::DeathState>(actor)
                    .is_none(),
                "surrender never requires death actor{actor:?}"
            );
            assert!(h.health(actor).current > 0.0);
            let expected = if index == 2 { 1 } else { 0 };
            assert_eq!(
                h.app
                    .world()
                    .resource::<crate::maps::InstanceManager>()
                    .completed_encounters(map),
                expected,
                "no credit until all three surrender"
            );
        }
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, 0),
            Some(EncounterState::Done)
        );
        assert_eq!(
            h.inbox(0)
                .ends
                .iter()
                .filter(|e| e.encounter_id == 2555 && e.success)
                .count(),
            1
        );
        assert!(h.inbox(1).ends.is_empty(), "copy-isolated credit");
    }
}

#[test]
fn encounter_coverage_uldaman_prehistoria_door_opens_after_bromach() {
    let (mut h, baelog, map) = dwarf_copy(DWARVES[0], 205);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let spawns = crate::game_objects::load_game_object_spawns(&conn).unwrap();
    let source = spawns
        .0
        .iter()
        .find(|spawn| spawn.info.entry == 376557)
        .unwrap();
    assert_eq!(source.map_id, MAP);
    let door = h
        .app
        .world_mut()
        .spawn((crate::game_objects::game_object_bundle(source), map))
        .id();
    let other_map = InMap {
        instance_id: map.instance_id + 1,
        ..map
    };
    let other_door = h
        .app
        .world_mut()
        .spawn((crate::game_objects::game_object_bundle(source), other_map))
        .id();
    let (eric, _) = spawn_authored_actor(&mut h.app, &conn, DWARVES[1], map, 205);
    let (olaf, _) = spawn_authored_actor(&mut h.app, &conn, DWARVES[2], map, 205);
    let (bromach, _) = spawn_authored_actor(&mut h.app, &conn, 184018, map, 205);
    for actor in [baelog, eric, olaf] {
        h.engage(actor);
        h.hit_pct(actor, 200.0);
        h.advance(0.1);
    }
    h.engage(bromach);
    h.hit_pct(bromach, 200.0);
    h.advance(0.1);
    assert_eq!(
        h.app
            .world()
            .get::<shared::protocol::GameObjectCollision>(door),
        Some(&shared::protocol::GameObjectCollision(true))
    );
    h.advance(18.0);
    assert_eq!(
        h.app
            .world()
            .get::<shared::protocol::GameObjectCollision>(door),
        Some(&shared::protocol::GameObjectCollision(true)),
        "door remains closed before 18.3s"
    );
    h.advance(0.3);
    assert_eq!(
        h.app
            .world()
            .get::<shared::protocol::GameObjectCollision>(door),
        Some(&shared::protocol::GameObjectCollision(false)),
        "Bromach after dwarves opens the sourced door"
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::protocol::GameObjectCollision>(other_door),
        Some(&shared::protocol::GameObjectCollision(true)),
        "another copy stays closed"
    );
}

#[test]
fn encounter_coverage_uldaman_prehistoria_door_requires_lost_dwarves() {
    let (mut h, _, map) = dwarf_copy(DWARVES[0], 205);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let spawns = crate::game_objects::load_game_object_spawns(&conn).unwrap();
    let source = spawns
        .0
        .iter()
        .find(|spawn| spawn.info.entry == 376557)
        .unwrap();
    let door = h
        .app
        .world_mut()
        .spawn((crate::game_objects::game_object_bundle(source), map))
        .id();
    let (bromach, _) = spawn_authored_actor(&mut h.app, &conn, 184018, map, 205);
    h.engage(bromach);
    h.hit_pct(bromach, 200.0);
    h.advance(19.0);
    assert_eq!(
        h.app
            .world()
            .get::<shared::protocol::GameObjectCollision>(door),
        Some(&shared::protocol::GameObjectCollision(true)),
        "Bromach alone must not open the door"
    );
}

#[test]
fn encounter_coverage_lost_dwarves_sustained_damage_surrenders_all_three() {
    let (mut h, baelog, map) = dwarf_copy(DWARVES[0], 205);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (eric, _) = spawn_authored_actor(&mut h.app, &conn, DWARVES[1], map, 205);
    let (olaf, _) = spawn_authored_actor(&mut h.app, &conn, DWARVES[2], map, 205);
    let actors = [baelog, eric, olaf];
    for actor in actors {
        h.engage(actor);
    }
    for round in 0..9 {
        let damage_pct = if round == 8 { 20.0 } else { 10.0 };
        for actor in actors {
            h.hit_pct(actor, damage_pct);
        }
        h.advance(2.0);
        if round < 8 {
            assert_eq!(
                h.app
                    .world()
                    .resource::<crate::maps::InstanceManager>()
                    .completed_encounters(map),
                0,
                "no encounter credit before the three surrenders"
            );
        }
    }
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        1,
        "sustained damage to all three grants encounter credit; health={:?}, state={:?}",
        actors.map(|actor| h.health(actor)),
        actors.map(|actor| h
            .app
            .world()
            .get::<crate::creature_ai::CreatureState>(actor)
            .copied())
    );
    for actor in actors {
        assert!(
            h.health(actor).current > 0.0,
            "surrendered dwarf remains alive"
        );
    }
}

#[test]
fn encounter_coverage_lost_dwarves_all_actor_current_lanes_and_native_completions() {
    for difficulty in [1, 205] {
        // Native Bulwark Attributes1/0x40 channels for12s; its occupancy delays
        // Olaf's due Shield/Longboat requests without changing source repeats.
        for (entry, lanes) in [
            (
                184581,
                vec![
                    (369563, vec![8.5, 26.8, 51.7, 68.7, 93.6, 110.6]),
                    (369573, vec![20.6, 41.2, 77.0, 97.6]),
                    (375924, vec![24.8, 103.6]),
                ],
            ),
            (
                184582,
                vec![
                    (369791, vec![6.1, 32.7, 57.6, 84.2, 109.1]),
                    (375924, vec![24.8, 103.6]),
                ],
            ),
            (
                184580,
                vec![
                    (369677, vec![12.1, 31.2, 63.1, 80.1, 110.0]),
                    (369602, vec![17.2, 51.1, 85.8, 119.7]),
                    (375924, vec![29.2, 108.0]),
                ],
            ),
        ] {
            let (mut h, boss, map) = dwarf_copy(entry, difficulty);
            let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
            h.app
                .insert_resource(super::cast_admission::CastAdmissions {
                    actor: boss,
                    origin,
                    entries: Vec::new(),
                });
            h.engage(boss);
            h.advance(130.0);
            let attempts = &h
                .app
                .world()
                .resource::<super::cast_admission::CastAdmissions>()
                .entries;
            eprintln!("DWARF{entry} difficulty{difficulty} {attempts:?}");
            let expected_count: usize = lanes.iter().map(|(_, times)| times.len()).sum();
            assert_eq!(
                attempts.len(),
                expected_count,
                "only source actor's abilities"
            );
            for (spell, times) in lanes {
                let actual: Vec<_> = attempts.iter().filter(|a| a.spell == spell).collect();
                assert_eq!(actual.len(), times.len(), "ongoing source lane{spell}");
                for (attempt, time) in actual.iter().zip(times.iter()) {
                    assert!(attempt.result.is_ok(), "native admission {attempt:?}");
                    assert!(
                        (attempt.time - *time).abs() <= 0.15,
                        "ordered repeat/native casting {attempt:?} expected{time}"
                    );
                }
                assert_eq!(
                    cast_count(&h, boss, spell),
                    times.len(),
                    "real native completions{spell}"
                );
            }
            assert!(
                h.inbox(1)
                    .spell_gos
                    .iter()
                    .all(|g| g.caster != boss.to_bits())
            );
            assert_eq!(
                h.app
                    .world()
                    .resource::<crate::maps::InstanceManager>()
                    .completed_encounters(map),
                0,
                "one unsurrendered actor cannot credit council"
            );
        }
    }
}

#[test]
fn encounter_coverage_lost_dwarves_threshold_stops_casts_and_becomes_friendly() {
    let (mut h, boss, map) = dwarf_copy(DWARVES[0], 205);
    let flags = *h.app.world().get::<UnitFlags>(boss).unwrap();
    h.engage(boss);
    h.hit_pct(boss, 89.0);
    h.advance(12.0);
    assert_eq!(cast_count(&h, boss, 369563), 1, "above10% still fights");
    assert!(h.health(boss).current > h.health(boss).max * 0.1);
    h.hit_pct(boss, 200.0);
    h.advance(0.1);
    assert!(h.health(boss).current > 0.0 && h.health(boss).current <= h.health(boss).max * 0.1);
    assert!(
        !h.app.world().get::<CombatStatus>(boss).unwrap().0,
        "surrender exits combat"
    );
    assert_eq!(*h.app.world().get::<UnitFlags>(boss).unwrap(), flags);
    assert_eq!(
        *h.app.world().get::<UnitFactionTemplate>(boss).unwrap(),
        UnitFactionTemplate(35)
    );
    assert!(!crate::spell_cast::targets::relation_matches(
        h.app.world(),
        h.player(),
        boss,
        crate::spell_cast::targets::Relation::Enemy,
    ));
    let before = h.inbox(0).spell_gos.len();
    h.advance(60.0);
    assert_eq!(
        h.inbox(0).spell_gos.len(),
        before,
        "surrendered actor remains silent after becoming friendly"
    );
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(boss)
            .is_none()
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0
    );
}

#[test]
fn encounter_coverage_lost_dwarves_surrender_stops_garrick_and_player_attacks() {
    let (mut h, boss, _) = dwarf_copy(DWARVES[0], 205);
    let owner = h.player();
    let garrick = crate::followers::native_fixture::spawn(h.app.world_mut(), owner, 209057)
        .expect("authored Garrick follower");
    h.app.add_systems(FixedUpdate, crate::followers::update_ai);
    h.engage(boss);
    crate::combat::start_attack_now(
        h.app.world_mut(),
        garrick,
        crate::combat::AutoAttack::new(boss),
    );
    h.hit_pct(boss, 200.0);
    h.advance(0.1);
    assert!(!crate::spell_cast::targets::relation_matches(
        h.app.world(),
        owner,
        boss,
        crate::spell_cast::targets::Relation::Enemy,
    ));
    assert!(!crate::spell_cast::targets::relation_matches(
        h.app.world(),
        garrick,
        boss,
        crate::spell_cast::targets::Relation::Enemy,
    ));
    let log_start = h.inbox(0).log.len();
    h.advance(6.0);
    assert!(
        h.inbox(0).log[log_start..].iter().all(|event| {
            event.source != Some(garrick.to_bits())
                || event.target != Some(boss.to_bits())
                || event.kind != CombatLogKind::Damage
        }),
        "Garrick must not resume damaging the friendly dwarf"
    );
}

#[test]
fn encounter_coverage_lost_dwarves_surrender_removes_garrick_threat_and_preserves_other_combat() {
    let (mut h, boss, map) = dwarf_copy(DWARVES[0], 205);
    let owner = h.player();
    let garrick = crate::followers::native_fixture::spawn(h.app.world_mut(), owner, 209057)
        .expect("authored Garrick follower");
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (other, _) = spawn_authored_actor(&mut h.app, &conn, DWARVES[1], map, 205);
    crate::creature_aggro::add_threat(h.app.world_mut(), garrick, boss, 100.0);
    crate::creature_aggro::add_threat(h.app.world_mut(), garrick, other, 80.0);
    assert!(crate::combat_refs::has_active_reference_to(
        h.app.world(),
        garrick,
        boss
    ));
    assert!(
        h.app
            .world()
            .get::<ThreatTable>(garrick)
            .unwrap()
            .entries()
            .iter()
            .any(|entry| entry.entity == boss.to_bits())
    );
    h.engage(boss);
    h.hit_pct(boss, 200.0);
    h.advance(0.1);
    assert!(!crate::combat_refs::has_active_reference_to(
        h.app.world(),
        garrick,
        boss
    ));
    assert!(
        !h.app
            .world()
            .get::<ThreatTable>(garrick)
            .unwrap()
            .entries()
            .iter()
            .any(|entry| entry.entity == boss.to_bits())
    );
    assert!(crate::combat_refs::has_active_reference_to(
        h.app.world(),
        garrick,
        other
    ));
    assert!(
        h.app
            .world()
            .get::<ThreatTable>(garrick)
            .unwrap()
            .entries()
            .iter()
            .any(|entry| entry.entity == other.to_bits())
    );
    assert!(h.app.world().get::<CombatStatus>(garrick).unwrap().0);
    crate::combat::combat_stop(h.app.world_mut(), other);
    assert!(!h.app.world().get::<CombatStatus>(garrick).unwrap().0);
}

#[test]
fn encounter_coverage_lost_dwarves_evade_restores_partial_surrender_progress() {
    let (mut h, baelog, map) = dwarf_copy(DWARVES[0], 205);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (eric, _) = spawn_authored_actor(&mut h.app, &conn, DWARVES[1], map, 205);
    let original_faction = *h.app.world().get::<UnitFactionTemplate>(baelog).unwrap();
    h.engage(baelog);
    h.hit_pct(baelog, 200.0);
    h.advance(0.1);
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(baelog)
            .is_none()
    );
    assert!(!h.app.world().get::<CombatStatus>(baelog).unwrap().0);
    h.engage(eric);
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), eric);
    h.advance(14.0);
    assert_eq!(
        *h.app.world().get::<UnitFactionTemplate>(baelog).unwrap(),
        original_faction,
        "a wipe restores the sourced hostile faction"
    );
    assert!(
        cast_count(&h, baelog, 369563) > 0,
        "native home/health recovery and reset restore unsurrendered combat clocks"
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0,
        "partial surrender cannot survive wipe as completed credit"
    );
}

#[test]
fn encounter_coverage_lost_dwarves_reset_excludes_old_surrender_from_new_credit() {
    let (mut h, baelog, map) = dwarf_copy(DWARVES[0], 205);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (eric, _) = spawn_authored_actor(&mut h.app, &conn, DWARVES[1], map, 205);
    let (olaf, _) = spawn_authored_actor(&mut h.app, &conn, DWARVES[2], map, 205);
    h.engage(baelog);
    h.hit_pct(baelog, 200.0);
    h.advance(0.1);
    h.engage(eric);
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), eric);
    h.advance(0.1);
    for actor in [eric, olaf] {
        h.engage(actor);
        h.hit_pct(actor, 200.0);
        h.advance(0.1);
    }
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0,
        "previous-pull surrender cannot complete a new pull"
    );
    h.advance(2.0);
    assert!(
        h.health(baelog).current > h.health(baelog).max * 0.1,
        "actual native regeneration, not injected healing"
    );
    h.engage(baelog);
    h.hit_pct(baelog, 200.0);
    h.advance(0.1);
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        1,
        "fresh third surrender completes council"
    );
    for actor in [baelog, eric, olaf] {
        assert!(
            h.app
                .world()
                .get::<crate::combat::DeathState>(actor)
                .is_none()
        );
    }
}
