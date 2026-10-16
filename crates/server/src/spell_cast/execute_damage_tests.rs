//! Native Execute damage follows the Rage actually spent, not a later pool value.
use super::test_harness::*;
use crate::auras::{AuraApplication, AuraSource};
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

const EXECUTE: u32 = 163201;
const DAMAGE: u32 = 260798;
const PROTECTION: u32 = 137048;
const ARMS: u32 = 137049;

fn spawn_execute_caster(spec: u32, talents: &[u32]) -> Harness {
    let mut ids = vec![EXECUTE, DAMAGE, spec];
    ids.extend_from_slice(talents);
    let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &ids, 1).unwrap();
    apply_self_aura(&mut h, spec);
    h
}

fn apply_self_aura(h: &mut Harness, spell_id: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn take_damage(h: &mut Harness) -> f32 {
    let events: Vec<_> = h
        .inbox
        .log
        .drain(..)
        .filter(|e| {
            e.spell_id == Some(DAMAGE)
                && e.target == Some(h.npc.to_bits())
                && e.kind == CombatLogKind::Damage
        })
        .collect();
    assert_eq!(events.len(), 1, "one native child hit: {events:?}");
    // Native frontal blocks still count toward the original damage amount.
    (events[0].amount + events[0].blocked) as f32
}

fn unscaled_child(h: &mut Harness) -> f32 {
    let before = h.power(PowerType::Rage);
    assert!(super::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        DAMAGE,
        Some(h.npc),
        1
    ));
    h.advance(3);
    assert_eq!(h.power(PowerType::Rage), before, "standalone child is free");
    let amount = take_damage(h);
    assert!(amount > 0.0);
    amount
}

fn assert_rage_damage(spec: u32, rage: f32, expected_spend: f32, multiplier: f32) {
    let mut h = spawn_execute_caster(spec, &[]);
    let baseline = unscaled_child(&mut h);
    h.set_power(PowerType::Rage, rage);
    let health = h.health(h.npc);
    h.cast_and_deliver(EXECUTE, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == EXECUTE));
    assert_eq!(h.power(PowerType::Rage), rage - expected_spend);
    assert!(h.health(h.npc) < health);
    let amount = take_damage(&mut h);
    assert_multiplier(baseline, amount, multiplier);
}

fn assert_multiplier(baseline: f32, amount: f32, multiplier: f32) {
    // Two independent native Variance0.05 rolls, each in [0.975,1.025].
    // Two points cover integer rounding of damage and blocked amounts.
    let low = baseline * multiplier * 0.975 / 1.025 - 2.0;
    let high = baseline * multiplier * 1.025 / 0.975 + 2.0;
    assert!(
        (low..=high).contains(&amount),
        "baseline={baseline},amount={amount},multiplier={multiplier},expected={low}..={high}"
    );
}

#[test]
fn execute_damage_native_protection_base_partial_and_full_actual_rage() {
    for (rage, spent, multiplier) in [
        (200.0, 200.0, 1.0),
        (300.0, 300.0, 1.5),
        (400.0, 400.0, 2.0),
        (700.0, 400.0, 2.0),
    ] {
        assert_rage_damage(PROTECTION, rage, spent, multiplier);
    }
}

#[test]
fn execute_damage_native_arms_shares_actual_rage_formula() {
    assert_rage_damage(ARMS, 400.0, 400.0, 2.0);
}

#[test]
fn execute_damage_native_sudden_death_uses_authored_rage_without_spending() {
    let mut h = spawn_execute_caster(PROTECTION, &[29725, 52437]);
    let baseline = unscaled_child(&mut h);
    apply_self_aura(&mut h, 29725);
    apply_self_aura(&mut h, 52437);
    h.set_power(PowerType::Rage, 100.0);
    h.cast_and_deliver(EXECUTE, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == EXECUTE));
    assert_eq!(h.power(PowerType::Rage), 100.0);
    let amount = take_damage(&mut h);
    assert_multiplier(baseline, amount, 2.0);
}

#[test]
fn execute_damage_native_standalone_child_does_not_spend_rage() {
    let mut h = spawn_execute_caster(PROTECTION, &[]);
    h.set_power(PowerType::Rage, 700.0);
    let before = unscaled_child(&mut h);
    h.cast_and_deliver(EXECUTE, Some(h.npc));
    let amount = take_damage(&mut h);
    assert_multiplier(before, amount, 2.0);
    let after = unscaled_child(&mut h);
    assert_multiplier(before, after, 1.0);
}

#[test]
fn execute_damage_native_triggered_parent_calculates_cost_but_spends_no_rage() {
    for (rage, multiplier) in [(100.0, 1.0), (300.0, 1.5), (700.0, 2.0)] {
        let mut h = spawn_execute_caster(PROTECTION, &[]);
        let baseline = unscaled_child(&mut h);
        h.set_power(PowerType::Rage, rage);
        assert!(super::try_cast_triggered(
            h.app.world_mut(),
            h.caster,
            EXECUTE,
            Some(h.npc),
            1
        ));
        h.advance(3);
        assert_eq!(h.power(PowerType::Rage), rage);
        let amount = take_damage(&mut h);
        assert_multiplier(baseline, amount, multiplier);
    }
}

#[test]
fn execute_damage_native_refund_does_not_reduce_launched_damage_budget() {
    let mut h = spawn_execute_caster(ARMS, &[316405]);
    let baseline = unscaled_child(&mut h);
    apply_self_aura(&mut h, 316405);
    h.set_power(PowerType::Rage, 400.0);
    h.cast_and_deliver(EXECUTE, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::Rage), 40.0, "native ten-percent refund");
    let amount = take_damage(&mut h);
    assert_multiplier(baseline, amount, 2.0);
}
