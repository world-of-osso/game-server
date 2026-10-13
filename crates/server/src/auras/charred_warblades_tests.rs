//! TC a352b1fa spell_dh.cpp480-519: actual Fire damage accumulates a
//! native effect0 percentage for the next PERIODIC_DUMMY heal, then resets.
use super::*;
use crate::spell_cast::test_harness::{Harness, harness};
use shared::components::Health;
use shared::protocol::CombatLogKind;

const CHARRED: u32 = 213_010;
const HEAL: u32 = 213_011;
/// Native Chaos school includes Fire; native damage, coefficients and proc flags.
const CHAOS_HIT: u32 = 201_428;
const FIRE_HIT: u32 = 258_921;
const PHYSICAL_HIT: u32 = 203_795;

fn fixture() -> Harness {
    let mut h = harness(
        Vec::new(),
        &[CHARRED, HEAL, CHAOS_HIT, FIRE_HIT, PHYSICAL_HIT],
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
