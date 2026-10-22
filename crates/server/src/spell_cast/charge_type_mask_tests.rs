//! TC GetChargeRecoveryTime: native205 TypeMask and453 category remain independent.
use super::test_harness::*;
use bevy::prelude::*;
use shared::protocol::SpellChargesUpdate;

const SWIFT_JUSTICE: u32 = 383228;
const JUDGMENT: u32 = 20271;
const CRUSADER_STRIKE: u32 = 35395;

fn fixture() -> Harness {
    let mut h = harness(Vec::new(), &[SWIFT_JUSTICE, JUDGMENT, CRUSADER_STRIKE], 2).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            JUDGMENT,
            CRUSADER_STRIKE,
        ]));
    h.advance(3);
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == SWIFT_JUSTICE,
    );
    h.advance(2);
    h
}

fn apply(h: &mut Harness, target: Entity, effect_mask: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target,
            spell_id: SWIFT_JUSTICE,
            effect_mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

fn cast(h: &mut Harness, spell_id: u32) -> SpellChargesUpdate {
    h.inbox.charges.clear();
    h.inbox.failed.clear();
    h.cast_and_deliver(spell_id, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.inbox
        .charges
        .iter()
        .find(|row| row.spell_id == spell_id)
        .expect("actual charge receipt")
        .clone()
}

#[test]
fn census_tail_205_native_swift_justice_reduces_only_matching_category_type() {
    let mut h = fixture();
    let caster = h.caster;
    apply(&mut h, caster, 1);
    let judgment = cast(&mut h, JUDGMENT);
    assert_eq!(judgment.recharge_ms, 9000);
    h.advance(35);
    assert_eq!(cast(&mut h, CRUSADER_STRIKE).recharge_ms, 6000);
}

#[test]
fn census_tail_205_native_whole_swift_justice_keeps_both_authored_fields() {
    let mut h = fixture();
    let caster = h.caster;
    apply(&mut h, caster, 3);
    assert_eq!(cast(&mut h, JUDGMENT).recharge_ms, 9000);
    h.advance(35);
    assert_eq!(cast(&mut h, CRUSADER_STRIKE).recharge_ms, 4000);
}

#[test]
fn census_tail_205_native_change_keeps_pending_charge_then_changes_future_consumption() {
    let mut h = fixture();
    let first = cast(&mut h, JUDGMENT);
    assert_eq!(first.recharge_ms, 11000);
    h.advance(40);
    let caster = h.caster;
    apply(&mut h, caster, 1);
    h.advance(1);
    let category = world_db()
        .unwrap()
        .store
        .get(JUDGMENT)
        .unwrap()
        .categories
        .as_ref()
        .unwrap()
        .charge_category;
    let now = crate::auras::aura_now(h.app.world());
    let snapshot = h
        .app
        .world()
        .get::<super::SpellHistory>(caster)
        .unwrap()
        .charges[&category]
        .snapshot(now);
    assert_eq!(snapshot.recharge_ms, 11000);
    assert!(
        (8000..=9100).contains(&snapshot.remaining_ms),
        "unchanged existing11s clock{snapshot:?}"
    );
    h.advance(200);
    assert_eq!(cast(&mut h, JUDGMENT).recharge_ms, 9000);
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == SWIFT_JUSTICE,
    );
    h.advance(200);
    assert_eq!(cast(&mut h, JUDGMENT).recharge_ms, 11000);
}

#[test]
fn census_tail_205_native_foreign_swift_justice_does_not_modify_caster() {
    let mut h = fixture();
    let npc = h.npc;
    apply(&mut h, npc, 1);
    assert_eq!(cast(&mut h, JUDGMENT).recharge_ms, 11000);
}
