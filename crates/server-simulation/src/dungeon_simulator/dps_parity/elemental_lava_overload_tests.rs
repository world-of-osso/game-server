//! SimC76f8118 sc_shaman.cpp6835–6855: Lava Burst Overload scales with caster crit chance.
use super::elemental_mechanics_tests::{cast, fixture, settle};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn sample(with_crit_buff: bool, spell: u32, feedback_crit: bool) -> (f64, f64) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    cast(&mut run, 188389);
    if !feedback_crit {
        let instance = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 1270062 && aura.caster == run.actor)
            .expect("selected Feedback Loop")
            .instance_id;
        assert!(crate::auras::set_effect_amount(
            run.app.world_mut(),
            run.actor,
            instance,
            0,
            0.0
        ));
    }
    if with_crit_buff {
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            run.actor,
            118522,
            Some(run.actor),
            1
        ));
        settle(&mut run);
    }
    let world = run.app.world();
    let parent = world
        .resource::<crate::spell_info::SpellStore>()
        .get(51505)
        .unwrap();
    let crit =
        crate::spell_cast::effects::caster_action_crit_chance(world, run.actor, parent) as f64;
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    let _cast_seed = crate::random::seed_scope(70448);
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        spell,
        Some(run.targets[0]),
        1
    ));
    for _ in 0..40 {
        run.app.update();
    }
    let hit = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .find(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
        })
        .expect("native Lava Burst impact");
    assert!(hit.crit, "actual Flame Shock makes the hit critical");
    (hit.amount as f64, crit)
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_lava_overload_native_crit_buff_increases_critical_damage() {
    let (base, base_crit) = sample(false, 285466, true);
    let (buffed, buffed_crit) = sample(true, 285466, true);
    assert!(
        buffed_crit > base_crit,
        "actual native Elemental Blast crit buff changed caster crit"
    );
    let expected = (1.0 + buffed_crit / 100.0) / (1.0 + base_crit / 100.0);
    let ratio = buffed / base;
    assert!(
        (ratio - expected).abs() < 0.02,
        "crit scaling: {buffed}/{base} = {ratio}, expected {expected}"
    );
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_lava_and_overload_scale_with_action_crit_bonus() {
    for spell in [285452, 285466] {
        let (base, base_crit) = sample(false, spell, false);
        let (buffed, buffed_crit) = sample(false, spell, true);
        assert!((buffed_crit - base_crit - 10.0).abs() < 0.01);
        let expected = (1.0 + buffed_crit / 100.0) / (1.0 + base_crit / 100.0);
        let ratio = buffed / base;
        assert!(
            (ratio - expected).abs() < 0.01,
            "spell {spell}: {buffed}/{base} = {ratio}, expected {expected}"
        );
    }
}
