//! Actual Survival spenders earn selected per-target native Armaments residuals.
use super::marksmanship_mechanics_tests::cast;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_raptor_and_swipe_earn_owned_armaments_for_real_recipients() {
    // SimC76f8118 sc_hunter.cpp6140–6167: each actual Raptor/Swipe
    // impact banks selected effect2 into the native rolling residual on its recipient.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(255, 3).unwrap();
    hunter_fixture::provision_pet(run.app.world_mut(), run.actor).unwrap();
    cast(&mut run, 883, true);
    engage(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1253831));
    cast(&mut run, 186270, false);
    assert_owned_residual(&run, run.targets[0]);
    cast(&mut run, 186270, false);
    for &target in &run.targets {
        assert_owned_residual(&run, target);
    }
    for _ in 0..80 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for &target in &run.targets {
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(1253836)
                    && event.kind == CombatLogKind::Damage
                    && event.periodic
                    && event.amount > 0),
            "actual Swipe target {target:?} must receive positive native Armaments ticks"
        );
    }
}

fn assert_owned_residual(run: &Run, target: Entity) {
    let auras = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(target)
        .unwrap();
    assert!(
        auras
            .iter()
            .any(|aura| aura.caster == run.actor && aura.spell_id == 1253836),
        "actual target {target:?} must receive the owned native Armaments residual"
    );
}
