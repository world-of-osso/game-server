//! Actual ordinary Wrath, not seeded mode, drives native Hammer action/child integration.
use super::ret_purpose_tests::current_holy_power;
use super::ret_tier_tests::{has, native_ret};
use super::*;
use crate::spell_cast::{CastFailure, test_harness::ObservedCombatEvents};
use shared::protocol::CombatLogKind;

const WRATH: u32 = 31884;
const MODE: u32 = 1277026;
const HAMMER: u32 = 24275;
const TOLL: u32 = 375576;
const TOLL_HAMMER: u32 = 1279408;
const TOLL_JUDGMENT: u32 = 406957;

fn cast_wrath(run: &mut Run) {
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        WRATH,
        Some(run.actor),
        false,
    )
    .unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    assert!(has(run, WRATH));
    assert!(
        has(run, MODE),
        "actual Wrath, not a direct mode grant, owns this setup"
    );
}

fn cast_ready(run: &mut Run, spell: u32) {
    for _ in 0..200 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(run.targets[0]),
            false,
        ) {
            Ok(()) => return,
            Err(CastFailure::OnGlobalCooldown | CastFailure::InProgress) => run.app.update(),
            Err(reason) => panic!("ordinary native spell{spell} failed: {reason:?}"),
        }
    }
    panic!("ordinary native spell{spell} never became ready");
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

fn wait_for_damage(run: &mut Run, spell: u32) {
    for _ in 0..200 {
        run.app.update();
        if damage(run, spell) > 0 {
            return;
        }
    }
    panic!("actual native spell{spell} never dealt damage");
}

#[test]
#[ignore = "ordinary native Hammer after actual Wrath must deal damage and grant native resource"]
fn dps_parity_ret_wrath_integration_native_hammer_damage_and_resource() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    assert_eq!(
        literal_rotation::execution::resolve_action(run.app.world(), run.actor, HAMMER).unwrap(),
        Some(HAMMER)
    );
    let health = run
        .app
        .world()
        .get::<shared::components::Health>(run.targets[0])
        .unwrap();
    assert_eq!(
        health.current, health.max,
        "real target must be outside native execute state"
    );
    assert!(!has(&run, WRATH));
    cast_wrath(&mut run);
    let before = current_holy_power(&run);
    let amount = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(HAMMER)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .base_points;
    cast_ready(&mut run, HAMMER);
    wait_for_damage(&mut run, HAMMER);
    assert_eq!(current_holy_power(&run) - before, amount);
}

#[test]
#[ignore = "shared native target state2 is uncomputed: actual full-health Hammer must be rejected"]
fn dps_parity_ret_wrath_integration_native_uncomputed_target_state_two() {
    let mut run = native_ret(4);
    assert_eq!(
        literal_rotation::execution::resolve_action(run.app.world(), run.actor, HAMMER).unwrap(),
        Some(HAMMER)
    );
    let health = run
        .app
        .world()
        .get::<shared::components::Health>(run.targets[0])
        .unwrap();
    assert_eq!(health.current, health.max);
    assert!(!has(&run, WRATH));
    let state = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(HAMMER)
        .unwrap()
        .aura_restrictions
        .as_ref()
        .unwrap()
        .target_aura_state;
    assert_eq!(state, 2, "native2 is TC WOUNDED_20_PERCENT");
    let result = crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        HAMMER,
        Some(run.targets[0]),
        false,
    );
    assert_eq!(result, Err(CastFailure::TargetAuraState));
}

#[test]
#[ignore = "actual ordinary Wrath mode changes Toll branch and keeps native Hammer funding"]
fn dps_parity_ret_wrath_integration_native_toll_hammer_child_and_resource() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    cast_wrath(&mut run);
    let before = current_holy_power(&run);
    let amount = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(TOLL_HAMMER)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .base_points;
    cast_ready(&mut run, TOLL);
    wait_for_damage(&mut run, TOLL_HAMMER);
    assert_eq!(damage(&run, TOLL_JUDGMENT), 0);
    assert_eq!(
        current_holy_power(&run) - before,
        amount,
        "natural mode must use native Hammer funding without extra Judgment power"
    );
}
