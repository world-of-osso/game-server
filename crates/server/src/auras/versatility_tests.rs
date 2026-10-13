//! TC a352b1fa SpellAuraEffects.cpp4024, Unit.cpp6944/7036/8229:
//!471 is additive percentage points; incoming damage uses half the total.
use super::*;
use crate::spell_cast::test_harness::{Harness, harness_with, world_db};
use crate::stats::CharacterStats;
use shared::components::Health;
use shared::protocol::CombatLogKind;

const MARK_OF_THE_WILD: u32 = 1126;
const CJ_BALL: u32 = 314_633;
const FLASH_HEAL: u32 = 2061;

fn fixture() -> Harness {
    let mut heal = world_db().unwrap().store.get(FLASH_HEAL).unwrap().clone();
    // Keep all source coefficients/targets; remove only random variance to
    // distinguish the native3% bonus from its±2.5% random healing interval.
    heal.effects[0].variance = 0.0;
    let mut h = harness_with(vec![heal], &[MARK_OF_THE_WILD, CJ_BALL], 8, true).unwrap();
    h.advance(3);
    h
}

fn apply_mark(h: &mut Harness) {
    let unit = h.caster;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: MARK_OF_THE_WILD,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    h.advance(3);
}

fn fix_test_power(h: &mut Harness) {
    let mut stats = h
        .app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap();
    // Deterministic native spell-math endpoints, not a player-build claim.
    stats.spell_power = 1000.0;
    stats.healing_power = 1000.0;
    stats.crit_pct = 0.0;
}

#[test]
fn class_completeness_native_versatility_mark_updates_computed_player_percentage() {
    let mut h = fixture();
    assert_eq!(
        h.app
            .world()
            .get::<CharacterStats>(h.caster)
            .unwrap()
            .versatility_pct,
        0.0
    );
    apply_mark(&mut h);
    assert_eq!(
        h.app
            .world()
            .get::<CharacterStats>(h.caster)
            .unwrap()
            .versatility_pct,
        3.0
    );
}

#[test]
fn class_completeness_native_versatility_mark_increases_real_spell_damage() {
    let mut h = fixture();
    apply_mark(&mut h);
    fix_test_power(&mut h);
    h.cast_and_deliver(CJ_BALL, Some(h.npc));
    h.advance(60);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let damage = h
        .inbox
        .log
        .iter()
        .find(|r| r.spell_id == Some(CJ_BALL) && r.kind == CombatLogKind::Damage)
        .unwrap();
    assert!(!damage.crit);
    // Native SP0.59×1000×1.03, rounded.
    assert_eq!(damage.amount, 608);
}

#[test]
fn class_completeness_native_versatility_mark_increases_real_healing() {
    let mut h = fixture();
    apply_mark(&mut h);
    fix_test_power(&mut h);
    let unit = h.caster;
    *h.app.world_mut().get_mut::<Health>(unit).unwrap() = Health {
        current: 1.0,
        max: 20_000.0,
    };
    h.cast_and_deliver(FLASH_HEAL, Some(unit));
    h.advance(40);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let heal = h
        .inbox
        .log
        .iter()
        .find(|r| r.spell_id == Some(FLASH_HEAL) && r.kind == CombatLogKind::Heal)
        .unwrap();
    assert!(!heal.crit);
    // Native SP6.82×1000×1.03, rounded.
    assert_eq!(heal.amount, 7025);
}

#[test]
fn class_completeness_native_versatility_mark_reduces_real_incoming_damage_by_half() {
    let mut h = fixture();
    apply_mark(&mut h);
    let unit = h.caster;
    *h.app.world_mut().get_mut::<Health>(unit).unwrap() = Health {
        current: 20_000.0,
        max: 20_000.0,
    };
    h.app.world_mut().entity_mut(h.npc).insert(CharacterStats {
        spell_power: 1000.0,
        crit_pct: 0.0,
        ..Default::default()
    });
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.npc, CJ_BALL, Some(unit), 1);
    h.advance(60);
    let damage = h
        .inbox
        .log
        .iter()
        .find(|r| r.spell_id == Some(CJ_BALL) && r.kind == CombatLogKind::Damage)
        .unwrap();
    assert!(!damage.crit);
    // Native3% versatility contributes1.5% damage reduction, not3%.
    assert_eq!(damage.amount, 581);
    assert_eq!(h.health(unit), 20_000.0 - 581.0);
}
