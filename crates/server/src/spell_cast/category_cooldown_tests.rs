//! TC a352b1fa SpellHistory.cpp:469-500:341 adds its signed milliseconds
//! to both positive cooldowns of spells with the matching category.
use super::test_harness::*;
use shared::spell_data::CastFailReason;

const QUICK_WITTED: u32 = 382_297;
const COUNTERSPELL: u32 = 2139;
const WIND_SHEAR: u32 = 57_994;
const OPTICAL_BLAST: u32 = 115_781;
const ANGELS_MERCY: u32 = 238_100;
const DESPERATE_PRAYER: u32 = 19_236;
const HOLY_PALADIN: u32 = 428_076;
const SHARED_PVP_TRIGGER: u32 = 72_757;

fn cast_cooldown(h: &mut Harness, spell: u32, target: bevy::prelude::Entity) -> u32 {
    h.inbox.cooldowns.clear();
    h.cast_and_deliver(spell, Some(target));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.inbox
        .cooldowns
        .iter()
        .find(|r| r.spell_id == spell && !r.is_gcd)
        .expect("native cast must publish a cooldown")
        .duration_ms
}

fn quick_witted_harness() -> Harness {
    let mut h = harness(Vec::new(), &[QUICK_WITTED, COUNTERSPELL], 8).unwrap();
    h.advance(3);
    h
}

#[test]
fn class_completeness_native_category_cooldown_excludes_other_categories_and_absent_talent() {
    let mut absent = harness(Vec::new(), &[COUNTERSPELL], 8).unwrap();
    let target = absent.npc;
    assert_eq!(cast_cooldown(&mut absent, COUNTERSPELL, target), 25_000);
    // Cross-class native Wind Shear fixture has category0, not88; no
    // claim that a mage actually learns Wind Shear.
    let mut other = harness(Vec::new(), &[QUICK_WITTED, WIND_SHEAR], 8).unwrap();
    other.advance(3);
    let target = other.npc;
    assert_eq!(cast_cooldown(&mut other, WIND_SHEAR, target), 12_000);
}

#[test]
fn class_completeness_native_category_cooldown_reduces_both_native_recovery_fields() {
    // Native pet spell115781 has both spell andcategory cooldown24000ms;
    // shared cast pipeline fixture, not player spellbook availability.
    // Energy-bearing caster supplies the native pet spell's Energy cost.
    let mut h = harness(Vec::new(), &[QUICK_WITTED, OPTICAL_BLAST], 4).unwrap();
    h.advance(3);
    h.set_power(shared::components::PowerType::Energy, 100.0);
    let target = h.npc;
    assert_eq!(cast_cooldown(&mut h, OPTICAL_BLAST, target), 19_000);
    h.advance(385);
    assert_eq!(cast_cooldown(&mut h, OPTICAL_BLAST, target), 19_000);
}

#[test]
fn class_completeness_native_category_cooldown_unlearning_restores_new_lockout() {
    let mut h = quick_witted_harness();
    let target = h.npc;
    assert_eq!(cast_cooldown(&mut h, COUNTERSPELL, target), 20_000);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            COUNTERSPELL,
        ]));
    h.advance(405);
    assert_eq!(cast_cooldown(&mut h, COUNTERSPELL, target), 25_000);
}

#[test]
fn class_completeness_native_category_cooldown_learning_does_not_retime_running_cooldown() {
    let mut h = harness(Vec::new(), &[COUNTERSPELL, QUICK_WITTED], 8).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            COUNTERSPELL,
        ]));
    h.advance(3);
    let target = h.npc;
    assert_eq!(cast_cooldown(&mut h, COUNTERSPELL, target), 25_000);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            COUNTERSPELL,
            QUICK_WITTED,
        ]));
    h.advance(405);
    h.cast_and_deliver(COUNTERSPELL, Some(target));
    assert_eq!(
        h.inbox.failed.last().unwrap().reason,
        CastFailReason::OnCooldown
    );
    h.inbox.failed.clear();
    h.advance(105);
    assert_eq!(cast_cooldown(&mut h, COUNTERSPELL, target), 20_000);
}

#[test]
fn class_completeness_native_category_cooldown_angels_mercy_reduces_prayer_category() {
    let mut h = harness(Vec::new(), &[ANGELS_MERCY, DESPERATE_PRAYER], 5).unwrap();
    h.advance(3);
    let target = h.caster;
    assert_eq!(cast_cooldown(&mut h, DESPERATE_PRAYER, target), 70_000);
}

#[test]
fn class_completeness_native_category_cooldown_native_healer_modifier_clamps_at_zero() {
    // Authored healer -30000ms and native shared1182 trigger30000ms.
    // This internal trigger fixture tests clamping, not ordinary player input.
    let mut h = harness(Vec::new(), &[HOLY_PALADIN, SHARED_PVP_TRIGGER], 2).unwrap();
    h.advance(3);
    h.cast_and_deliver(SHARED_PVP_TRIGGER, Some(h.caster));
    h.cast_and_deliver(SHARED_PVP_TRIGGER, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .all(|r| r.is_gcd || r.spell_id != SHARED_PVP_TRIGGER)
    );
    assert_eq!(
        h.inbox
            .go
            .iter()
            .filter(|r| r.spell_id == SHARED_PVP_TRIGGER)
            .count(),
        2
    );
}

#[test]
fn class_completeness_native_category_cooldown_changes_actual_counterspell_receipt() {
    let mut h = quick_witted_harness();
    h.cast_and_deliver(COUNTERSPELL, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let receipt = h
        .inbox
        .cooldowns
        .iter()
        .find(|receipt| receipt.spell_id == COUNTERSPELL && !receipt.is_gcd)
        .expect("Counterspell starts its native category88 cooldown");
    assert_eq!(receipt.duration_ms, 20_000);
    assert_eq!(receipt.category, 88);
}

#[test]
fn class_completeness_native_category_cooldown_allows_real_recast_after_twenty_seconds() {
    let mut h = quick_witted_harness();
    h.cast_and_deliver(COUNTERSPELL, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(360);
    h.cast_and_deliver(COUNTERSPELL, Some(h.npc));
    assert_eq!(
        h.inbox.failed.last().unwrap().reason,
        CastFailReason::OnCooldown
    );
    h.inbox.failed.clear();
    h.advance(45);
    h.cast_and_deliver(COUNTERSPELL, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        h.inbox
            .go
            .iter()
            .filter(|event| event.spell_id == COUNTERSPELL)
            .count(),
        2
    );
}
