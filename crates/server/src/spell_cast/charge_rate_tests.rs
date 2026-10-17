//! Native148: TC a352b1fa SpellHistory.cpp880-914,1003-1005.
use super::test_harness::*;
use crate::auras::{AuraApplication, AuraSource, RemoveReason, apply_aura, remove_auras};
use shared::components::{Health, Position};
use shared::protocol::{CombatLogKind, SpellChargesUpdate};

const TRUESHOT: u32 = 288_613;
const AIMED_SHOT: u32 = 19_434;

fn fixture() -> Harness {
    let mut h = harness(Vec::new(), &[TRUESHOT, AIMED_SHOT], 3).unwrap();
    h.app.world_mut().get_mut::<Position>(h.npc).unwrap().x = 10.0;
    *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
        current: 20_000.0,
        max: 20_000.0,
    };
    h
}

fn apply_charge_rate(h: &mut Harness) {
    let unit = h.caster;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: TRUESHOT,
            // Isolate authentic effect1:148, amount40, charge category1715.
            effect_mask: 1 << 1,
            source: AuraSource::Cast,
        },
    );
}

fn remaining_ms(h: &Harness) -> u32 {
    let history = h.app.world().get::<super::SpellHistory>(h.caster).unwrap();
    let remaining = history.charges[&1715].next_ready_at() - super::history_now(h.app.world());
    (remaining.max(0.0) * 1000.0) as u32
}

fn cast_aimed_shot(h: &mut Harness) -> SpellChargesUpdate {
    h.inbox.log.clear();
    h.inbox.charges.clear();
    h.cast_and_deliver(AIMED_SHOT, Some(h.npc));
    h.advance(100);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.log.iter().any(|event| {
        event.spell_id == Some(AIMED_SHOT) && event.kind == CombatLogKind::Damage
    }));
    h.inbox
        .charges
        .first()
        .expect("actual consumed charge receipt")
        .clone()
}

#[test]
fn class_completeness_native_charge_rate_trueshot_accelerates_actual_aimed_shot_recharge() {
    let mut base = fixture();
    let normal = cast_aimed_shot(&mut base);
    let mut h = fixture();
    apply_charge_rate(&mut h);
    let fast = cast_aimed_shot(&mut h);
    assert_eq!(fast.category, 1715);
    assert_eq!(fast.current, fast.max - 1);
    assert_eq!(
        fast.recharge_ms,
        (normal.recharge_ms as f64 / 1.4).floor() as u32
    );
}

#[test]
fn class_completeness_native_charge_rate_retimes_running_aimed_shot_and_notifies_owner() {
    let mut h = fixture();
    cast_aimed_shot(&mut h);
    let before = h.inbox.charges.last().unwrap().clone();
    let remaining_ms = remaining_ms(&h);
    h.inbox.charges.clear();
    apply_charge_rate(&mut h);
    h.advance(3);
    let after = h.inbox.charges.last().expect("owner charge update");
    let expected = (remaining_ms as f64 / 1.4).floor() as u32;
    assert!(
        after.remaining_ms.abs_diff(expected) <= 200,
        "{before:?} -> {after:?}"
    );
    assert_eq!(
        after.recharge_ms,
        (before.recharge_ms as f64 / 1.4).floor() as u32
    );
}

#[test]
fn class_completeness_native_charge_rate_removal_restores_only_remaining_progress() {
    let mut h = fixture();
    apply_charge_rate(&mut h);
    cast_aimed_shot(&mut h);
    let remaining = remaining_ms(&h);
    h.inbox.charges.clear();
    remove_auras(
        h.app.world_mut(),
        h.caster,
        RemoveReason::Cancelled,
        |aura| aura.spell_id == TRUESHOT,
    );
    h.advance(3);
    let after = h.inbox.charges.last().expect("removal owner receipt");
    assert_eq!(after.recharge_ms, 15_000);
    let expected = (remaining as f64 * 15_000.0 / 10_714.0) as u32;
    assert!(after.remaining_ms.abs_diff(expected) <= 200, "{after:?}");
}

#[test]
fn class_completeness_native_charge_rate_unchanged_refresh_does_not_double_acceleration() {
    let mut h = fixture();
    apply_charge_rate(&mut h);
    cast_aimed_shot(&mut h);
    let remaining = remaining_ms(&h);
    apply_charge_rate(&mut h);
    h.advance(3);
    assert!(remaining_ms(&h).abs_diff(remaining.saturating_sub(150)) <= 50);
    let next = cast_aimed_shot(&mut h);
    assert_eq!(next.recharge_ms, 10_714);
    assert_eq!(next.current, 0);
}

#[test]
fn class_completeness_native_charge_rate_native_expiry_retimes_second_queued_charge() {
    let mut h = fixture();
    cast_aimed_shot(&mut h);
    let second = cast_aimed_shot(&mut h);
    assert_eq!(second.current, 0);
    apply_charge_rate(&mut h);
    h.advance(3);
    assert_eq!(h.inbox.charges.last().unwrap().recharge_ms, 10_714);
    h.inbox.charges.clear();
    // Both consumed charges recover serially; the second remains running
    // when the native15s aura expires, restoring only its unfinished work.
    h.advance(300);
    let after = h.inbox.charges.last().expect("expiry owner receipt");
    assert_eq!(after.recharge_ms, 15_000);
    assert_eq!(after.current, 1);
    assert!(after.remaining_ms < 2_000, "{after:?}");
    h.advance(45);
    assert_eq!(h.inbox.charges.last().unwrap().current, 2);
    assert_eq!(h.inbox.charges.last().unwrap().remaining_ms, 0);
}

#[test]
fn class_completeness_native_charge_rate_foreign_recipient_does_not_accelerate_caster() {
    let mut h = fixture();
    let other = spawn_player(&mut h.app, 3, 2.0);
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: other,
            spell_id: TRUESHOT,
            effect_mask: 1 << 1,
            source: AuraSource::Cast,
        },
    );
    assert_eq!(cast_aimed_shot(&mut h).recharge_ms, 15_000);
}

#[test]
fn class_completeness_native_charge_rate_other_category_has_no_effect() {
    // Fiery Rush148 uses category1500, not Aimed Shot1715.
    let mut h = harness(Vec::new(), &[AIMED_SHOT, 383_637], 3).unwrap();
    h.app.world_mut().get_mut::<Position>(h.npc).unwrap().x = 10.0;
    *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
        current: 20_000.0,
        max: 20_000.0,
    };
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 383_637,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    assert_eq!(cast_aimed_shot(&mut h).recharge_ms, 15_000);
}

#[test]
fn class_completeness_native_charge_rate_native_same_category_effects_multiply() {
    let mut h = harness(Vec::new(), &[AIMED_SHOT, TRUESHOT, 321_461], 3).unwrap();
    h.app.world_mut().get_mut::<Position>(h.npc).unwrap().x = 10.0;
    *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
        current: 20_000.0,
        max: 20_000.0,
    };
    apply_charge_rate(&mut h);
    // Native Quick Shot148 amount150 on1715; independent product, not sum.
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 321_461,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    assert_eq!(cast_aimed_shot(&mut h).recharge_ms, 4_285);
}

#[test]
fn class_completeness_native_charge_rate_negative_demonic_resolve_slows_fiery_brand() {
    let mut base = harness(Vec::new(), &[204_021], 12).unwrap();
    base.cast_and_deliver(204_021, Some(base.npc));
    base.advance(3);
    assert!(base.inbox.failed.is_empty(), "{:?}", base.inbox.failed);
    let normal = base.inbox.charges.first().unwrap().clone();
    let mut h = harness(Vec::new(), &[204_021, 331_227], 12).unwrap();
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 331_227,
            effect_mask: 1 << 1,
            source: AuraSource::Cast,
        },
    );
    h.cast_and_deliver(204_021, Some(h.npc));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let slow = h.inbox.charges.first().unwrap();
    assert_eq!(slow.category, 1541);
    // Native negative25 slows recovery, unlike454's duration multiplier.
    assert_eq!(
        slow.recharge_ms,
        (normal.recharge_ms as f64 / 0.75).floor() as u32
    );
}

#[test]
fn class_completeness_native_charge_rate_per_cast_amount_refresh_preserves_progress() {
    let mut h = fixture();
    apply_charge_rate(&mut h);
    cast_aimed_shot(&mut h);
    let remaining = remaining_ms(&h);
    let store = h.app.world().resource::<crate::spell_info::SpellStore>();
    let mut refreshed = store.get(TRUESHOT).unwrap().clone();
    // Exercise the per-cast amount boundary with native Quick Shot150.
    // This fixture does NOT assert a Retail binding from Quick Shot to Trueshot.
    let quick = super::test_harness::world_db()
        .unwrap()
        .store
        .get(321_461)
        .unwrap()
        .clone();
    refreshed
        .effects
        .iter_mut()
        .find(|e| e.index == 1)
        .unwrap()
        .base_points = quick
        .effects
        .iter()
        .find(|e| e.aura == 148)
        .unwrap()
        .base_points;
    crate::auras::apply_aura_with_spell(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: TRUESHOT,
            effect_mask: 1 << 1,
            source: AuraSource::Cast,
        },
        &refreshed,
    );
    h.inbox.charges.clear();
    h.advance(3);
    let after = h
        .inbox
        .charges
        .last()
        .expect("amount refresh owner receipt");
    assert_eq!(after.recharge_ms, 6_000);
    let expected = (remaining as f64 * 6_000.0 / 10_714.0) as u32;
    assert!(after.remaining_ms.abs_diff(expected) <= 200, "{after:?}");
}
