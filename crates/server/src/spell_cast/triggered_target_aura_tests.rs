//! Native412677 triggers412680; an existing Stack Timer must not be refreshed.
use bevy::prelude::Entity;

use super::test_harness::*;
use crate::auras::{ActiveAuras, RemoveReason, remove_auras};

const AURA_STACK: u32 = 412677;
const STACK_TIMER: u32 = 412680;

fn native_fixture() -> Harness {
    let data = world_db().unwrap();
    let spells = [AURA_STACK, STACK_TIMER]
        .into_iter()
        .map(|id| data.store.get(id).expect("native spell").clone())
        .collect();
    harness(spells, &[], 5).unwrap()
}

fn cast_parent(h: &mut Harness, caster: Entity) {
    super::creature::creature_cast(h.app.world_mut(), caster, AURA_STACK, Some(caster), false)
        .unwrap();
    h.advance(3);
    assert!(crate::auras::has_aura(h.app.world(), caster, AURA_STACK));
}

fn timer_expiry(h: &Harness, unit: Entity) -> f64 {
    h.app
        .world()
        .get::<ActiveAuras>(unit)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == STACK_TIMER)
        .expect("real parent must apply its native timer child")
        .expires_at
        .expect("native child has180s duration")
}

#[test]
fn native_triggered_target_exclusion_does_not_refresh_existing_stack_timer() {
    let mut h = native_fixture();
    let caster = h.npc;
    cast_parent(&mut h, caster);
    let original_expiry = timer_expiry(&h, caster);
    h.advance(40);
    cast_parent(&mut h, caster);
    assert_eq!(timer_expiry(&h, caster), original_expiry);
}

#[test]
fn native_triggered_target_exclusion_removal_allows_new_stack_timer() {
    let mut h = native_fixture();
    let caster = h.npc;
    cast_parent(&mut h, caster);
    let original_expiry = timer_expiry(&h, caster);
    remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |aura| {
        aura.spell_id == STACK_TIMER
    });
    assert!(!crate::auras::has_aura(h.app.world(), caster, STACK_TIMER));
    h.advance(40);
    cast_parent(&mut h, caster);
    assert!(timer_expiry(&h, caster) > original_expiry);
}

#[test]
fn native_triggered_target_exclusion_does_not_borrow_another_units_timer() {
    let mut h = native_fixture();
    let first = h.npc;
    let other = spawn_player(&mut h.app, 5, 2.0);
    cast_parent(&mut h, first);
    let first_expiry = timer_expiry(&h, first);
    h.advance(40);
    cast_parent(&mut h, other);
    assert_eq!(timer_expiry(&h, first), first_expiry);
    assert!(timer_expiry(&h, other) > first_expiry);
}
