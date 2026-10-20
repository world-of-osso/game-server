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
    h.app
        .init_resource::<crate::spell_cast::test_harness::ObservedCombatEvents>();
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

fn observed(h: &Harness, spell: u32, source: Entity) -> Vec<CombatLogEvent> {
    h.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|e| {
            e.spell_id == Some(spell)
                && e.source == Some(source.to_bits())
                && e.kind == CombatLogKind::Damage
                && e.amount > 0
        })
        .cloned()
        .collect()
}

fn cast_lightning(h: &mut Harness, target: Entity) {
    let before = observed(h, LIGHTNING_BOLT, h.caster).len();
    h.cast_and_deliver(LIGHTNING_BOLT, Some(target));
    for _ in 0..200 {
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        if observed(h, LIGHTNING_BOLT, h.caster).len() > before {
            h.advance(1);
            return;
        }
        h.advance(1);
    }
    panic!(
        "ordinary native188196 did not produce owned damage: {:?}",
        h.inbox.log
    );
}

fn remove_passive(h: &mut Harness) {
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            PERIODIC,
            DAMAGE,
            LIGHTNING_BOLT,
        ]));
    super::remove_auras(
        h.app.world_mut(),
        h.caster,
        super::RemoveReason::Cancelled,
        |a| a.spell_id == PASSIVE,
    );
    h.advance(3);
    assert!(!super::has_aura(h.app.world(), h.caster, PASSIVE));
}

fn proc_expiry(h: &Harness, source: Entity) -> f64 {
    h.app
        .world()
        .get::<ActiveAuras>(source)
        .and_then(|a| a.iter().find(|a| a.spell_id == PERIODIC))
        .expect("ordinary damage must proc native170377")
        .expires_at
        .unwrap()
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

#[test]
#[ignore = "behavioral RED before native Earthen Rage proc consumer"]
fn native_earthen_rage_ordinary_damage_procs_self_aura_and_three_scheduled_children() {
    let mut h = fixture();
    let (source, target) = (h.caster, h.npc);
    cast_lightning(&mut h, target);
    let expiry = proc_expiry(&h, source);
    let store = h.app.world().resource::<SpellStore>();
    // Native SpellMisc144253: 8704 includes EXTRA_INITIAL_PERIOD (0x200).
    let periodic_misc = store.get(PERIODIC).unwrap().misc.as_ref().unwrap();
    assert_eq!(periodic_misc.attributes[5], 8704);
    assert_ne!(periodic_misc.attributes[5] & 0x200, 0);
    // Native170379 flight is not canceled by removing the passive.
    let child_misc = store.get(DAMAGE).unwrap().misc.as_ref().unwrap();
    assert_eq!(child_misc.speed, 90.0);
    assert_eq!(child_misc.launch_delay, 0.75);
    let flight = 0.75 + 5.0 / 90.0;
    assert!(observed(&h, DAMAGE, source).is_empty());
    let mut arrivals = Vec::new();
    for _ in 0..180 {
        h.advance(1);
        let count = observed(&h, DAMAGE, source).len();
        if count > arrivals.len() {
            assert_eq!(count, arrivals.len() + 1);
            arrivals.push(h.app.world().resource::<Time>().elapsed_secs_f64() - (expiry - 6.0));
        }
    }
    assert_eq!(arrivals.len(), 3, "{arrivals:?}");
    for (arrival, dispatch) in arrivals.iter().zip([0.0, 3.0, 6.0]) {
        let expected_arrival = dispatch + flight;
        assert!((arrival - expected_arrival).abs() <= 0.15, "{arrivals:?}");
    }
    let children = observed(&h, DAMAGE, source);
    assert_eq!(children.len(), 3, "{children:?}");
    assert!(
        children
            .iter()
            .all(|e| e.target == Some(target.to_bits()) && e.amount >= 41 && e.amount <= 43),
        "{children:?}"
    );
    assert!(h.app.world().resource::<Time>().elapsed_secs_f64() > expiry);
    assert!(!super::has_aura(h.app.world(), source, PERIODIC));
    h.advance(120);
    assert_eq!(observed(&h, DAMAGE, source).len(), 3);
}

#[test]
#[ignore = "behavioral RED before last-action-target native Earthen Rage state"]
fn native_earthen_rage_latest_actual_action_target_replaces_first_and_not_selection() {
    let mut h = fixture();
    let (source, first) = (h.caster, h.npc);
    let second = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 4.0);
    cast_lightning(&mut h, first);
    let _ = proc_expiry(&h, source);
    cast_lightning(&mut h, second);
    h.select(Some(first)); // Selection must not replace the last actual action target.
    h.advance(3);
    let before = observed(&h, DAMAGE, source).len();
    h.advance(140);
    let children = observed(&h, DAMAGE, source);
    let after_retarget = &children[before..];
    assert!(!after_retarget.is_empty(), "{children:?}");
    assert!(
        after_retarget
            .iter()
            .all(|e| e.target == Some(second.to_bits())),
        "{children:?}"
    );
}

#[test]
#[ignore = "behavioral control for native passive ownership"]
fn native_earthen_rage_without_passive_does_not_proc_or_damage() {
    let mut h = fixture();
    remove_passive(&mut h);
    let (source, target) = (h.caster, h.npc);
    cast_lightning(&mut h, target);
    h.advance(180);
    assert!(!super::has_aura(h.app.world(), source, PERIODIC));
    assert!(observed(&h, DAMAGE, source).is_empty());
}

#[test]
#[ignore = "behavioral control for spell-only native proc mask"]
fn native_earthen_rage_ordinary_melee_swings_do_not_proc_spell_passive() {
    let mut h = fixture();
    let target = h.npc;
    h.attack(target);
    h.advance(200);
    h.stop_attack();
    h.advance(3);
    assert!(!h.inbox.attack_starts.is_empty());
    assert!(!super::has_aura(h.app.world(), h.caster, PERIODIC));
    assert!(observed(&h, DAMAGE, h.caster).is_empty());
}

#[test]
#[ignore = "behavioral control against damage-child self-reentry"]
fn native_earthen_rage_child_cannot_reactivate_or_refresh_its_parent() {
    let mut h = fixture();
    let (source, target) = (h.caster, h.npc);
    crate::spell_cast::cast_triggered(h.app.world_mut(), source, DAMAGE, Some(target), 1);
    h.advance(80);
    assert_eq!(observed(&h, DAMAGE, source).len(), 1);
    assert!(!super::has_aura(h.app.world(), source, PERIODIC));
    h.advance(160);
    assert_eq!(observed(&h, DAMAGE, source).len(), 1);
}

#[test]
#[ignore = "behavioral RED before passive-scoped Earthen Rage target lifecycle"]
fn native_earthen_rage_passive_removal_stops_pending_child_dispatch() {
    let mut h = fixture();
    let (source, target) = (h.caster, h.npc);
    cast_lightning(&mut h, target);
    let _ = proc_expiry(&h, source);
    remove_passive(&mut h);
    // Drain only the native initial child already in flight: Speed90,
    // LaunchDelay0.75;20 ordinary50ms frames remain before the next3s tick.
    h.advance(20);
    let before = observed(&h, DAMAGE, source).len();
    assert_eq!(
        before, 1,
        "initial native missile must survive passive removal"
    );
    h.advance(180);
    assert_eq!(observed(&h, DAMAGE, source).len(), before);
}

#[test]
#[ignore = "behavioral RED before owned Earthen Rage target isolation"]
fn native_earthen_rage_two_shamans_keep_independent_last_action_targets() {
    let mut h = fixture();
    let (first, first_target) = (h.caster, h.npc);
    let second = crate::spell_cast::test_harness::spawn_player(&mut h.app, 7, 1.0);
    let second_target = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 4.0);
    h.app
        .world_mut()
        .entity_mut(second)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            PASSIVE,
            PERIODIC,
            DAMAGE,
            LIGHTNING_BOLT,
        ]));
    h.advance(3);
    assert!(super::has_aura(h.app.world(), second, PASSIVE));
    cast_lightning(&mut h, first_target);
    let _ = proc_expiry(&h, first);
    crate::spell_cast::creature_cast(
        h.app.world_mut(),
        second,
        LIGHTNING_BOLT,
        Some(second_target),
        false,
    )
    .unwrap();
    h.advance(200);
    let first_children = observed(&h, DAMAGE, first);
    let second_children = observed(&h, DAMAGE, second);
    assert!(!first_children.is_empty() && !second_children.is_empty());
    assert!(
        first_children
            .iter()
            .all(|e| e.target == Some(first_target.to_bits())),
        "{first_children:?}"
    );
    assert!(
        second_children
            .iter()
            .all(|e| e.target == Some(second_target.to_bits())),
        "{second_children:?}"
    );
}

#[test]
#[ignore = "behavioral RED before actual resolved-target Earthen Rage dispatch"]
fn native_earthen_rage_despawned_action_target_has_no_fallback_victim() {
    let mut h = fixture();
    let target = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 4.0);
    let source = h.caster;
    cast_lightning(&mut h, target);
    let _ = proc_expiry(&h, source);
    h.app.world_mut().despawn(target);
    h.select(Some(h.npc));
    h.advance(180);
    assert!(observed(&h, DAMAGE, source).is_empty());
}
