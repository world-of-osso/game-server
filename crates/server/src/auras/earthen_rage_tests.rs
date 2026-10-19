//! Native70448 Elemental talent170374; TC a352b1fa spell_shaman.cpp861–920.
//! Qualification only: native root eligibility/clock/child payload, NOT parent-chain proof.
use bevy::prelude::*;
use shared::protocol::{CombatLogEvent, CombatLogKind};

use super::{ActiveAuras, AuraSource};
use crate::spell_cast::test_harness::{Harness, harness};
use crate::spell_info::SpellStore;

const PASSIVE: u32 = 170_374;
const PERIODIC: u32 = 170_377;
const DAMAGE: u32 = 170_379;
const LIGHTNING_BOLT: u32 = 188_196;

fn fixture() -> Harness {
    let mut h = harness(Vec::new(), &[PASSIVE, PERIODIC, DAMAGE, LIGHTNING_BOLT], 7).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::ActiveSpec(262));
    h.advance(3);
    h
}

fn damage_events(h: &Harness, spell: u32) -> Vec<&CombatLogEvent> {
    h.inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(spell) && e.kind == CombatLogKind::Damage && e.amount > 0)
        .collect()
}

fn assert_native_passive(h: &Harness) {
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == PASSIVE)
        .unwrap();
    assert_eq!(aura.caster, h.caster);
    assert_eq!(aura.source, AuraSource::KnownSpell);
    assert!(
        aura.effects
            .iter()
            .any(|e| e.index == 0 && e.aura_type == 4)
    );
    let store = h.app.world().resource::<SpellStore>();
    let options = store.get(PASSIVE).unwrap().aura_options.as_ref().unwrap();
    assert_eq!(options.proc_chance, 100);
    assert_eq!(options.proc_type_mask, [65536, 0]);
    assert_eq!(options.proc_charges, 0);
    assert_eq!(options.proc_category_recovery_ms, 0);
}

#[test]
fn native_earthen_rage_passive_and_ordinary_lightning_eligibility_are_qualified() {
    let mut h = fixture();
    assert_native_passive(&h);
    let target = h.npc;
    h.cast_and_deliver(LIGHTNING_BOLT, Some(target));
    h.advance(160);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let events = damage_events(&h, LIGHTNING_BOLT);
    assert!(!events.is_empty(), "{:?}", h.inbox.log);
    assert!(
        events
            .iter()
            .all(|e| e.source == Some(h.caster.to_bits()) && e.target == Some(target.to_bits()))
    );
    assert_native_passive(&h);
    eprintln!(
        "native170374KnownSpell/A4/Proc100/mask65536; ordinary188196 actualownedharmfuldamage observed"
    );
}

#[test]
fn native_earthen_rage_proc_aura_six_second_lifetime_is_qualified() {
    let mut h = fixture();
    let caster = h.caster;
    crate::spell_cast::cast_triggered(h.app.world_mut(), caster, PERIODIC, Some(caster), 1);
    h.advance(3);
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == PERIODIC)
        .unwrap();
    assert_eq!(aura.caster, caster);
    assert_eq!(aura.duration_ms, 6000);
    let expiry = aura.expires_at.unwrap();
    let effect = aura.effects.iter().find(|e| e.index == 0).unwrap();
    assert_eq!(effect.aura_type, 226);
    assert_eq!(effect.period_ms, 3000);
    while h.app.world().resource::<Time>().elapsed_secs_f64() + 0.1 < expiry {
        assert!(super::has_aura(h.app.world(), caster, PERIODIC));
        h.advance(1);
    }
    h.advance(4);
    assert!(!super::has_aura(h.app.world(), caster, PERIODIC));
    assert_native_passive(&h);
    eprintln!(
        "native170377self/A226period3000/duration6000 naturallyexpires withoutrefresh; passive retained"
    );
}

#[test]
fn native_earthen_rage_direct_damage_child_payload_is_qualified() {
    let mut h = fixture();
    let (caster, target) = (h.caster, h.npc);
    let before = h.health(target);
    crate::spell_cast::cast_triggered(h.app.world_mut(), caster, DAMAGE, Some(target), 1);
    h.advance(80);
    let events = damage_events(&h, DAMAGE);
    assert_eq!(events.len(), 1, "{:?}", h.inbox.log);
    assert_eq!(events[0].source, Some(caster.to_bits()));
    assert_eq!(events[0].target, Some(target.to_bits()));
    assert_eq!(h.health(target), before - events[0].amount as f32);
    eprintln!(
        "native170379child realdamage{} source{:?} target{:?}; no coefficient/variance/HP/RNGwrites",
        events[0].amount, events[0].source, events[0].target
    );
}
