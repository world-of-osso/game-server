//! SimC76f8118 sc_shaman.cpp6835–6855: Lava Burst Overload scales with caster crit chance.
use super::elemental_mechanics_tests::{cast, fixture, settle};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn sample(with_crit_buff: bool, spell: u32) -> (f64, f64) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    cast(&mut run, 188389);
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
    let crit = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .crit_pct as f64;
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
    let (base, base_crit) = sample(false, 285466);
    let (buffed, buffed_crit) = sample(true, 285466);
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
fn elemental_lava_burst_native_crit_buff_increases_critical_damage() {
    let (base, base_crit) = sample(false, 285452);
    let (buffed, buffed_crit) = sample(true, 285452);
    assert!(buffed_crit > base_crit);
    let expected = (1.0 + buffed_crit / 100.0) / (1.0 + base_crit / 100.0);
    let ratio = buffed / base;
    assert!((ratio - expected).abs() < 0.02, "crit scaling: {buffed}/{base} = {ratio}, expected {expected}");
}
