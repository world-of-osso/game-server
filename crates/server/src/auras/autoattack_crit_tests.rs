//! TC a352b1fa Unit.cpp2387: signed caster aura334 is autoattack-only.
//! Native340094 supplies100 without aura290; native456718 supplies-999.
use lightyear::prelude::MessageSender;
use shared::protocol::{CancelAura, CombatChannel, CombatLogEvent, CombatLogKind};

use super::ActiveAuras;
use crate::spell_cast::test_harness::{Harness, harness};
use crate::stats::CharacterStats;

const MASTER_ASSASSINS_MARK: u32 = 340_094;
const SHADOW_CORRUPTION: u32 = 456_718;
const FIREBALL: u32 = 133;
const STRIKE: u32 = 38_598;
const ROGUE: u8 = 4;

fn native_harness() -> Harness {
    harness(
        Vec::new(),
        &[MASTER_ASSASSINS_MARK, SHADOW_CORRUPTION, FIREBALL, STRIKE],
        ROGUE,
    )
    .expect("native70391 aura334 spell fixtures")
}

fn cast_buff(h: &mut Harness, spell_id: u32) {
    h.cast_and_deliver(spell_id, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(h.caster)
            .unwrap()
            .has_spell(spell_id)
    );
}

fn unblocked_swings(h: &mut Harness) -> Vec<CombatLogEvent> {
    h.inbox.log.clear();
    h.attack(h.npc);
    // Ordinary unarmed2s swing clock; no timer, RNG or aura-duration edits.
    h.advance(800);
    h.stop_attack();
    h.advance(3);
    let swings: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.source == Some(h.caster.to_bits())
                && event.spell_id.is_none()
                && event.amount > 0
                && event.blocked == 0
        })
        .cloned()
        .collect();
    assert!(swings.len() >= 5, "{swings:?}");
    swings
}

fn cast_spell_damage(h: &mut Harness, spell_id: u32) -> CombatLogEvent {
    // Native weapon spells can miss: sample bounded ordinary requests, not RNG.
    for _ in 0..8 {
        h.inbox.log.clear();
        h.cast_and_deliver(spell_id, Some(h.npc));
        h.advance(80);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        let damage = h.inbox.log.iter().find(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(spell_id)
                && event.amount > 0
        });
        if let Some(damage) = damage {
            return damage.clone();
        }
    }
    panic!("no native{spell_id} damage after eight ordinary requests");
}

#[test]
fn class_completeness_native_334_makes_autoattacks_critical_and_cancel_removes_it() {
    let mut h = native_harness();
    assert!(unblocked_swings(&mut h).iter().all(|event| !event.crit));
    cast_buff(&mut h, MASTER_ASSASSINS_MARK);
    let buffed = unblocked_swings(&mut h);
    assert!(buffed.iter().all(|event| event.crit), "{buffed:?}");
    h.app
        .world_mut()
        .get_mut::<MessageSender<CancelAura>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(CancelAura {
            spell_id: MASTER_ASSASSINS_MARK,
        });
    h.advance(3);
    assert!(unblocked_swings(&mut h).iter().all(|event| !event.crit));
}

#[test]
fn class_completeness_native_334_does_not_grant_magic_or_weapon_spell_crit() {
    let mut h = native_harness();
    cast_buff(&mut h, MASTER_ASSASSINS_MARK);
    assert!(!cast_spell_damage(&mut h, FIREBALL).crit);
    assert!(!cast_spell_damage(&mut h, STRIKE).crit);
}

#[test]
fn class_completeness_native_negative_334_removes_autoattack_not_spell_crit() {
    let mut h = native_harness();
    // Deliberate existing stat input, not a legal-build or rating-budget claim.
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 100.0;
    cast_buff(&mut h, MASTER_ASSASSINS_MARK);
    cast_buff(&mut h, SHADOW_CORRUPTION);
    let swings = unblocked_swings(&mut h);
    assert!(swings.iter().all(|event| !event.crit), "{swings:?}");
    assert!(cast_spell_damage(&mut h, FIREBALL).crit);
}

#[test]
fn class_completeness_native_334_on_victim_does_not_grant_attacker_crit() {
    let mut h = native_harness();
    let npc = h.npc;
    crate::spell_cast::creature_cast(
        h.app.world_mut(),
        npc,
        MASTER_ASSASSINS_MARK,
        Some(npc),
        true,
    )
    .expect("native NPC self-buff");
    h.advance(3);
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(npc)
            .unwrap()
            .has_spell(MASTER_ASSASSINS_MARK)
    );
    assert!(unblocked_swings(&mut h).iter().all(|event| !event.crit));
}
