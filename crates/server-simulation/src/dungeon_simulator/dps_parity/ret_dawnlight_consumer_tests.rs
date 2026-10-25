//! Consumer RED follows actual native producer GREEN; budget is never injected.
use super::ret_dawnlight_tests::{budget, cast_and_damage, native_count};
use super::ret_purpose_tests::{current_holy_power, earn_finisher_power};
use super::ret_tier_tests::{cast_finisher, native_ret};
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const WAKE: u32 = 255937;
const FINAL_VERDICT: u32 = 383328;
const DAWNLIGHT: u32 = 431380;

fn owned_dot(run: &Run) -> Option<&crate::auras::AuraInstance> {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.targets[0])?
        .iter()
        .find(|aura| aura.spell_id == DAWNLIGHT && aura.caster == run.actor)
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
                && event.kind == CombatLogKind::Damage
        })
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
    panic!("native spell{spell} never damaged the actual target");
}

fn wake_budget() -> Run {
    let mut run = native_ret(4);
    assert_eq!(native_count(&run), 3.0);
    assert!(owned_dot(&run).is_none());
    cast_and_damage(&mut run, WAKE);
    assert_eq!(current_holy_power(&run), 3.0);
    let aura = budget(&run).expect("producer prerequisite must already be GREEN");
    assert_eq!(aura.stacks, 3);
    assert_eq!(aura.max_stacks, 6);
    run
}

#[test]
#[ignore = "strict actual Wake budget and paid foreground hit must apply native Dawnlight and spend one use"]
fn dps_parity_dawnlight_consumer_paid_native_verdict_applies_dot_and_spends_one() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = wake_budget();
    cast_finisher(&mut run, FINAL_VERDICT);
    wait_for_damage(&mut run, FINAL_VERDICT);
    assert_eq!(
        current_holy_power(&run),
        0.0,
        "ordinary Verdict paid its real3HP"
    );
    assert!(
        owned_dot(&run).is_some(),
        "successful funded foreground spender must apply caster-owned native431380"
    );
    wait_for_damage(&mut run, DAWNLIGHT);
    assert_eq!(
        budget(&run).unwrap().stacks,
        2,
        "one spender consumes one actual use"
    );
    assert_eq!(budget(&run).unwrap().max_stacks, 6);
}

#[test]
#[ignore = "actual native431380 periodic payload already works without a consumer shim"]
fn dps_parity_dawnlight_consumer_native_payload_control_ticks() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(DAWNLIGHT)
        .unwrap();
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap();
    assert_eq!(effect.aura, 3);
    assert_eq!(effect.aura_period_ms, 2000);
    assert!((effect.bonus_coefficient - 0.6375).abs() < 0.000001);
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        DAWNLIGHT,
        Some(run.targets[0]),
        0,
    );
    assert!(owned_dot(&run).is_some());
    wait_for_damage(&mut run, DAWNLIGHT);
    assert!(
        budget(&run).is_none(),
        "payload cannot manufacture a counter"
    );
}

#[test]
#[ignore = "full-mask background spender must not consume or apply the foreground policy"]
fn dps_parity_dawnlight_consumer_background_verdict_keeps_earned_budget() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = wake_budget();
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        FINAL_VERDICT,
        Some(run.targets[0]),
        0,
    );
    wait_for_damage(&mut run, FINAL_VERDICT);
    assert_eq!(
        current_holy_power(&run),
        3.0,
        "full-mask background is not a paid cast"
    );
    assert_eq!(budget(&run).unwrap().stacks, 3);
    assert!(owned_dot(&run).is_none());
}

#[test]
#[ignore = "naturally funded ordinary spender without Wake cannot invent a budget or Dawnlight"]
fn dps_parity_dawnlight_consumer_paid_no_wake_keeps_dot_absent() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    cast_and_damage(&mut run, 20271);
    earn_finisher_power(&mut run, FINAL_VERDICT);
    assert!(budget(&run).is_none());
    cast_finisher(&mut run, FINAL_VERDICT);
    wait_for_damage(&mut run, FINAL_VERDICT);
    assert!(budget(&run).is_none());
    assert!(owned_dot(&run).is_none());
}
