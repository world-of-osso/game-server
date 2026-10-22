//! Native70448 aura259; TC a352b1fa Unit.cpp7512–7550.
use super::test_harness::*;
use crate::auras::{self, AuraApplication, AuraSource, RemoveReason};
use bevy::prelude::*;
use shared::protocol::CombatLogKind;

const LIFE_COCOON: u32 = 116_849;
const RESTORATION_BONUS: u32 = 145_384;
const RENEW: u32 = 139;
const REJUVENATION: u32 = 774;
const COPIED_REJUVENATION: u32 = 70_691;
const DARK_REGENERATION: u32 = 108_359;

fn fixture() -> Harness {
    harness(
        Vec::new(),
        &[
            LIFE_COCOON,
            RESTORATION_BONUS,
            RENEW,
            REJUVENATION,
            COPIED_REJUVENATION,
            DARK_REGENERATION,
            209_140,
        ],
        5,
    )
    .unwrap()
}

fn apply(h: &mut Harness, spell_id: u32, index: u8, target: Entity) {
    let caster = h.caster;
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

fn remove(h: &mut Harness, spell_id: u32, target: Entity) {
    auras::remove_auras(h.app.world_mut(), target, RemoveReason::Cancelled, |aura| {
        aura.spell_id == spell_id
    });
}

fn heal_receipt(h: &mut Harness, spell_id: u32, periodic: bool) -> i32 {
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

fn next_renew_tick(h: &mut Harness) -> i32 {
    let before = h.inbox.log.len();
    for _ in 0..70 {
        h.advance(1);
        if let Some(event) = h.inbox.log[before..].iter().find(|event| {
            event.kind == CombatLogKind::Heal && event.spell_id == Some(RENEW) && event.periodic
        }) {
            return event.amount;
        }
    }
    panic!("native3000ms Renew did not produce its next periodic heal");
}

#[test]
#[ignore = "census-tail259 RED before native periodic-taken consumer"]
fn census_tail_259_native_renew_ticks_use_live_recipient_modifier_then_removal() {
    let mut h = fixture();
    crate::spell_cast::creature_cast(h.app.world_mut(), h.npc, 209_140, Some(h.caster), false)
        .unwrap();
    h.advance(3);
    assert_eq!(h.health(h.caster), 200.0);
    let target = h.caster;
    apply(&mut h, RENEW, 0, target);
    assert_eq!(next_renew_tick(&mut h), 61);
    apply(&mut h, LIFE_COCOON, 1, target);
    assert_eq!(next_renew_tick(&mut h), 91);
    remove(&mut h, LIFE_COCOON, target);
    assert_eq!(next_renew_tick(&mut h), 61);
    assert_eq!(h.health(target), 413.0);
}

#[test]
#[ignore = "census-tail259 RED before native periodic-taken consumer"]
fn census_tail_259_native_strongest_positive_is_not_sum_or_product() {
    let mut h = fixture();
    let target = h.caster;
    apply(&mut h, LIFE_COCOON, 1, target);
    apply(&mut h, RESTORATION_BONUS, 2, target);
    assert_eq!(heal_receipt(&mut h, REJUVENATION, true), 150);
    remove(&mut h, LIFE_COCOON, target);
    assert_eq!(heal_receipt(&mut h, REJUVENATION, true), 125);
    remove(&mut h, RESTORATION_BONUS, target);
    assert_eq!(heal_receipt(&mut h, REJUVENATION, true), 100);
}

#[test]
#[ignore = "census-tail259 RED before native periodic-taken consumer"]
fn census_tail_259_native_direct_and_foreign_recipient_are_unmodified() {
    let mut h = fixture();
    let foreign = h.npc;
    apply(&mut h, LIFE_COCOON, 1, foreign);
    assert_eq!(heal_receipt(&mut h, REJUVENATION, true), 100);
    let target = h.caster;
    apply(&mut h, LIFE_COCOON, 1, target);
    assert_eq!(heal_receipt(&mut h, REJUVENATION, false), 100);
}

#[test]
#[ignore = "census-tail259 RED before native periodic-taken consumer"]
fn census_tail_259_native_ignore_healing_suppresses_positive_even_allow_negative() {
    let mut h = fixture();
    let target = h.caster;
    apply(&mut h, LIFE_COCOON, 1, target);
    for spell_id in [COPIED_REJUVENATION, DARK_REGENERATION] {
        let misc = world_db()
            .unwrap()
            .store
            .get(spell_id)
            .unwrap()
            .misc
            .as_ref()
            .unwrap();
        assert_ne!(misc.attributes[6] & 0x0800_0000, 0);
        assert_eq!(heal_receipt(&mut h, spell_id, true), 100);
    }
    assert_ne!(
        world_db()
            .unwrap()
            .store
            .get(DARK_REGENERATION)
            .unwrap()
            .misc
            .as_ref()
            .unwrap()
            .attributes[13]
            & 0x0400_0000,
        0
    );
    assert_eq!(heal_receipt(&mut h, REJUVENATION, true), 150);
}
