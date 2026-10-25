//! TC a352b1fa SpellInfo.cpp4140-4144: power/family-masked aura63
//! changes optional spend, never the base admission cost.
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

use super::powers::PowerPools;
use super::test_harness::{Harness, harness};
use crate::class_progression::KnownSpells;

const DEEPER_STRATAGEM: u32 = 193_531;
const DEVIOUS_STRATAGEM: u32 = 394_321;
const ENVENOM: u32 = 32_645;
const ANTICIPATION: u32 = 114_015;
const FANG_STRIKE: u32 = 1_301_405;
const ROGUE: u8 = 4;

fn native_harness(talents: &[u32], expected_cap: f32) -> Harness {
    let mut known = talents.to_vec();
    known.push(ENVENOM);
    let mut h = harness(Vec::new(), &known, ROGUE).expect("native70391 finisher fixtures");
    h.advance(8);
    let cap = h
        .app
        .world()
        .get::<PowerPools>(h.caster)
        .unwrap()
        .get(PowerType::ComboPoints)
        .unwrap()
        .max;
    assert_eq!(cap, expected_cap, "actual passive-derived cap before input");
    h
}

fn cast_finisher(h: &mut Harness, points: f32, expected_remaining: f32) {
    // Concrete available power, within the asserted native passive-derived cap.
    h.set_power(PowerType::ComboPoints, points);
    h.set_power(PowerType::Energy, 100.0);
    h.cast_and_deliver(ENVENOM, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.log.iter().any(|event| {
        event.spell_id == Some(ENVENOM) && event.kind == CombatLogKind::CastSuccess
    }));
    assert_eq!(h.power(PowerType::ComboPoints), expected_remaining);
    // Native35 Energy base cost; at most one50ms frame of10/s regeneration.
    let energy = h.power(PowerType::Energy);
    assert!((65.0..=65.5).contains(&energy), "energy {energy}");
}

#[test]
fn additional_optional_native_stratagem_spends_six_and_seven_combo_points() {
    for talents in [
        vec![DEEPER_STRATAGEM],
        vec![DEEPER_STRATAGEM, DEVIOUS_STRATAGEM],
    ] {
        let cap = 5.0 + talents.len() as f32;
        let mut h = native_harness(&talents, cap);
        cast_finisher(&mut h, cap, 0.0);
    }
}

#[test]
fn additional_optional_native_stratagem_does_not_require_the_extra_points() {
    for points in [1.0, 3.0, 5.0] {
        let mut h = native_harness(&[DEEPER_STRATAGEM, DEVIOUS_STRATAGEM], 7.0);
        cast_finisher(&mut h, points, 0.0);
    }
    let mut ordinary = native_harness(&[], 5.0);
    cast_finisher(&mut ordinary, 5.0, 0.0);
}

#[test]
fn additional_optional_native_unlearning_keeps_only_the_remaining_stratagem() {
    let mut h = native_harness(&[DEEPER_STRATAGEM, DEVIOUS_STRATAGEM], 7.0);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(KnownSpells::fixture(vec![DEVIOUS_STRATAGEM, ENVENOM]));
    h.advance(8);
    let cap = h
        .app
        .world()
        .get::<PowerPools>(h.caster)
        .unwrap()
        .get(PowerType::ComboPoints)
        .unwrap()
        .max;
    assert_eq!(cap, 6.0);
    cast_finisher(&mut h, 6.0, 0.0);
}

#[test]
fn additional_optional_native_foreign_stratagem_does_not_increase_spend() {
    // Anticipation is an authentic cap-only fixture, not a current legal build.
    let mut h = harness(
        Vec::new(),
        &[ANTICIPATION, DEEPER_STRATAGEM, ENVENOM],
        ROGUE,
    )
    .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(KnownSpells::fixture(vec![ANTICIPATION, ENVENOM]));
    let foreign = super::test_harness::spawn_player(&mut h.app, ROGUE, 0.0);
    h.app
        .world_mut()
        .entity_mut(foreign)
        .insert(KnownSpells::fixture(vec![DEEPER_STRATAGEM]));
    h.advance(8);
    assert!(crate::auras::has_aura(
        h.app.world(),
        foreign,
        DEEPER_STRATAGEM
    ));
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.caster,
        DEEPER_STRATAGEM
    ));
    let cap = h
        .app
        .world()
        .get::<PowerPools>(h.caster)
        .unwrap()
        .get(PowerType::ComboPoints)
        .unwrap()
        .max;
    assert_eq!(cap, 10.0);
    cast_finisher(&mut h, 6.0, 1.0);
}

#[test]
fn additional_optional_native_same_power_wrong_spell_mask_does_not_change_spend() {
    let mut h = native_harness(&[ANTICIPATION, FANG_STRIKE], 10.0);
    // Native Fang Strike's-6 ComboPoint field names Dispatch, not Envenom.
    h.cast_and_deliver(FANG_STRIKE, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, FANG_STRIKE));
    cast_finisher(&mut h, 6.0, 1.0);
}
