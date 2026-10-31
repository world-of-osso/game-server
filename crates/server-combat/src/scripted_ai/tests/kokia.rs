//! Current Retail LittleWigs2777a9d Kokia.lua + actual70448 native targets/effects.
use super::*;

const KOKIA: u32 = 189232;
const MAP: u32 = 2521;
const BIT: u32 = 0;
const SEARING_BLOWS: u32 = 372858;

fn kokia_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(KOKIA, MAP, BIT, difficulty, None);
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
        .filter(|go| go.caster == boss.to_bits() && go.spell_id == spell)
        .count()
}

#[test]
fn boss_alerts_kokia_real_schedule_and_actual_cast_without_imported_clock() {
    use shared::protocol::EncounterTimelineEvent;
    let (mut h, boss, _) = kokia_copy(205);
    h.engage(boss);
    h.advance(0.1);
    let timers = h
        .inbox(0)
        .timeline
        .iter()
        .find_map(|event| match event {
            EncounterTimelineEvent::Timers { caster, timers, .. } if *caster == boss.to_bits() => {
                Some(timers)
            }
            _ => None,
        })
        .expect("actual Kokia schedule must publish without an imported addon catalog");
    assert_eq!(timers.len(), 3);
    for (spell, duration) in [(372864, 8502), (372110, 19431), (SEARING_BLOWS, 28953)] {
        let timer = timers.iter().find(|timer| timer.spell_id == spell).unwrap();
        assert_eq!(timer.duration_ms, duration);
        assert!((duration - 100..=duration).contains(&timer.remaining_ms));
    }
    assert!(
        !h.inbox(0)
            .timeline
            .iter()
            .any(|event| matches!(event, EncounterTimelineEvent::Cast { .. }))
    );
    h.advance(1.0);
    let remaining = h
        .inbox(0)
        .timeline
        .iter()
        .rev()
        .find_map(|event| match event {
            EncounterTimelineEvent::Timers { timers, .. } => timers
                .iter()
                .find(|timer| timer.spell_id == SEARING_BLOWS)
                .map(|timer| timer.remaining_ms),
            _ => None,
        })
        .unwrap();
    assert!(
        (27853..=27953).contains(&remaining),
        "snapshot reads advancing real request clock"
    );
    h.advance(32.0);
    assert!(cast_count(&h, boss, SEARING_BLOWS) > 0);
    assert!(h.inbox(0).timeline.iter().any(|event| matches!(event,
        EncounterTimelineEvent::Cast { caster, spell_id: SEARING_BLOWS, severity: shared::protocol::EncounterSeverity::Medium, warning_text }
        if *caster == boss.to_bits() && warning_text == "Searing Blows (1)")));
    assert!(h.inbox(0).timeline.iter().any(|event| matches!(event,
        EncounterTimelineEvent::Timers { timers, .. }
        if timers.iter().any(|timer| timer.spell_id == 372864 && timer.duration_ms == 40088))));
    assert!(
        h.inbox(1).timeline.is_empty(),
        "other copy cannot receive timers or warnings"
    );
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    h.advance(0.1);
    assert!(!h.inbox(0).ends.last().unwrap().success);
    let count = h.inbox(0).timeline.len();
    let pulls = h.inbox(0).starts.len();
    let player = h.player();
    h.app.world_mut().get_mut::<Position>(player).unwrap().x += 1000.0;
    h.advance(10.0);
    assert_eq!(
        h.inbox(0).starts.len(),
        pulls,
        "distant player cannot start another pull"
    );
    assert_eq!(
        h.inbox(0).timeline.len(),
        count,
        "reset retires actual schedule alerts"
    );
}

#[test]
fn boss_alerts_opted_lua_scripts_publish_real_initial_requests_only() {
    use shared::protocol::EncounterTimelineEvent;
    for (entry, map, ordinal, expected) in [
        (188252, 2521, 1, &[(1307297, 5000), (1307308, 15000)][..]),
        (
            209230,
            2648,
            0,
            &[(1214325, 5100), (474018, 9400), (444123, 16000)][..],
        ),
        (
            190609,
            2526,
            3,
            &[
                (373326, 7000),
                (1282251, 9000),
                (374343, 14000),
                (388822, 28000),
            ][..],
        ),
        (
            186739,
            2515,
            1,
            &[(384223, 3300), (372222, 5200), (385578, 10600)][..],
        ),
        (186738, 2515, 3, &[(384978, 7400), (384699, 12200)][..]),
    ] {
        let (mut h, boss, _) = bound_boss_at_difficulty(entry, map, ordinal, 205, None);
        h.engage(boss);
        h.advance(0.1);
        let timers = h
            .inbox(0)
            .timeline
            .iter()
            .find_map(|event| match event {
                EncounterTimelineEvent::Timers { timers, .. } => Some(timers),
                _ => None,
            })
            .unwrap_or_else(|| panic!("Lua-backed entry{entry} must publish its actual requests"));
        assert_eq!(
            timers.len(),
            expected.len(),
            "entry{entry}: no guessed extra mechanics"
        );
        for &(spell, duration) in expected {
            let timer = timers
                .iter()
                .find(|timer| timer.spell_id == spell)
                .unwrap_or_else(|| panic!("entry{entry} missing spell{spell}"));
            assert_eq!(timer.duration_ms, duration);
            assert!((duration - 100..=duration).contains(&timer.remaining_ms));
        }
        assert!(h.inbox(1).timeline.is_empty());
    }
}

#[test]
fn encounter_coverage_kokia_current_retail_inputs_qualify() {
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    for (spell, expected) in [
        (372863, (3, 0, 1309540)),
        (372110, (3, 0, 372107)),
        (SEARING_BLOWS, (6, 23, 372859)),
    ] {
        let actual:(u32,u32,u32)=conn.query_row("SELECT Effect,EffectAura,EffectTriggerSpell FROM spell_effect WHERE SpellID=? AND DifficultyID=0 AND EffectIndex=0",[spell],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
        assert_eq!(actual, expected, "current native effect/child{spell}");
    }
    assert_eq!(
        conn.query_row(
            "SELECT MapID,Bit FROM dungeon_encounter WHERE ID=2606",
            [],
            |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?))
        )
        .unwrap(),
        (MAP, BIT)
    );
}

#[test]
fn encounter_coverage_kokia_ritual_summons_after_success() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = kokia_copy(difficulty);
        let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
        h.app
            .insert_resource(super::cast_admission::CastAdmissions {
                actor: boss,
                origin,
                entries: Vec::new(),
            });
        h.engage(boss);
        h.advance(12.5);
        let count = |world: &mut World| {
            world
                .query::<(&shared::components::Npc, &InMap)>()
                .iter(world)
                .filter(|(npc, actor_map)| npc.template_id == 189886 && **actor_map == map)
                .count()
        };
        assert_eq!(count(h.app.world_mut()), 0, "ritual has not completed");
        h.advance(2.0);
        eprintln!(
            "Ritual admissions: {:?}",
            h.app
                .world()
                .resource::<super::cast_admission::CastAdmissions>()
                .entries
        );
        assert_eq!(
            count(h.app.world_mut()),
            1,
            "successful ritual summons native Firestorm"
        );
    }
}

#[test]
fn encounter_coverage_kokia_missing_energy_pool_does_not_create_one() {
    let (mut h, boss, _) = kokia_copy(205);
    h.app
        .world_mut()
        .entity_mut(boss)
        .remove::<crate::spell_cast::powers::PowerPools>();
    h.engage(boss);
    h.advance(1.0);
    assert!(
        h.app
            .world()
            .get::<crate::spell_cast::powers::PowerPools>(boss)
            .is_none()
    );
}

#[test]
fn encounter_coverage_kokia_logged_energy_refills_after_ritual() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = kokia_copy(difficulty);
        h.engage(boss);
        h.advance(14.0);
        let energy = |world: &World| {
            crate::spell_cast::powers::power_value(
                world,
                boss,
                shared::components::PowerType::Energy,
            )
            .unwrap()
            .0
        };
        let before = energy(h.app.world());
        h.advance(5.0);
        let gained = energy(h.app.world()) - before;
        assert!(
            (gained - 14.32).abs() < 0.3,
            "logged refill difficulty{difficulty}: {gained}"
        );
    }
}

#[test]
fn encounter_coverage_kokia_interrupted_ritual_does_not_summon() {
    let (mut h, boss, _) = kokia_copy(205);
    h.engage(boss);
    h.advance(9.0);
    assert_eq!(
        h.app.world().get::<CastState>(boss).unwrap().spell_id,
        372863
    );
    crate::spell_cast::cancel_cast(h.app.world_mut(), boss);
    h.advance(6.0);
    assert!(h.creatures_of(189886).is_empty());
}

#[test]
fn encounter_coverage_kokia_native_searing_periodic_weapon_damage_qualifies() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = kokia_copy(difficulty);
        let player = h.player();
        h.engage(boss);
        crate::spell_cast::creature_cast(
            h.app.world_mut(),
            boss,
            SEARING_BLOWS,
            Some(player),
            false,
        )
        .expect("ordinary native372858 admission");
        h.advance(3.5);
        assert_eq!(cast_count(&h, boss, SEARING_BLOWS), 1);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.amount > 0
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(372859)),
            "real native aura23→372859 weapon damage in difficulty{difficulty}"
        );
        assert!(!h.has_aura(h.players[1].0, SEARING_BLOWS));
    }
}

#[test]
fn encounter_coverage_kokia_first_searing_blows_follows_current_retail_clock() {
    let (mut h, boss, _) = kokia_copy(205);
    h.engage(boss);
    h.advance(32.0);
    assert_eq!(
        cast_count(&h, boss, SEARING_BLOWS),
        1,
        "current Retail28s ability request, not legacy21.6s"
    );
}

#[test]
fn encounter_coverage_kokia_current_lanes_native_damage_and_isolated_credit() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = kokia_copy(difficulty);
        let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
        h.app
            .insert_resource(super::cast_admission::CastAdmissions {
                actor: boss,
                origin,
                entries: Vec::new(),
            });
        h.engage(boss);
        h.advance(140.0);
        let attempts = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        eprintln!("KOKIA difficulty{difficulty} actual native requests={attempts:?}");
        let lanes = [
            (372863, &[8.502, 48.590, 88.678, 128.766][..]),
            (
                372107,
                &[19.431, 40.081, 60.731, 81.381, 102.031, 122.681][..],
            ),
            (SEARING_BLOWS, &[28.953, 69.081, 109.209][..]),
        ];
        assert_eq!(
            attempts
                .iter()
                .filter(|attempt| lanes.iter().any(|(spell, _)| *spell == attempt.spell))
                .count(),
            13,
            "only logged native source lanes"
        );
        for (spell, expected) in lanes {
            let casts: Vec<_> = attempts
                .iter()
                .filter(|attempt| attempt.spell == spell)
                .collect();
            assert_eq!(
                casts.len(),
                expected.len(),
                "ongoing native source lane{spell}"
            );
            for (attempt, time) in casts.iter().zip(expected) {
                assert!(
                    attempt.result.is_ok(),
                    "ordinary native admission{attempt:?}"
                );
                assert!(
                    (attempt.time - *time).abs() <= 0.75,
                    "currentRetail source-order lane{spell}: {attempt:?} expected{time}"
                );
            }
            assert_eq!(
                cast_count(&h, boss, spell),
                expected.len(),
                "actual native completions{spell}"
            );
        }
        let player = h.player();
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.amount > 0
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(372859)),
            "real scheduled aura23 weapon damage"
        );
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|start| start.encounter_id == 2606 && start.difficulty_id == difficulty)
        );
        assert!(h.inbox(1).starts.is_empty());
        assert!(
            h.inbox(1)
                .spell_gos
                .iter()
                .all(|go| go.caster != boss.to_bits())
        );
        h.hit_pct(boss, 200.0);
        h.advance(0.1);
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, BIT),
            Some(EncounterState::Done)
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            1 << BIT,
            "Kokia only, no Melidrussa bit"
        );
        assert_eq!(
            h.inbox(0)
                .ends
                .iter()
                .filter(|end| end.encounter_id == 2606 && end.success)
                .count(),
            1
        );
        assert!(h.inbox(1).ends.is_empty());
        let before = cast_count(&h, boss, SEARING_BLOWS)
            + cast_count(&h, boss, 372107)
            + cast_count(&h, boss, 372863);
        h.advance(45.0);
        assert_eq!(
            cast_count(&h, boss, SEARING_BLOWS)
                + cast_count(&h, boss, 372107)
                + cast_count(&h, boss, 372863),
            before,
            "corpse silence"
        );
    }
}

#[test]
fn encounter_coverage_kokia_evade_rearms_current_source_lanes() {
    let (mut h, boss, map) = kokia_copy(205);
    h.engage(boss);
    h.advance(32.0);
    assert_eq!(cast_count(&h, boss, SEARING_BLOWS), 1);
    assert_eq!(cast_count(&h, boss, 372107), 1);
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, BIT),
        Some(EncounterState::NotStarted)
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0
    );
    h.engage(boss);
    h.advance(27.8);
    assert_eq!(
        cast_count(&h, boss, SEARING_BLOWS),
        1,
        "not before currentRetail28s"
    );
    h.advance(4.0);
    assert_eq!(cast_count(&h, boss, SEARING_BLOWS), 2);
    h.advance(11.0);
    assert_eq!(
        cast_count(&h, boss, 372107),
        3,
        "reset resumes19 then20 source durations/native2.5s casts"
    );
}

/// Spawn the copy's real authored game objects and return its Fire Wall's state and collision.
fn spawn_fire_wall(h: &mut Harness, map: InMap) -> (u8, bool) {
    let canonical = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let objects = crate::game_objects::load_game_object_spawns(&canonical).unwrap();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, h.app.world());
    let content = crate::zones::MapContent {
        map,
        difficulty_id: 205,
        dead_bosses: Vec::new(),
    };
    crate::maps::transfer::queue_map_content(&mut commands, None, Some(&objects), content, None);
    queue.apply(h.app.world_mut());
    fire_wall(h, map)
}

fn fire_wall(h: &mut Harness, map: InMap) -> (u8, bool) {
    let mut walls = h.app.world_mut().query::<(
        &shared::protocol::GameObjectInfo,
        &InMap,
        &crate::game_objects::GameObjectState,
        &shared::protocol::GameObjectCollision,
    )>();
    let walls: Vec<_> = walls
        .iter(h.app.world())
        .filter(|(info, copy, ..)| info.entry == 377194 && **copy == map)
        .map(|(_, _, state, collision)| (state.0, collision.0))
        .collect();
    assert_eq!(walls.len(), 1, "one authored Fire Wall per copy");
    walls[0]
}

fn fire_wall_animation(h: &mut Harness, map: InMap) -> shared::protocol::GameObjectAnimation {
    let mut walls = h.app.world_mut().query::<(
        &shared::protocol::GameObjectInfo,
        &InMap,
        &shared::protocol::GameObjectAnimation,
    )>();
    *walls
        .iter(h.app.world())
        .find(|(info, copy, _)| info.entry == 377194 && **copy == map)
        .expect("fire wall animation")
        .2
}

#[test]
fn kokia_fire_wall_opens_only_when_kokia_is_done() {
    let (mut h, _, map) = kokia_copy(205);
    assert_eq!(
        spawn_fire_wall(&mut h, map),
        (1, true),
        "closed before Kokia"
    );
    instance_script::set_boss_state(h.app.world_mut(), map, BIT, EncounterState::InProgress);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, BIT),
        Some(EncounterState::InProgress)
    );
    assert_eq!(fire_wall(&mut h, map), (1, true), "closed while engaged");
    assert!(instance_script::set_boss_state(
        h.app.world_mut(),
        map,
        BIT,
        EncounterState::Done
    ));
    assert_eq!(
        fire_wall(&mut h, map),
        (0, false),
        "open, without collision"
    );
    assert_eq!(
        fire_wall_animation(&mut h, map),
        shared::protocol::GameObjectAnimation {
            animation_id: 148,
            looping: false,
        }
    );
}

#[test]
fn kokia_fire_wall_spawns_open_in_a_copy_with_kokia_done() {
    let (mut h, _, map) = kokia_copy(205);
    assert!(instance_script::set_boss_state(
        h.app.world_mut(),
        map,
        BIT,
        EncounterState::Done
    ));
    assert_eq!(spawn_fire_wall(&mut h, map), (0, false));
    assert_eq!(
        fire_wall_animation(&mut h, map),
        shared::protocol::GameObjectAnimation {
            animation_id: 149,
            looping: true,
        }
    );
}
