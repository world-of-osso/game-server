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
    assert_eq!(remaining.len(), total as usize);
    // TC rounds the fresh amount after adding unpaid estimated amount / total ticks.
    let expected = (paid.amount as f32 * (1.0 + (total - 1) as f32 / total as f32)).round() as i32;
    for tick in remaining {
        assert_eq!(
            tick.amount, expected,
            "native flag must carry unpaid damage, not simply replace it"
        );
    }
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
    assert!(!later.is_empty());
    assert!(
        later.iter().all(|tick| tick.amount == paid.amount),
        "unflagged native HoT must not acquire a rolling pool: {later:?}"
    );
}
