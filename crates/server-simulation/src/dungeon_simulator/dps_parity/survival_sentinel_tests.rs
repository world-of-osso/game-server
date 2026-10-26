//! Actual selected-Sentinel producers, consumption and native bounce impacts.
use super::marksmanship_mechanics_tests::cast;
use super::*;
use shared::protocol::CombatLogKind;

fn player() -> Run {
    let mut run = prepare_literal(255, 1).unwrap();
    hunter_fixture::provision_pet(run.app.world_mut(), run.actor).unwrap();
    cast(&mut run, 883, true);
    engage(&mut run);
    run
}

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_sentinel_unrelated_bomb_cannot_arm_chakram() {
    // SimC76f8118 sc_hunter.cpp6413–6414/6913–6914: selected Sentinel
    // earns Chakram from Takedown or Trueshot, never ordinary harmful spells.
    let mut run = player();
    cast(&mut run, 259495, false);
    for _ in 0..60 {
        run.app.update();
    }
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 1264946),
        "native Moonlight passive must not grant an enabler after Wildfire Bomb"
    );
}

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_sentinel_takedown_earns_one_consumed_native_chakram() {
    // SimC76f8118 sc_hunter.cpp6413–6414/4480–4502: the shared
    // Sentinel action consumes its real enabler and schedules real native bounces.
    let mut run = player();
    cast(&mut run, 1250646, false);
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1250646)
                && event.kind == CombatLogKind::CastSuccess),
        "Takedown must actually execute before it earns Chakram"
    );
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1264946));
    cast(&mut run, 1264949, false);
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 1264946),
        "an actual Chakram must consume the earned native enabler"
    );
    for _ in 0..160 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let impacts = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(1266081)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert!(
        impacts > 0,
        "selected Survival Sentinel must execute the shared native bounce payload"
    );
}
