//! Actual Boomstick uses earned Tip and its native channel buff.
use super::marksmanship_mechanics_tests::cast;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_boomstick_spends_one_earned_tip_and_applies_channel_buff() {
    // SimC76f8118 sc_hunter.cpp6304–6324/6344–6350: real Boomstick
    // execution consumes one earned Tip and grants native471536 during its channel.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(255, 1).unwrap();
    hunter_fixture::provision_pet(run.app.world_mut(), run.actor).unwrap();
    cast(&mut run, 883, true);
    engage(&mut run);
    cast(&mut run, 259489, false);
    let before = owned_tip(&run);
    assert!(before > 0, "actual Kill Command must first earn Tip");
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    cast(&mut run, 1261193, false);
    assert_eq!(
        owned_tip(&run),
        before - 1,
        "actual Boomstick must consume one earned Tip"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.target == Some(run.actor.to_bits())
                && event.spell_id == Some(471536)
                && event.kind == CombatLogKind::AuraApplied),
        "actual tipped Boomstick must earn native channel buff"
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(1261215)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "actual channel must deal native Boomstick damage"
    );
}

#[test]
#[ignore = "requires native Survival255 profile and world data"]
fn survival_boomstick_extends_existing_mongoose_stacks_only_with_four_piece() {
    for equipped in [false, true] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(255, 1).unwrap();
        hunter_fixture::provision_pet(run.app.world_mut(), run.actor).unwrap();
        cast(&mut run, 883, true);
        engage(&mut run);
        if !equipped {
            crate::auras::remove_auras(
                run.app.world_mut(),
                run.actor,
                crate::auras::RemoveReason::Cancelled,
                |aura| aura.spell_id == 1296635,
            );
        }
        cast(&mut run, 186270, false);
        let deadline = mongoose_deadlines(&run)[0];
        let start = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        cast(&mut run, 1261193, false);
        for _ in 0..100 {
            if run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0[start..]
                .iter().any(|event| event.spell_id == Some(1261215) && event.kind == CombatLogKind::Damage) {
                break;
            }
            run.app.update();
        }
        assert!(run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0[start..]
            .iter().any(|event| event.spell_id == Some(1261215) && event.kind == CombatLogKind::Damage),
            "actual Boomstick tick must land");
        let extension = crate::spell_cast::scripts::script_effect_value(
            run.app.world(), run.actor, 1296635, 0,
        ) as f64 / 1000.0;
        let expected = deadline + if equipped { extension } else { 0.0 };
        assert!(mongoose_deadlines(&run).iter().any(|&value| (value - expected).abs() < 0.001),
            "first real Boomstick tick must extend the pre-existing stack only with four-piece; expected {expected}");
    }
}

fn mongoose_deadlines(run: &Run) -> Vec<f64> {
    run.app.world().get::<crate::auras::ActiveAuras>(run.actor).unwrap().iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == 259388)
        .expect("actual Raptor Strike must earn Mongoose Fury")
        .stack_expirations().to_vec()
}

fn owned_tip(run: &Run) -> u8 {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == 260286)
        .map_or(0, |aura| aura.stacks)
}
