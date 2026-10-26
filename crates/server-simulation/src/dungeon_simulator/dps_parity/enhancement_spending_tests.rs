//! Earned Storm Unleashed must supply one free Crash without delaying its normal charge.
use super::*;
use crate::auras::ActiveAuras;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use server::simc_apl::Facts;

fn stacks(run: &Run, spell: u32) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == run.actor)
        .map_or(0, |aura| aura.stacks)
}

fn crash_value(run: &Run, property: &str) -> f64 {
    let bindings = Bindings {
        actions: [("crash_lightning".into(), 187874)].into(),
        global_cooldown_spell: 187874,
        ..Default::default()
    };
    NativeFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        Encounter::Fixture {
            started_at: 0.0,
            duration: 3000.0,
            style: FightStyle::Patchwerk,
        },
        1,
    )
    .value(property)
    .unwrap()
}

fn earn_storm_with_normal_crash_recharging(run: &mut Run) {
    engage(run);
    let target = run.targets[0];
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::AutoAttack(target),
    )
    .unwrap();
    let mut spent = 0;
    let mut needs_crash = true;
    let mut normal_ready_at = 0.0;
    for _ in 0..40000 {
        if needs_crash && stacks(run, 344179) >= 5 {
            enhancement_generation_tests::cast(run, 187874, target);
            normal_ready_at = crate::spell_cast::now(run.app.world())
                + crash_value(run, "cooldown.crash_lightning.remains");
            needs_crash = false;
        }
        if !needs_crash {
            let paid = enhancement_generation_tests::try_spend_native_maelstrom(run, target);
            spent += paid;
            needs_crash = paid > 0;
        }
        if stacks(run, 1262830) > 0 {
            assert!(
                normal_ready_at > crate::spell_cast::now(run.app.world()),
                "the earned proc must overlap a real normal Crash recharge"
            );
            return;
        }
        if spent >= 250 {
            break;
        }
        run.app.update();
    }
    panic!(
        "real MW spending must earn Storm Unleashed within its first250-card deck: spent={spent}"
    );
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_real_spending_earns_free_crash_without_extending_normal_recharge() {
    // SimC76f8118 sc_shaman.cpp11830–11845/13088–13098: five spaced successes/250 MW.
    // 6107/12888–12904: consume one proc; the separate SU clock cannot extend the normal clock.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(1262713)
    );
    earn_storm_with_normal_crash_recharging(&mut run);
    for _ in 0..100 {
        let now = crate::spell_cast::now(run.app.world());
        if !run
            .app
            .world()
            .get::<crate::spell_cast::SpellHistory>(run.actor)
            .unwrap()
            .has_active_gcd(now)
        {
            break;
        }
        run.app.update();
    }
    assert_eq!(stacks(&run, 1262830), 1);
    assert_eq!(
        crash_value(&run, "cooldown.crash_lightning.remains"),
        0.0,
        "earned native411 supplies the extra charge"
    );
    let target = run.targets[0];
    // Read the running charge after real proc-driven haste changes, then cast without advancing frames.
    let remaining = crash_value(&run, "cooldown.crash_lightning.full_recharge_time");
    assert!(remaining > 0.0, "the free cast precedes normal recovery");
    crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, 187874, Some(target), false)
        .unwrap();
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .remove::<crate::combat::AutoAttack>();
    assert_eq!(
        stacks(&run, 1262830),
        0,
        "one actual Crash consumes the earned proc"
    );
    let actual = crash_value(&run, "cooldown.crash_lightning.remains");
    assert!(
        (actual - remaining).abs() < 0.051,
        "free Crash must preserve the old recharge, not append another: expected={remaining}, actual={actual}"
    );
}
