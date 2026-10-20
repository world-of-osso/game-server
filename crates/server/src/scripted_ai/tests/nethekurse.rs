//! TC a352b1fa boss_nethekurse.cpp; authentic map540 / encounter1936 bit1.
use super::*;

const NETHEKURSE: u32 = 16807;
const SHATTERED_HALLS: u32 = 540;
const NETHEKURSE_BIT: u32 = 1;

fn nethekurse_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(
        NETHEKURSE,
        SHATTERED_HALLS,
        NETHEKURSE_BIT,
        difficulty,
        None,
    );
    let directory = crate::ground::ground_data_dir();
    let wdt = h.app.world().resource::<MapStore>().maps[&SHATTERED_HALLS].wdt_file_data_id;
    h.app.insert_resource(crate::ground::GroundMap::new(
        &directory,
        std::collections::HashMap::from([(SHATTERED_HALLS, wdt)]),
    ));
    h.app.insert_resource(
        crate::line_of_sight::LineOfSight::load(&directory.join("los"))
            .expect("authentic eye tables"),
    );
    // HARNESS SETUP: move the isolated entrant into range only after fixture startup.
    // Like the landed source fixtures, no autonomous pull may precede observation.
    let position = *h.app.world().get::<Position>(boss).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: position.x + 3.0,
        ..position
    });
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    (h, boss, map)
}

fn nethekurse_yells(h: &Harness, boss: Entity, group: u8) -> usize {
    let texts = &h.app.world().resource::<CreatureTexts>().groups[&NETHEKURSE][&group];
    h.inbox(0)
        .chat
        .iter()
        .filter(|line| {
            line.channel == shared::protocol::ChatType::MonsterYell(boss.to_bits())
                && texts
                    .iter()
                    .any(|text| line.content == text.male_text || line.content == text.female_text)
        })
        .count()
}

#[test]
fn encounter_coverage_nethekurse_aggro_all_alive() {
    for difficulty in [1, 2] {
        let (mut h, boss, map) = nethekurse_copy(difficulty);
        h.engage(boss);
        h.advance(0.5);
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, NETHEKURSE_BIT),
            Some(EncounterState::InProgress),
        );
        assert_eq!(
            nethekurse_yells(&h, boss, 3),
            1,
            "source SAY_AGGRO_ALL_ALIVE"
        );
        assert!(h.inbox(1).chat.is_empty(), "other-copy silence");
    }
}

fn observe_admissions(h: &mut Harness, boss: Entity) {
    let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
    h.app
        .insert_resource(crate::scripted_ai::tests::cast_admission::CastAdmissions {
            actor: boss,
            origin,
            entries: Vec::new(),
        });
    h.app.world_mut().entity_mut(boss).insert(
        crate::scripted_ai::tests::cast_admission::GroundCombatSeed(0),
    );
}

fn requested_times(h: &Harness, spell: u32) -> Vec<f32> {
    h.app
        .world()
        .resource::<crate::scripted_ai::tests::cast_admission::CastAdmissions>()
        .entries
        .iter()
        .filter(|entry| entry.spell == spell)
        .map(|entry| entry.time)
        .collect()
}

fn assert_request_clock(times: &[f32], first: (f32, f32), repeat: (f32, f32)) {
    assert!(times.len() >= 3, "three source requests: {times:?}");
    assert!(
        times[0] >= first.0 && times[0] <= first.1 + 0.15,
        "first: {times:?}"
    );
    assert!(
        times.windows(2).all(|pair| {
            let delta = pair[1] - pair[0];
            delta >= repeat.0 - 0.01 && delta <= repeat.1 + 0.1
        }),
        "repeat: {times:?}"
    );
}

#[test]
fn encounter_coverage_nethekurse_native_inputs_qualify() {
    for difficulty in [1, 2] {
        let (h, boss, _) = nethekurse_copy(difficulty);
        assert_eq!(
            h.app
                .world()
                .get::<shared::components::Npc>(boss)
                .unwrap()
                .template_id,
            NETHEKURSE
        );
        let store = h.app.world().resource::<SpellStore>();
        for spell in [30496, 30500, 30495, 35953, 30502, 30505, 30508] {
            assert!(store.get(spell).is_some(), "native70448 spell{spell}");
        }
        for spell in [30497, 35952] {
            assert!(
                store.get(spell).is_none(),
                "named Consumption gap{spell}, no replacement"
            );
        }
        let texts = &h.app.world().resource::<CreatureTexts>().groups[&NETHEKURSE];
        assert_eq!(
            (0..=9).map(|group| texts[&group].len()).collect::<Vec<_>>(),
            [1, 1, 1, 1, 1, 1, 4, 3, 4, 1]
        );
    }
}

#[test]
fn encounter_coverage_nethekurse_rotation_and_native_damage() {
    for difficulty in [1, 2] {
        let (mut h, boss, _) = nethekurse_copy(difficulty);
        observe_admissions(&mut h, boss);
        h.engage(boss);
        h.advance(65.0);
        let cleave = if difficulty == 2 { 35953 } else { 30495 };
        assert_request_clock(&requested_times(&h, 30496), (8.0, 8.0), (8.0, 8.0));
        assert_request_clock(&requested_times(&h, 30500), (10.0, 15.0), (10.0, 20.0));
        assert_request_clock(&requested_times(&h, cleave), (15.0, 20.0), (15.0, 20.0));
        let other_cleave = if difficulty == 2 { 30495 } else { 35953 };
        assert!(
            requested_times(&h, other_cleave).is_empty(),
            "difficulty-specific ability"
        );
        assert!(requested_times(&h, 30502).is_empty(), "no spin above25%");
        let player = h.player();
        assert!(
            h.inbox(0).log.iter().any(|event| {
                event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(cleave)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            }),
            "native ordinary cleave/slam actually damages"
        );
        assert!(h.inbox(1).spell_gos.is_empty(), "other-copy isolation");
        eprintln!(
            "NETHEKURSE difficulty{difficulty} admissions={:?}",
            h.app
                .world()
                .resource::<crate::scripted_ai::tests::cast_admission::CastAdmissions>()
                .entries
        );
    }
}

#[test]
fn encounter_coverage_nethekurse_dark_spin_strict_crossing_once() {
    let (mut h, boss, _) = nethekurse_copy(1);
    observe_admissions(&mut h, boss);
    h.engage(boss);
    h.advance(0.5);
    h.hit_pct(boss, 74.0);
    h.advance(0.2);
    assert!(
        requested_times(&h, 30502).is_empty(),
        "26% remains above threshold"
    );
    h.hit_pct(boss, 2.0);
    h.advance(0.2);
    assert!(
        h.health(boss).current < h.health(boss).max * 0.25,
        "real damage crosses below25%"
    );
    assert_eq!(
        requested_times(&h, 30502).len(),
        1,
        "one incoming-damage event"
    );
    assert!(h.has_aura(boss, 30502), "ordinary native Dark Spin aura");
    h.hit_pct(boss, 1.0);
    h.advance(5.0);
    assert_eq!(
        requested_times(&h, 30502).len(),
        1,
        "no repeated low-health trigger"
    );
    assert!(
        h.inbox(0).log.iter().any(|event| {
            event.source == Some(boss.to_bits())
                && [Some(30505), Some(30508)].contains(&event.spell_id)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }),
        "native Dark Spin periodic child actually damages"
    );
}

#[test]
fn encounter_coverage_nethekurse_death_credit_and_silent_corpse() {
    for difficulty in [1, 2] {
        let (mut h, boss, map) = nethekurse_copy(difficulty);
        h.engage(boss);
        h.advance(1.0);
        h.hit_pct(boss, 200.0);
        h.advance(0.2);
        assert_eq!(nethekurse_yells(&h, boss, 9), 1, "JustDied Talk9");
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, NETHEKURSE_BIT),
            Some(EncounterState::Done)
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            1 << NETHEKURSE_BIT
        );
        assert_eq!(
            h.inbox(0)
                .ends
                .iter()
                .filter(|end| end.encounter_id == 1936 && end.success)
                .count(),
            1
        );
        let casts = h.inbox(0).spell_gos.len();
        h.advance(30.0);
        assert_eq!(
            h.inbox(0).spell_gos.len(),
            casts,
            "corpse executes no events"
        );
        assert_eq!(nethekurse_yells(&h, boss, 9), 1);
        assert!(h.inbox(1).chat.is_empty() && h.inbox(1).ends.is_empty());
    }
}

#[test]
fn encounter_coverage_nethekurse_slay_player_and_npc() {
    for player_victim in [true, false] {
        let (mut h, boss, map) = nethekurse_copy(1);
        super::omor_slay::install_player_death_transition(&mut h);
        h.engage(boss);
        h.advance(0.5);
        let victim = if player_victim {
            h.player()
        } else {
            let conn =
                Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .unwrap();
            spawn_authored_actor(&mut h.app, &conn, NETHEKURSE, map, 1).0
        };
        // HARNESS SETUP: production explicit Unit::Kill, not a fabricated callback.
        crate::combat::kill_unit(h.app.world_mut(), boss, victim);
        h.advance(0.5);
        assert_eq!(
            nethekurse_yells(&h, boss, 8),
            1,
            "source KilledUnit is unfiltered"
        );
        h.advance(0.5);
        assert_eq!(nethekurse_yells(&h, boss, 8), 1, "no replay");
        assert!(h.inbox(1).chat.is_empty());
    }
}

fn populated_nethekurse(difficulty: u32) -> (Harness, Entity, InMap, Vec<Entity>) {
    let (mut h, _, map) = encounter_fixture(
        NETHEKURSE,
        SHATTERED_HALLS,
        NETHEKURSE_BIT,
        difficulty,
        None,
        EncounterState::NotStarted,
        true,
    );
    let boss = h.creatures_of(NETHEKURSE)[0];
    let peons = h
        .creatures_of(17083)
        .into_iter()
        .filter(|&actor| {
            h.app
                .world()
                .get::<crate::zones::CreatureStringIds>(actor)
                .is_some_and(|ids| ids.has("FelOrcConvertNethekurseEvent"))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        peons.len(),
        4,
        "four source StringId actors, not all17083 spawns"
    );
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    (h, boss, map, peons)
}

#[test]
#[ignore = "RED: authentic event convert death callbacks not ported"]
fn encounter_coverage_nethekurse_four_event_peons_release_and_all_dead_speech() {
    for difficulty in [1, 2] {
        let (mut h, boss, map, peons) = populated_nethekurse(difficulty);
        for (index, peon) in peons.into_iter().enumerate() {
            // HARNESS SETUP: actual lethal Fireball damage at the authored actor;
            // distant entrant prevents proximity pulls before the fourth death.
            h.hit_pct(peon, 200.0);
            h.advance(0.2);
            if index < 3 {
                assert_eq!(
                    instance_script::get_boss_state(h.app.world_mut(), map, NETHEKURSE_BIT),
                    Some(EncounterState::NotStarted)
                );
            }
        }
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, NETHEKURSE_BIT),
            Some(EncounterState::InProgress),
            "fourth event convert DoZoneInCombat"
        );
        assert_eq!(nethekurse_yells(&h, boss, 5), 1, "SAY_AGGRO_ALL_DEAD");
        assert!(h.inbox(1).starts.is_empty() && h.inbox(1).chat.is_empty());
    }
}

#[test]
#[ignore = "RED: authentic event convert death count not ported"]
fn encounter_coverage_nethekurse_non_event_peons_do_not_count_and_some_alive_speech() {
    let (mut h, boss, _, peons) = populated_nethekurse(1);
    let non_event = h
        .creatures_of(17083)
        .into_iter()
        .find(|actor| !peons.contains(actor))
        .expect("ordinary untagged convert");
    h.hit_pct(non_event, 200.0);
    h.advance(0.2);
    h.hit_pct(peons[0], 200.0);
    h.advance(0.2);
    let player = h.player();
    let position = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: position.x + 3.0,
        ..position
    });
    h.engage(boss);
    h.advance(0.2);
    assert_eq!(nethekurse_yells(&h, boss, 4), 1, "one event convert dead");
    assert_eq!(nethekurse_yells(&h, boss, 3), 0, "not the all-alive line");
    assert_eq!(
        nethekurse_yells(&h, boss, 5),
        0,
        "ordinary convert was not counted"
    );
}
