//! Native70448 aura504; TC a352b1fa Unit.cpp7579–7594.
use super::test_harness::*;
use crate::auras::{self, AuraApplication, AuraSource, RemoveReason};
use bevy::prelude::*;
use shared::protocol::CombatLogKind;

const SELF_EMBALMING_KIT: u32 = 357_556;
const SPHERE_OF_HOPE: u32 = 411_036;
const GIFT_OF_ETERNUS: u32 = 1_271_318;
const REVERSION: u32 = 366_155;
const REJUVENATION: u32 = 774;
const COPIED_REJUVENATION: u32 = 70_691;
const DARK_REGENERATION: u32 = 108_359;

fn fixture() -> Harness {
    harness(
        Vec::new(),
        &[
            SELF_EMBALMING_KIT,
            SPHERE_OF_HOPE,
            GIFT_OF_ETERNUS,
            REVERSION,
            REJUVENATION,
            COPIED_REJUVENATION,
            DARK_REGENERATION,
        ],
        5,
    )
    .unwrap()
}

fn apply(h: &mut Harness, spell_id: u32, caster: Entity, target: Entity) {
    let index = if spell_id == REVERSION || spell_id == GIFT_OF_ETERNUS {
        1
    } else {
        0
    };
    auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target,
            spell_id,
            effect_mask: 1 << index,
            source: AuraSource::Cast,
        },
    );
    assert!(auras::has_aura(h.app.world(), target, spell_id));
}

fn remove(h: &mut Harness, spell_id: u32) {
    let target = h.caster;
    auras::remove_auras(h.app.world_mut(), target, RemoveReason::Cancelled, |aura| {
        aura.spell_id == spell_id
    });
}

fn healed_amount(h: &mut Harness, spell_id: u32, periodic: bool) -> i32 {
    let spell = world_db().unwrap().store.get(spell_id).unwrap();
    let ctx = super::effects::EffectCtx {
        caster: h.caster,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    let target = h.caster;
    super::effects::apply_heal(h.app.world_mut(), &ctx, target, 100.0, false, periodic);
    h.advance(3);
    h.inbox
        .log
        .iter()
        .rfind(|event| event.kind == CombatLogKind::Heal && event.spell_id == Some(spell_id))
        .unwrap()
        .amount
}

#[test]
#[ignore = "census-tail504 RED before native caster-owned healing consumer"]
fn census_tail_504_native_direct_and_periodic_healing_follow_aura_caster_and_remove() {
    let mut h = fixture();
    let target = h.caster;
    apply(&mut h, SELF_EMBALMING_KIT, target, target);
    for periodic in [false, true] {
        assert_eq!(healed_amount(&mut h, REJUVENATION, periodic), 150);
    }
    remove(&mut h, SELF_EMBALMING_KIT);
    assert_eq!(healed_amount(&mut h, REJUVENATION, true), 100);
}

#[test]
#[ignore = "census-tail504 RED before native caster-owned healing consumer"]
fn census_tail_504_native_independent_signed_modifiers_multiply_not_sum_or_max() {
    let mut h = fixture();
    let target = h.caster;
    apply(&mut h, SELF_EMBALMING_KIT, target, target);
    apply(&mut h, SPHERE_OF_HOPE, target, target);
    assert_eq!(healed_amount(&mut h, REJUVENATION, false), 165);
    apply(&mut h, GIFT_OF_ETERNUS, target, target);
    assert_eq!(healed_amount(&mut h, REJUVENATION, false), 150);
    remove(&mut h, SELF_EMBALMING_KIT);
    assert_eq!(healed_amount(&mut h, REJUVENATION, false), 100);
    remove(&mut h, GIFT_OF_ETERNUS);
    assert_eq!(healed_amount(&mut h, REJUVENATION, false), 110);
}

#[test]
#[ignore = "census-tail504 RED before native caster-owned healing consumer"]
fn census_tail_504_native_foreign_caster_and_other_recipient_do_not_apply() {
    let mut h = fixture();
    let target = h.caster;
    let foreign = h.npc;
    apply(&mut h, SELF_EMBALMING_KIT, foreign, target);
    apply(&mut h, SPHERE_OF_HOPE, target, foreign);
    assert_eq!(healed_amount(&mut h, REJUVENATION, false), 100);
    remove(&mut h, SELF_EMBALMING_KIT);
    apply(&mut h, SELF_EMBALMING_KIT, target, target);
    assert_eq!(healed_amount(&mut h, REJUVENATION, false), 150);
}

#[test]
#[ignore = "census-tail504 RED before native caster-owned healing consumer"]
fn census_tail_504_native_ignore_healing_flags_keep_only_permitted_negative() {
    let mut h = fixture();
    let target = h.caster;
    apply(&mut h, SELF_EMBALMING_KIT, target, target);
    apply(&mut h, GIFT_OF_ETERNUS, target, target);
    assert_eq!(healed_amount(&mut h, COPIED_REJUVENATION, true), 100);
    assert_eq!(healed_amount(&mut h, DARK_REGENERATION, true), 91);
    assert_eq!(healed_amount(&mut h, REJUVENATION, true), 136);
}

#[test]
#[ignore = "census-tail504 RED before native caster-owned healing consumer"]
fn census_tail_504_native_zero_reversion_does_not_invent_healing_bonus() {
    let mut h = fixture();
    let target = h.caster;
    apply(&mut h, REVERSION, target, target);
    assert_eq!(healed_amount(&mut h, REJUVENATION, false), 100);
    assert_eq!(healed_amount(&mut h, REJUVENATION, true), 100);
}
