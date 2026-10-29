//! LOG_TIMERS_FROM_MYTHIC_PLUS, ERKHART_181089_MEDIAN_DELAY and joint death.
use super::*;

const KYRAKKA: u32 = 190484;
const ERKHART: u32 = 190485;
const MAP: u32 = 2521;
const BIT: u32 = 2;

fn ruby_copy(difficulty: u32) -> (Harness, Entity, Entity, InMap) {
    let (mut h, erkhart, map) = bound_boss_at_difficulty(ERKHART, MAP, BIT, difficulty, None);
    let spawn = prepared_inputs::encounter_inputs(KYRAKKA, MAP, false).authored_spawn(difficulty);
    let (kyrakka, _) = spawn_prepared_actor(&mut h.app, spawn, map, difficulty);
    let wdt = h.app.world().resource::<MapStore>().maps[&MAP].wdt_file_data_id;
    h.app.insert_resource(crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        std::collections::HashMap::from([(MAP, wdt)]),
    ));
    for actor in [erkhart, kyrakka] {
        let mut visibility = crate::interest::NetworkVisibility::default();
        visibility.gain_visibility(h.players[0].1);
        h.app.world_mut().entity_mut(actor).insert(visibility);
    }
    (h, erkhart, kyrakka, map)
}

#[test]
fn ruby_final_joint_death_grants_only_one_copy_credit() {
    for difficulty in [1, 205] {
        let (mut h, erkhart, kyrakka, map) = ruby_copy(difficulty);
        h.engage(erkhart);
        h.engage(kyrakka);
        h.advance(0.2);
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|start| start.encounter_id == 2623)
        );
        h.hit_pct(erkhart, 200.0);
        h.advance(0.1);
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            0
        );
        assert!(h.inbox(0).ends.iter().all(|end| !end.success));
        h.hit_pct(kyrakka, 200.0);
        h.advance(0.1);
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            1 << BIT
        );
        assert_eq!(
            h.inbox(0)
                .ends
                .iter()
                .filter(|end| end.encounter_id == 2623 && end.success)
                .count(),
            1
        );
        assert!(h.inbox(1).ends.is_empty());
    }
}

#[test]
fn ruby_final_erkhart_logged_clocks() {
    for difficulty in [1, 205] {
        let (mut h, erkhart, kyrakka, _) = ruby_copy(difficulty);
        let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
        h.app
            .insert_resource(super::cast_admission::CastAdmissions {
                actor: erkhart,
                origin,
                entries: Vec::new(),
            });
        h.engage(erkhart);
        h.engage(kyrakka);
        h.advance(55.0);
        let attempts = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        for (spell, first) in [(381512, 6.098), (381517, 10.991), (381516, 21.880)] {
            let casts: Vec<_> = attempts
                .iter()
                .filter(|attempt| attempt.spell == spell)
                .collect();
            assert!(casts.len() >= 2, "repeat lane{spell}: {attempts:?}");
            assert!(
                (casts[0].time - first).abs() < 0.2,
                "first lane{spell}: {attempts:?}"
            );
            assert!(
                casts.iter().all(|attempt| attempt.result.is_ok()),
                "native admission{spell}: {attempts:?}"
            );
        }
    }
}

#[test]
fn ruby_final_logged_clocks_and_stage_anchor() {
    for difficulty in [1, 205] {
        let (mut h, erkhart, kyrakka, _) = ruby_copy(difficulty);
        let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
        h.app
            .insert_resource(super::cast_admission::CastAdmissions {
                actor: kyrakka,
                origin,
                entries: Vec::new(),
            });
        h.engage(erkhart);
        h.engage(kyrakka);
        h.advance(1.0);
        for _ in 0..20 {
            if h.health(erkhart).current <= h.health(erkhart).max * 0.5 {
                break;
            }
            h.hit_pct(erkhart, 5.0);
        }
        let health = h.health(erkhart);
        assert!(
            health.current > 0.0 && health.current <= health.max * 0.5,
            "real damage must cross phase threshold: {health:?}"
        );
        h.advance(1.0);
        let movement = h
            .app
            .world()
            .get::<crate::scripted_ai::movement::PointMovement>(kyrakka)
            .expect("stage uses ordinary point movement");
        assert!(
            (movement.target.x - 1798.47).abs() < 0.01 && (movement.target.z + 36.11).abs() < 0.01
        );
        h.advance(37.5);
        let attempts = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        assert!(
            attempts
                .iter()
                .any(|a| a.spell == 381525 && (a.time - 2.713).abs() < 0.2)
        );
        assert!(
            attempts
                .iter()
                .any(|a| a.spell == 381602 && (a.time - 15.608).abs() < 0.2)
        );
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::CastSuccess
                    && event.spell_id == Some(181089)
                    && event.source == Some(erkhart.to_bits())),
            "actual Erkhart stage event"
        );
        assert!(
            !attempts.iter().any(|a| a.spell == 381605),
            "stage2 must wait for its anchor delay"
        );
        h.advance(2.0);
        eprintln!(
            "stage: position={:?} movement={:?} erkhart_health={:?} kyrakka_state={:?}",
            h.app.world().get::<Position>(kyrakka),
            h.app
                .world()
                .get::<crate::scripted_ai::movement::PointMovement>(kyrakka),
            h.app.world().get::<Health>(erkhart),
            h.app
                .world()
                .get::<crate::creature_ai::CreatureState>(kyrakka)
        );
        let attempts = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        assert!(
            attempts
                .iter()
                .any(|a| a.spell == 381605 && a.result.is_ok()),
            "stage2 native cast after181089: {attempts:?}"
        );
    }
}
