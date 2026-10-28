//! Shared native reapplication boundary, not a class proc/producer qualification.
//! Real rows only: Languish71023 has authored positive damage and rolling flag;
//! Renew139 has periodic healing without that flag. No amount overrides.
use super::{ActiveAuras, AuraInstance, aura_now};
use crate::spell_cast::cast_triggered;
use crate::spell_cast::test_harness::{
    Harness, ObservedCombatEvents, harness_with_plugins, world_db,
};
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use shared::protocol::{CombatLogEvent, CombatLogKind};

const LANGUISH: u32 = 71_023;
const DEEP_WOUNDS: u32 = 262_115;
const RENEW: u32 = 139;
const ROLLING_PERIODIC: u32 = 0x4000;

fn native_fixture() -> Harness {
    let mut h = harness_with_plugins(vec![], &[], 8, false, |app| {
        app.init_resource::<ObservedCombatEvents>();
    })
    .unwrap();
    h.app.insert_resource(world_db().unwrap().store.clone());
    h.stop_attack();
    h.advance(3);
    h
}

fn native_aura(h: &Harness, target: Entity, spell: u32) -> AuraInstance {
    h.app
        .world()
        .get::<ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == h.caster)
        .cloned()
        .expect("native child aura applied")
}

fn cast_native_child(h: &mut Harness, target: Entity, spell: u32) {
    let caster = h.caster;
    cast_triggered(h.app.world_mut(), caster, spell, Some(target), 1);
    h.advance(1);
}

fn ticks(h: &Harness, target: Entity, spell: u32, start: usize) -> Vec<CombatLogEvent> {
    h.app.world().resource::<ObservedCombatEvents>().0[start..]
        .iter()
        .filter(|event| {
            event.source == Some(h.caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(spell)
                && event.periodic
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Heal)
        })
        .cloned()
        .collect()
}

fn first_tick(h: &mut Harness, target: Entity, spell: u32) -> CombatLogEvent {
    let start = h.app.world().resource::<ObservedCombatEvents>().0.len();
    for _ in 0..150 {
        h.advance(1);
        if let Some(event) = ticks(h, target, spell, start).into_iter().next() {
            assert!(event.amount > 0);
            return event;
        }
    }
    panic!("native periodic child{spell} did not tick");
}

fn finish_aura(h: &mut Harness, aura: &AuraInstance) {
    let end = aura.expires_at.expect("finite native fixture aura");
    while aura_now(h.app.world()) <= end + 0.1 {
        h.advance(1);
    }
}

#[test]
fn generic_rolling_periodic_native_damage_keeps_unpaid_amount_and_original_tick_phase() {
    assert_native_full_mask_rolling(cast_native_child);
    assert_native_full_mask_rolling(cast_native_creature_child);
}

fn cast_native_creature_child(h: &mut Harness, target: Entity, spell: u32) {
    let caster = h.caster;
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, spell, Some(target), true)
        .expect("native bool FULL_MASK admission");
    h.advance(1);
}

fn assert_native_full_mask_rolling(cast_child: fn(&mut Harness, Entity, u32)) {
    let mut h = native_fixture();
    let target = h.npc;
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(LANGUISH)
        .unwrap();
    assert_ne!(
        spell.misc.as_ref().unwrap().attributes[10] & ROLLING_PERIODIC,
        0
    );
    cast_child(&mut h, target, LANGUISH);
    let initial = native_aura(&h, target, LANGUISH);
    let paid = first_tick(&mut h, target, LANGUISH);
    assert_eq!(paid.kind, CombatLogKind::Damage);
    let period = initial
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .period_ms;
    let total = initial.duration_ms / period;
    assert_eq!(total, 2);
    let mid_period = aura_now(h.app.world()) + f64::from(period) / 2_000.0;
    while aura_now(h.app.world()) < mid_period {
        h.advance(1);
    }
    assert!(aura_now(h.app.world()) < initial.expires_at.unwrap());
    let start = h.app.world().resource::<ObservedCombatEvents>().0.len();
    cast_child(&mut h, target, LANGUISH);
    let refreshed = native_aura(&h, target, LANGUISH);
    while aura_now(h.app.world()) <= initial.expires_at.unwrap() + 0.1 {
        h.advance(1);
    }
    assert_eq!(
        ticks(&h, target, LANGUISH, start).len(),
        1,
        "TC FULL_MASK includes DONT_RESET_PERIODIC_TIMER: keep original phase"
    );
    finish_aura(&mut h, &refreshed);
    let remaining = ticks(&h, target, LANGUISH, start);
    // SimC76f8118 action.cpp4614–4617/4995–5015: one unpaid tick plus
    // two fresh ticks is distributed over three full carried-phase ticks.
    assert_eq!(paid.amount, 5);
    assert_eq!(remaining.len(), 3);
    assert!(
        remaining.iter().all(|tick| tick.amount == 5),
        "rolling budget must pay three full native ticks: {remaining:?}"
    );
}

#[test]
fn ignite_refresh_preserves_unequal_deposit_budgets() {
    const IGNITE: u32 = 12654;
    fn deposit(h: &mut Harness, target: Entity, amount: f32) {
        crate::spell_cast::cast_triggered_with_effect0_amount(
            h.app.world_mut(),
            h.caster,
            IGNITE,
            Some(target),
            1,
            amount,
        );
        h.advance(1);
    }

    let mut rolling = native_fixture();
    let target = rolling.npc;
    deposit(&mut rolling, target, 100.0);
    let first = first_tick(&mut rolling, target, IGNITE).amount;
    let prior = native_aura(&rolling, target, IGNITE);
    let old_effect = prior
        .effects
        .iter()
        .find(|effect| effect.period_ms > 0)
        .unwrap();
    let period = f64::from(old_effect.period_ms) / 1000.0;
    let unpaid = 1.0 + (prior.expires_at.unwrap() - old_effect.next_tick_at()) / period;

    let mut control = native_fixture();
    let control_target = control.npc;
    deposit(&mut control, control_target, 20.0);
    let fresh = first_tick(&mut control, control_target, IGNITE).amount;
    deposit(&mut rolling, target, 20.0);
    let refreshed = native_aura(&rolling, target, IGNITE);
    let effect = refreshed
        .effects
        .iter()
        .find(|effect| effect.period_ms > 0)
        .unwrap();
    let future = 1.0 + (refreshed.expires_at.unwrap() - effect.next_tick_at()) / period;
    let fresh_ticks = f64::from(prior.duration_ms) / f64::from(effect.period_ms);
    let expected = (f64::from(first) * unpaid + f64::from(fresh) * fresh_ticks) / future;
    let tick = first_tick(&mut rolling, target, IGNITE).amount;
    assert!(
        (f64::from(tick) - expected).abs() <= 2.0,
        "unequal Ignite deposits must retain their own unpaid budgets: first={first}, fresh={fresh}, expected={expected}, tick={tick}"
    );
}

#[test]
fn rolling_deep_wounds_tracks_live_attack_power_across_refresh() {
    fn set_power(h: &mut Harness, power: f32) {
        let mut stats = h
            .app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(h.caster)
            .unwrap();
        stats.attack_power = power;
        stats.crit_pct = 0.0;
    }

    let mut rolling = native_fixture();
    let target = rolling.npc;
    set_power(&mut rolling, 1_000.0);
    cast_native_child(&mut rolling, target, DEEP_WOUNDS);
    first_tick(&mut rolling, target, DEEP_WOUNDS);
    let prior = native_aura(&rolling, target, DEEP_WOUNDS);
    let effect = prior
        .effects
        .iter()
        .find(|effect| effect.period_ms > 0)
        .unwrap();
    let period = f64::from(effect.period_ms) / 1_000.0;
    let unpaid = 1.0 + (prior.expires_at.unwrap() - effect.next_tick_at()) / period;
    set_power(&mut rolling, 2_000.0);
    cast_native_child(&mut rolling, target, DEEP_WOUNDS);
    let refreshed = native_aura(&rolling, target, DEEP_WOUNDS);
    let effect = refreshed
        .effects
        .iter()
        .find(|effect| effect.period_ms > 0)
        .unwrap();
    let future = 1.0 + (refreshed.expires_at.unwrap() - effect.next_tick_at()) / period;
    let fresh_ticks = f64::from(prior.duration_ms) / f64::from(effect.period_ms);
    let multiplier = (unpaid + fresh_ticks) / future;
    assert!(multiplier > 1.1);

    let mut control = native_fixture();
    let control_target = control.npc;
    set_power(&mut control, 2_000.0);
    cast_native_child(&mut control, control_target, DEEP_WOUNDS);
    let fresh = first_tick(&mut control, control_target, DEEP_WOUNDS).amount;
    let refreshed_tick = first_tick(&mut rolling, target, DEEP_WOUNDS).amount;
    assert!(
        (f64::from(refreshed_tick) - f64::from(fresh) * multiplier).abs() <= 2.0,
        "refresh must use current AP for the entire rolled multiplier: fresh={fresh}, rolled={refreshed_tick}, multiplier={multiplier}"
    );

    set_power(&mut rolling, 4_000.0);
    set_power(&mut control, 4_000.0);
    let live = first_tick(&mut control, control_target, DEEP_WOUNDS).amount;
    let rolled_live = first_tick(&mut rolling, target, DEEP_WOUNDS).amount;
    assert!(
        (f64::from(rolled_live) - f64::from(live) * multiplier).abs() <= 2.0,
        "later AP changes must scale with the entire rolled multiplier: fresh={live}, rolled={rolled_live}, multiplier={multiplier}"
    );
}

#[test]
fn generic_rolling_periodic_native_unflagged_renew_refresh_does_not_accumulate() {
    let mut h = native_fixture();
    let target = h.caster;
    let spell = h.app.world().resource::<SpellStore>().get(RENEW).unwrap();
    assert_eq!(
        spell.misc.as_ref().unwrap().attributes[10] & ROLLING_PERIODIC,
        0
    );
    cast_native_child(&mut h, target, RENEW);
    let paid = first_tick(&mut h, target, RENEW);
    assert_eq!(paid.kind, CombatLogKind::Heal);
    let start = h.app.world().resource::<ObservedCombatEvents>().0.len();
    cast_native_child(&mut h, target, RENEW);
    let refreshed = native_aura(&h, target, RENEW);
    finish_aura(&mut h, &refreshed);
    let later = ticks(&h, target, RENEW, start);
    let (tail, full) = later.split_last().unwrap();
    // Renew's native attr13 Pandemic carry (SimC76f8118 action.cpp684–685)
    // leaves a fractional final payment (dot.cpp1154–1170), never a larger one.
    assert!(
        full.iter().all(|tick| tick.amount == paid.amount) && tail.amount <= paid.amount,
        "unflagged native HoT must not acquire a rolling pool: {later:?}"
    );
}
