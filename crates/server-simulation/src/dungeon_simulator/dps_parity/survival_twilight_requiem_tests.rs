//! Actual Moonlight Chakram's final bounce executes selected Twilight Requiem.
use super::marksmanship_mechanics_tests::cast;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_chakram_bounces_once_per_cast_across_pack() {
    // SimC76f8118 sc_hunter.cpp4487–4503: one actual execute schedules the
    // initial hit plus native1264949 effect1 bounces, round-robin over enemies.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(255, 3).unwrap();
    hunter_fixture::provision_pet(run.app.world_mut(), run.actor).unwrap();
    cast(&mut run, 883, true);
    engage(&mut run);
    cast(&mut run, 1250646, false);
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    cast(&mut run, 1264949, false);
    for _ in 0..120 {
        run.app.update();
    }
    let actor = Some(run.actor.to_bits());
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    let per_target: Vec<usize> = run
        .targets
        .iter()
        .map(|target| {
            events
                .iter()
                .filter(|event| {
                    event.source == actor
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(1266081)
                        && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
                })
                .count()
        })
        .collect();
    assert_eq!(
        per_target.iter().sum::<usize>(),
        7,
        "one Chakram is the initial hit plus native6 bounces: {per_target:?}"
    );
    assert!(
        per_target.iter().all(|&hits| hits >= 2),
        "bounces round-robin over the three enemies: {per_target:?}"
    );
}

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_chakram_final_bounce_executes_twilight_requiem_area() {
    // SimC76f8118 sc_hunter.cpp4427–4434/4504–4507: at the final bounce,
    // selected Twilight Requiem executes native1266096 once around the primary.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(255, 3).unwrap();
    hunter_fixture::provision_pet(run.app.world_mut(), run.actor).unwrap();
    cast(&mut run, 883, true);
    engage(&mut run);
    cast(&mut run, 1250646, false);
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    cast(&mut run, 1264949, false);
    for _ in 0..120 {
        run.app.update();
    }
    let actor = Some(run.actor.to_bits());
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    for &target in &run.targets {
        let hits = events
            .iter()
            .filter(|event| {
                event.source == actor
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(1266096)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count();
        assert_eq!(
            hits, 1,
            "one actual Chakram executes Twilight Requiem once on in-radius enemy {target:?}"
        );
    }
}
