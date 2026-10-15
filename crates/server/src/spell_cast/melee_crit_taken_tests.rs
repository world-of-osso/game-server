//! TC a352b1fa Unit.cpp2898: victim aura187 affects melee, not ranged, crit.
use super::test_harness::*;
use crate::auras::{AuraApplication, AuraSource, RemoveReason};
use crate::combat::AutoAttack;
use crate::stats::CharacterStats;
use bevy::prelude::Entity;
use shared::components::{Health, PowerType};
use shared::protocol::CombatLogKind;

const HARDENED_BATTLELYNX: u32 = 460575;
const SLAM: u32 = 1464;
const NATIVE_CRIT_REDUCTION: f32 = 60.0;

fn protected_target() -> Harness {
    let mut h = harness_with_native_enemy_dodge_reduction(
        Vec::new(),
        &[
            HARDENED_BATTLELYNX,
            SLAM,
            75,
            1449,
            236422,
            5487,
            441685,
            389590,
            454788,
            269360,
            9483,
        ],
        1,
    )
    .unwrap();
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = NATIVE_CRIT_REDUCTION;
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = 1_000_000.0;
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().max = 1_000_000.0;
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.npc,
            target: h.npc,
            spell_id: HARDENED_BATTLELYNX,
            effect_mask: 1 << 2,
            source: AuraSource::Cast,
        },
    );
    h
}

fn apply_native(h: &mut Harness, target: Entity, spell: u32, index: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: target,
            target,
            spell_id: spell,
            effect_mask: 1 << index,
            source: AuraSource::Cast,
        },
    );
}

fn remove_protection(h: &mut Harness) {
    crate::auras::remove_auras(h.app.world_mut(), h.npc, RemoveReason::Cancelled, |aura| {
        aura.spell_id == HARDENED_BATTLELYNX
    });
}

fn slam_repeatedly(h: &mut Harness) {
    for _ in 0..24 {
        h.set_power(PowerType::Rage, 1000.0);
        h.cast_and_deliver(SLAM, Some(h.npc));
        h.advance(40);
    }
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
}

fn damage_events(h: &Harness) -> Vec<&shared::protocol::CombatLogEvent> {
    h.inbox
        .log
        .iter()
        .filter(|event| {
            event.target == Some(h.npc.to_bits()) && event.kind == CombatLogKind::Damage
        })
        .collect()
}

#[test]
fn class_completeness_native_hardened_battlelynx_prevents_slam_crit_until_removed() {
    let mut h = protected_target();
    slam_repeatedly(&mut h);
    let protected = damage_events(&h);
    assert!(protected.len() >= 24, "{:?}", h.inbox.log);
    assert!(protected.iter().all(|event| !event.crit), "{protected:?}");
    crate::auras::remove_auras(h.app.world_mut(), h.npc, RemoveReason::Cancelled, |aura| {
        aura.spell_id == HARDENED_BATTLELYNX
    });
    h.inbox.log.clear();
    slam_repeatedly(&mut h);
    assert!(
        damage_events(&h).iter().any(|event| event.crit),
        "{:?}",
        h.inbox.log
    );
}

#[test]
fn class_completeness_native_hardened_battlelynx_prevents_auto_attack_crit() {
    let mut h = protected_target();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(AutoAttack::new(h.npc));
    h.advance(1000);
    let protected = damage_events(&h);
    assert!(protected.len() >= 12, "{:?}", h.inbox.log);
    assert!(protected.iter().all(|event| !event.crit), "{protected:?}");
}

#[test]
fn class_completeness_native_boulder_uses_melee_crit_taken_despite_ranged_defense() {
    let mut h = protected_target();
    charge_range(&mut h);
    for _ in 0..24 {
        h.cast_and_deliver(9483, Some(h.npc));
        h.advance(200);
    }
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let damage = damage_events(&h);
    assert!(damage.len() >= 24, "{:?}", h.inbox.log);
    assert!(damage.iter().all(|event| !event.crit), "{damage:?}");
}

#[test]
fn class_completeness_native_shred_script_crit_precedes_victim_reduction() {
    let mut h = harness_with_native_enemy_dodge_reduction(
        Vec::new(),
        &[5221, 768, 5215, 343232, HARDENED_BATTLELYNX],
        11,
    )
    .unwrap();
    let caster = h.caster;
    let npc = h.npc;
    apply_native(&mut h, caster, 768, 0);
    apply_native(&mut h, caster, 5215, 0);
    apply_native(&mut h, npc, HARDENED_BATTLELYNX, 2);
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(caster)
        .unwrap()
        .crit_pct = 80.0;
    h.cast_and_deliver(5221, Some(npc));
    h.advance(3);
    let damage = damage_events(&h);
    assert!(!damage.is_empty(), "{:?}", h.inbox.log);
    // Native Shred doubles80 first:160 minus native60 yields100, not40.
    assert!(damage.iter().all(|event| event.crit), "{damage:?}");
}

#[test]
fn class_completeness_native_melee_crit_reduction_does_not_change_ranged_or_magic() {
    for spell in [75, 1449] {
        let mut h = protected_target();
        h.app
            .world_mut()
            .get_mut::<CharacterStats>(h.caster)
            .unwrap()
            .crit_pct = 100.0;
        if spell == 75 {
            charge_range(&mut h);
        }
        h.cast_and_deliver(spell, Some(h.npc));
        h.advance(60);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        let damage = damage_events(&h);
        assert!(!damage.is_empty(), "{:?}", h.inbox.log);
        assert!(damage.iter().all(|event| event.crit), "{damage:?}");
    }
}

#[test]
fn class_completeness_native_victim_crit_reductions_add_across_parent_auras() {
    let mut h = protected_target();
    remove_protection(&mut h);
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 6.0;
    let npc = h.npc;
    apply_native(&mut h, npc, 389590, 0); // Demonic Resilience -2.
    apply_native(&mut h, npc, 454788, 1); // Runic Protection -3.
    apply_native(&mut h, npc, 269360, 2); // Avoidance -1.
    slam_repeatedly(&mut h);
    let damage = damage_events(&h);
    assert!(damage.len() >= 24, "{:?}", h.inbox.log);
    assert!(damage.iter().all(|event| !event.crit), "{damage:?}");
}

#[test]
fn class_completeness_native_melee_crit_modifier_belongs_to_the_victim() {
    let mut h = protected_target();
    remove_protection(&mut h);
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 100.0;
    let caster = h.caster;
    apply_native(&mut h, caster, HARDENED_BATTLELYNX, 2);
    slam_repeatedly(&mut h);
    let damage = damage_events(&h);
    assert!(damage.iter().all(|event| event.crit), "{damage:?}");
}

#[test]
fn class_completeness_native_brittle_increases_victim_melee_crit_chance() {
    let mut h = protected_target();
    remove_protection(&mut h);
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 0.0;
    let npc = h.npc;
    apply_native(&mut h, npc, 236422, 3);
    slam_repeatedly(&mut h);
    let damage = damage_events(&h);
    assert!(!damage.is_empty());
    assert!(damage.iter().all(|event| event.crit), "{damage:?}");
}

#[test]
fn class_completeness_native_bear_reduction_stops_creature_melee_crits() {
    let mut h = protected_target();
    remove_protection(&mut h);
    let caster = h.caster;
    apply_native(&mut h, caster, 5487, 2);
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(AutoAttack::new(caster));
    h.advance(600);
    let damage: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.source == Some(h.npc.to_bits()) && event.kind == CombatLogKind::Damage
        })
        .collect();
    assert!(!damage.is_empty(), "{:?}", h.inbox.log);
    assert!(damage.iter().all(|event| !event.crit), "{damage:?}");
}

#[test]
fn class_completeness_native_wildshape_crit_reduction_expires_at_six_seconds() {
    let mut h = protected_target();
    remove_protection(&mut h);
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 6.0;
    let npc = h.npc;
    apply_native(&mut h, npc, 441685, 3);
    for _ in 0..3 {
        h.set_power(PowerType::Rage, 1000.0);
        h.cast_and_deliver(SLAM, Some(npc));
        h.advance(32);
    }
    assert!(crate::auras::has_aura(h.app.world(), npc, 441685));
    assert!(damage_events(&h).iter().all(|event| !event.crit));
    h.advance(30);
    assert!(!crate::auras::has_aura(h.app.world(), npc, 441685));
    h.inbox.log.clear();
    for _ in 0..240 {
        h.set_power(PowerType::Rage, 1000.0);
        h.cast_and_deliver(SLAM, Some(npc));
        h.advance(32);
        if damage_events(&h).iter().any(|event| event.crit) {
            return;
        }
    }
    panic!("no witnessed critical Slam after native six-second expiry");
}
