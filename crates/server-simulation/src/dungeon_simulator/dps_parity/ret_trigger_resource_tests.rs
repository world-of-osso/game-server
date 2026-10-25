//! Triggered HoW mirrors and the real Divine Resonance counter lifecycle.
use super::ret_dawnlight_consumer_tests::{damage, wait_for_damage};
use super::ret_purpose_tests::current_holy_power;
use super::ret_tier_tests::{cast_finisher, native_ret};
use super::*;
use crate::spell_cast::test_harness::ObservedTriggeredCasts;

fn resonance_stacks(run: &Run) -> Option<u8> {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)?
        .iter()
        .find(|aura| aura.spell_id == 1266308 && aura.caster == run.actor)
        .map(|aura| aura.stacks)
}

#[test]
#[ignore = "real Toll Hammer during Wrath also executes Walk into Light"]
fn dps_parity_ret_trigger_batch_toll_hammer_mirrors_blade() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    run.app.init_resource::<ObservedTriggeredCasts>();
    cast_finisher(&mut run, 31884);
    let before = current_holy_power(&run);
    cast_finisher(&mut run, 375576);
    wait_for_damage(&mut run, 1279408);
    for _ in 0..100 {
        if damage(&run, 184575) > 0 {
            break;
        }
        run.app.update();
    }
    assert!(
        damage(&run, 184575) > 0,
        "real triggered Toll Hammer must mirror native Blade"
    );
    assert!(
        run.app
            .world()
            .resource::<ObservedTriggeredCasts>()
            .0
            .iter()
            .any(|(caster, spell, target)| *caster == run.actor
                && *spell == 184575
                && *target == Some(run.targets[0]))
    );
    assert_eq!(
        current_holy_power(&run),
        (before + 3.0).min(5.0),
        "native Toll Hammer1 + mirrored Blade2 must fund the native pool"
    );
}

#[test]
#[ignore = "real Toll arms three Resonance uses; foreground Hammer spends exactly one and echoes"]
fn dps_parity_ret_trigger_batch_resonance_foreground_hammer_spends_one() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    cast_finisher(&mut run, 31884);
    cast_finisher(&mut run, 375576);
    wait_for_damage(&mut run, 1279408);
    assert_eq!(
        resonance_stacks(&run),
        Some(3),
        "actual Toll supplies native maximum3"
    );
    run.app.init_resource::<ObservedTriggeredCasts>();
    cast_finisher(&mut run, 24275);
    wait_for_damage(&mut run, 24275);
    assert_eq!(
        resonance_stacks(&run),
        Some(2),
        "one real foreground Hammer consumes one stack, not the entire native aura"
    );
    assert!(
        run.app
            .world()
            .resource::<ObservedTriggeredCasts>()
            .0
            .iter()
            .any(|(caster, spell, target)| *caster == run.actor
                && *spell == 24275
                && *target == Some(run.targets[0])),
        "the spent use must actually launch native background Hammer at the real primary"
    );
}
