//! TC a352b1fa spell_dh.cpp480-519: actual Fire damage accumulates a
//! native effect0 percentage for the next PERIODIC_DUMMY heal, then resets.
use super::*;
use crate::spell_cast::test_harness::{Harness, harness, spawn_player};
use shared::components::Health;
use shared::protocol::CombatLogKind;

const CHARRED: u32 = 213_010;
const HEAL: u32 = 213_011;
/// Native Chaos school includes Fire; native damage, coefficients and proc flags.
const CHAOS_HIT: u32 = 201_428;
const FIRE_HIT: u32 = 258_921;
const PHYSICAL_HIT: u32 = 203_795;
const ICE_BARRIER: u32 = 11_426;

fn fixture() -> Harness {
    let mut h = harness(
        Vec::new(),
        &[
            CHARRED,
            HEAL,
            CHAOS_HIT,
            FIRE_HIT,
            PHYSICAL_HIT,
            ICE_BARRIER,
        ],
        12,
    )
    .unwrap();
    *h.app.world_mut().get_mut::<Health>(h.caster).unwrap() = Health {
        current: 500.0,
        max: 5000.0,
    };
    let unit = h.caster;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: CHARRED,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    h
}

fn hit(h: &mut Harness, spell: u32) -> u32 {
    let start = h.inbox.log.len();
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, spell, Some(h.npc), 1);
    h.advance(1);
    h.inbox.log[start..]
        .iter()
        .filter(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(spell))
        .map(|event| u32::try_from(event.amount).unwrap())
        .sum()
}

fn expected_heal(h: &Harness, damage: u32) -> u32 {
    let percent = h
        .app
        .world()
        .get::<ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == CHARRED)
        .unwrap()
        .effects[0]
        .amount;
    (damage as f32 * percent / 100.0) as u32
}

#[test]
fn class_completeness_native_charred_warblades_accumulates_fire_damage_then_heals_once() {
    let mut h = fixture();
    let first = hit(&mut h, CHAOS_HIT);
    let second = hit(&mut h, FIRE_HIT);
    assert!(first > 0 && second > 0, "real damage precedes heal proof");
    let expected = expected_heal(&h, first) + expected_heal(&h, second);
    assert!(expected > 0);
    assert_eq!(
        h.health(h.caster),
        500.0,
        "heal waits for native 1000ms period"
    );
    h.advance(22);
    assert_eq!(
        h.health(h.caster),
        500.0 + expected as f32,
        "actual Fire damage must supply accumulated self-heal"
    );
    let count = h
        .inbox
        .log
        .iter()
        .filter(|event| event.kind == CombatLogKind::Heal && event.spell_id == Some(HEAL))
        .count();
    assert_eq!(count, 1);
    h.advance(22);
    assert_eq!(
        h.health(h.caster),
        500.0 + expected as f32,
        "no damage means no repeated heal"
    );
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(|event| event.kind == CombatLogKind::Heal && event.spell_id == Some(HEAL))
            .count(),
        count
    );
}

#[test]
fn class_completeness_native_charred_warblades_rejects_non_fire_and_foreign_actor_damage() {
    let mut h = fixture();
    assert!(hit(&mut h, PHYSICAL_HIT) > 0);
    let other = spawn_player(&mut h.app, 12, 2.0);
    let target_before = h.health(h.npc);
    crate::spell_cast::cast_triggered(h.app.world_mut(), other, FIRE_HIT, Some(h.npc), 1);
    h.advance(22);
    assert!(
        h.health(h.npc) < target_before,
        "foreign actor dealt real Fire damage"
    );
    assert_eq!(h.health(h.caster), 500.0);
    assert!(!h.inbox.log.iter().any(|event| event.spell_id == Some(HEAL)));
}

#[test]
fn class_completeness_native_charred_warblades_truncates_each_hit_before_accumulating() {
    let mut h = fixture();
    let mut expected = 0;
    for _ in 0..3 {
        let damage = hit(&mut h, FIRE_HIT);
        assert!(damage > 0);
        expected += expected_heal(&h, damage);
    }
    h.advance(22);
    assert_eq!(h.health(h.caster), 500.0 + expected as f32);
    let next = hit(&mut h, FIRE_HIT);
    let later = expected_heal(&h, next);
    h.advance(22);
    assert_eq!(h.health(h.caster), 500.0 + (expected + later) as f32);
}

#[test]
fn class_completeness_native_charred_warblades_removal_discards_pending_heals() {
    let mut h = fixture();
    assert!(hit(&mut h, FIRE_HIT) > 0);
    let unit = h.caster;
    remove_auras(h.app.world_mut(), unit, RemoveReason::Cancelled, |aura| {
        aura.spell_id == CHARRED
    });
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: CHARRED,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    h.advance(22);
    assert_eq!(
        h.health(unit),
        500.0,
        "replacement instance cannot inherit stale accumulated damage"
    );
    let new_damage = hit(&mut h, FIRE_HIT);
    let expected = expected_heal(&h, new_damage);
    h.advance(22);
    assert_eq!(h.health(unit), 500.0 + expected as f32);
}

#[test]
fn class_completeness_native_charred_warblades_refresh_keeps_pending_heals() {
    let mut h = fixture();
    let damage = hit(&mut h, FIRE_HIT);
    let expected = expected_heal(&h, damage);
    let unit = h.caster;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: CHARRED,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    h.advance(22);
    assert_eq!(h.health(unit), 500.0 + expected as f32);
}

#[test]
fn class_completeness_native_charred_warblades_fully_absorbed_fire_does_not_heal() {
    let mut h = fixture();
    let source = h.caster;
    let target = h.npc;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: source,
            target,
            spell_id: ICE_BARRIER,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    let before = h.health(target);
    let damage = hit(&mut h, FIRE_HIT);
    assert_eq!(damage, 0, "real native barrier fully absorbs the hit");
    assert_eq!(h.health(target), before);
    h.advance(22);
    assert_eq!(h.health(source), 500.0);
    assert!(!h.inbox.log.iter().any(|event| event.spell_id == Some(HEAL)));
}
