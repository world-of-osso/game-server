//! TC a352b1fa spell_paladin.cpp806–821; native70448 owns periodic amounts/cadence.
use super::ret_tier_tests::{has, native_ret};
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const EXPURGATION_TALENT: u32 = 383344;
const EXPURGATION: u32 = 383346;
const BLADE_OF_JUSTICE: u32 = 184575;
const JUDGMENT: u32 = 20271;

fn owns_dot(run: &Run) -> bool {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.targets[0])
        .is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == EXPURGATION && aura.caster == run.actor)
        })
}

fn damage(run: &Run, spell: u32) -> i64 {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(spell)
        })
        .filter(|event| event.kind == CombatLogKind::Damage)
        .map(|event| i64::from(event.amount))
        .sum()
}

fn cast(run: &mut Run, spell: u32) {
    assert!(
        has(run, EXPURGATION_TALENT),
        "actual selected passive must be active"
    );
    assert!(!owns_dot(run));
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        spell,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
}

#[test]
#[ignore = "native selected Expurgation must follow an actual Blade of Justice hit"]
fn dps_parity_ret_expurgation_native_blade_applies_owned_periodic_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    cast(&mut run, BLADE_OF_JUSTICE);
    for _ in 0..200 {
        run.app.update();
        if damage(&run, EXPURGATION) > 0 {
            break;
        }
    }
    assert!(
        damage(&run, BLADE_OF_JUSTICE) > 0,
        "setup requires an actual native Blade hit"
    );
    assert!(
        owns_dot(&run),
        "TC Dummy-proc383344 must apply its real owned383346"
    );
    assert!(
        damage(&run, EXPURGATION) > 0,
        "native periodic effect must actually tick"
    );
}

#[test]
#[ignore = "native Expurgation family scope must not turn Judgment into Blade of Justice"]
fn dps_parity_ret_expurgation_native_judgment_keeps_unrelated_proc_scope() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    cast(&mut run, JUDGMENT);
    for _ in 0..200 {
        run.app.update();
    }
    assert!(
        damage(&run, JUDGMENT) > 0,
        "setup requires an actual native Judgment hit"
    );
    assert!(!owns_dot(&run));
    assert_eq!(damage(&run, EXPURGATION), 0);
}
