//! Native parent/child chains exercise triggered caster aura identity, not flags.
use bevy::prelude::Entity;

use super::test_harness::*;
use crate::auras::{RemoveReason, has_aura, remove_auras};

const LUNAR_PARENT: u32 = 181445;
const LUNAR_CHILD: u32 = 181448;
const LUNAR_PRODUCER: u32 = 298992;
const FISSURE_SHOCK: u32 = 146481;
const DARK_ASCENDANCY: u32 = 146480;
const EXCLUDED_TRANSFORM: u32 = 146543;

fn native_fixture() -> Harness {
    let data = world_db().unwrap();
    let spells = [
        LUNAR_PARENT,
        181446,
        LUNAR_CHILD,
        181449,
        LUNAR_PRODUCER,
        298993,
        FISSURE_SHOCK,
        DARK_ASCENDANCY,
        EXCLUDED_TRANSFORM,
    ]
    .into_iter()
    .map(|id| data.store.get(id).expect("native child closure").clone())
    .collect();
    harness(spells, &[], 5).unwrap()
}

fn ordinary_cast(h: &mut Harness, caster: Entity, spell: u32, target: Entity) {
    super::creature::creature_cast(h.app.world_mut(), caster, spell, Some(target), false).unwrap();
    h.advance(3);
}

fn cancel_aura(h: &mut Harness, target: Entity, spell: u32) {
    remove_auras(h.app.world_mut(), target, RemoveReason::Cancelled, |aura| {
        aura.spell_id == spell
    });
    assert!(!has_aura(h.app.world(), target, spell));
}

#[test]
fn native_triggered_caster_required_ref_refuses_lunar_child_without_producer() {
    let mut h = native_fixture();
    let caster = h.npc;
    ordinary_cast(&mut h, caster, LUNAR_PARENT, caster);
    assert!(has_aura(h.app.world(), caster, LUNAR_PARENT));
    assert!(!has_aura(h.app.world(), caster, LUNAR_PRODUCER));
    assert!(!has_aura(h.app.world(), caster, LUNAR_CHILD));
}

#[test]
fn native_triggered_caster_required_ref_accepts_actual_lunar_producer() {
    let mut h = native_fixture();
    let caster = h.npc;
    ordinary_cast(&mut h, caster, LUNAR_PRODUCER, caster);
    assert!(has_aura(h.app.world(), caster, LUNAR_PRODUCER));
    assert!(has_aura(h.app.world(), caster, LUNAR_CHILD));
}

#[test]
fn native_triggered_caster_required_ref_removal_and_foreign_recipient_do_not_qualify() {
    let mut h = native_fixture();
    let caster = h.npc;
    ordinary_cast(&mut h, caster, LUNAR_PRODUCER, caster);
    cancel_aura(&mut h, caster, LUNAR_PRODUCER);
    cancel_aura(&mut h, caster, LUNAR_CHILD);
    let other = spawn_player(&mut h.app, 5, 2.0);
    ordinary_cast(&mut h, other, LUNAR_PRODUCER, other);
    assert!(has_aura(h.app.world(), other, LUNAR_PRODUCER));
    ordinary_cast(&mut h, caster, LUNAR_PARENT, caster);
    assert!(!has_aura(h.app.world(), caster, LUNAR_CHILD));
}

#[test]
fn native_triggered_caster_excluded_ref_refuses_dark_ascendancy_only_on_caster() {
    let mut h = native_fixture();
    let caster = h.npc;
    let target = h.caster;
    ordinary_cast(&mut h, caster, EXCLUDED_TRANSFORM, caster);
    assert!(has_aura(h.app.world(), caster, EXCLUDED_TRANSFORM));
    ordinary_cast(&mut h, caster, FISSURE_SHOCK, target);
    assert!(!has_aura(h.app.world(), caster, DARK_ASCENDANCY));
}

#[test]
fn native_triggered_caster_excluded_ref_removal_and_foreign_recipient_allow_child() {
    let mut h = native_fixture();
    let caster = h.npc;
    let target = h.caster;
    ordinary_cast(&mut h, caster, EXCLUDED_TRANSFORM, caster);
    cancel_aura(&mut h, caster, EXCLUDED_TRANSFORM);
    ordinary_cast(&mut h, target, EXCLUDED_TRANSFORM, target);
    assert!(has_aura(h.app.world(), target, EXCLUDED_TRANSFORM));
    ordinary_cast(&mut h, caster, FISSURE_SHOCK, target);
    assert!(has_aura(h.app.world(), caster, DARK_ASCENDANCY));
}
