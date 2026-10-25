//! Actual Explosive Shot ticks must drive cleave and the equipped MID2 cooldowns.
use super::marksmanship_mechanics_tests::{cast, owned_buff, player};
use super::*;
use shared::protocol::CombatLogKind;

fn hits(run: &Run, spell: u32, target: Entity) -> usize {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count()
}

fn full_recharge(run: &Run, name: &str, spell: u32) -> f64 {
    use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
    use server::simc_apl::Facts;
    let bindings = Bindings {
        actions: [(name.into(), spell)].into(),
        global_cooldown_spell: spell,
        ..Default::default()
    };
    NativeFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: FightStyle::Patchwerk,
        },
        1,
    )
    .value(&format!("cooldown.{name}.full_recharge_time"))
    .unwrap()
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn native_mm_explosive_ticks_cleave_only_actual_secondary_targets() {
    // SimC76f8118 sc_hunter.cpp5872–5900: actual primary tick damage
    // feeds the native212680 child, never a second hit on the primary.
    let mut run = player(3);
    cast(&mut run, 212431, false);
    for _ in 0..160 {
        run.app.update();
    }
    assert!(
        hits(&run, 212431, run.targets[0]) > 0,
        "actual Explosive Shot primary ticks"
    );
    assert_eq!(
        hits(&run, 212680, run.targets[0]),
        0,
        "cleave excludes the actual primary"
    );
    for &target in &run.targets[1..] {
        assert!(
            hits(&run, 212680, target) > 0,
            "native cleave reaches actual secondary {target:?}"
        );
    }
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn native_mm_equipped_mid2_explosive_ticks_reduce_both_running_recharges() {
    // SimC76f8118 sc_hunter.cpp5901–5906: every actual Explosive tick
    // applies the equipped four-piece effect2 to Aimed and Rapid cooldowns.
    let mut run = player(1);
    assert!(
        owned_buff(&run, 1296634),
        "actual equipped native MID2 four-piece"
    );
    cast(&mut run, 19434, false);
    cast(&mut run, 19434, false);
    cast(&mut run, 257044, false);
    let aimed_before = full_recharge(&run, "aimed_shot", 19434);
    let rapid_before = full_recharge(&run, "rapid_fire", 257044);
    let start = crate::spell_cast::history_now(run.app.world());
    cast(&mut run, 212431, false);
    for _ in 0..100 {
        run.app.update();
    }
    let elapsed = crate::spell_cast::history_now(run.app.world()) - start;
    let ticks = hits(&run, 212431, run.targets[0]);
    let tier = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1296634)
        .unwrap();
    let per_tick = f64::from(
        tier.effects
            .iter()
            .find(|effect| effect.index == 1)
            .unwrap()
            .base_points,
    ) / 1000.0;
    let expected = per_tick * ticks as f64;
    assert!(
        expected > 0.0,
        "actual native tier timing and producing ticks"
    );
    for (name, spell, before) in [
        ("aimed_shot", 19434, aimed_before),
        ("rapid_fire", 257044, rapid_before),
    ] {
        let reduction = before - elapsed - full_recharge(&run, name, spell);
        assert!(
            (reduction - expected).abs() < 0.1,
            "{name}: actual reduction={reduction}, native expected={expected}"
        );
    }
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn native_mm_rapid_completion_consumes_the_casters_earned_trick_shots() {
    // SimC76f8118 sc_hunter.cpp5833–5836/3632–3637: a target-carried
    // channel aura still spends its CASTER's Trick Shots, not the target's.
    let mut run = player(3);
    cast(&mut run, 260243, false);
    assert!(owned_buff(&run, 257622), "actual Volley earns Trick Shots");
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 260243,
    );
    assert!(
        !owned_buff(&run, 260243),
        "Volley no longer protects Trick Shots from spending"
    );
    cast(&mut run, 257044, false);
    assert!(
        !owned_buff(&run, 257622),
        "actual completed Rapid Fire spends the caster's earned use"
    );
}
