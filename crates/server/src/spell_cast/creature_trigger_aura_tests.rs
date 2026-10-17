//! Native metadata on UnitAI's distinct creature_cast(triggered=true) route.
use bevy::prelude::Entity;
use shared::spell_data::CastFailReason;

use super::test_harness::*;
use crate::auras::{ActiveAuras, has_aura};

const SHEILUN_GIFT: u32 = 205406;
const SHEILUN_CLOUD: u32 = 214502;
const STEALTH: u32 = 1784;
const SHADOW_DANCE: u32 = 185313;
const DIGESTIVE_SPITTLE: u32 = 449285;
const HAMMER_OF_JUSTICE: u32 = 853;
const SLOW_FALL: u32 = 130;
const NO_FEATHER_FALL: u32 = 79636;
const SHATTERING_HIGHLIGHT: u32 = 1293610;
const DIVINE_SHIELD: u32 = 642;
const HEADLOCK: u32 = 440490;
const FROSTBOLT: u32 = 116;

fn native_fixture() -> Harness {
    let data = world_db().unwrap();
    let spells = [
        SHEILUN_GIFT,
        SHEILUN_CLOUD,
        STEALTH,
        SHADOW_DANCE,
        158185,
        158188,
        98877,
        DIGESTIVE_SPITTLE,
        HAMMER_OF_JUSTICE,
        SLOW_FALL,
        NO_FEATHER_FALL,
        SHATTERING_HIGHLIGHT,
        DIVINE_SHIELD,
        HEADLOCK,
        FROSTBOLT,
    ]
    .into_iter()
    .map(|id| {
        data.store
            .get(id)
            .expect("native producer/consumer closure")
            .clone()
    })
    .collect();
    harness(spells, &[], 2).unwrap()
}

fn cast_npc(h: &mut Harness, spell: u32, target: Entity) -> Result<(), super::CastFailure> {
    super::creature::creature_cast(h.app.world_mut(), h.npc, spell, Some(target), true)
}

fn apply_native_self(h: &mut Harness, unit: Entity, spell: u32) {
    super::creature::creature_cast(h.app.world_mut(), unit, spell, Some(unit), true).unwrap();
    assert!(has_aura(h.app.world(), unit, spell));
}

fn stun_npc(h: &mut Harness) {
    super::creature::creature_cast(
        h.app.world_mut(),
        h.caster,
        HAMMER_OF_JUSTICE,
        Some(h.npc),
        false,
    )
    .unwrap();
    h.advance(3);
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(h.npc)
            .unwrap()
            .has_type(12)
    );
}

fn assert_source_failure(result: Result<(), super::CastFailure>, expected: &str) {
    let (reason, detail) = result
        .expect_err("native aura metadata must refuse")
        .to_contract();
    assert_eq!(reason, CastFailReason::InvalidTarget);
    assert_eq!(detail.as_deref(), Some(expected));
}

#[test]
fn native_creature_trigger_requires_its_own_sheilun_cloud() {
    let mut h = native_fixture();
    let target = h.npc;
    assert_source_failure(
        cast_npc(&mut h, SHEILUN_GIFT, target),
        "SPELL_FAILED_CASTER_AURASTATE",
    );
}

#[test]
fn native_creature_trigger_excludes_caster_stealth_before_shadow_dance() {
    let mut h = native_fixture();
    let caster = h.npc;
    apply_native_self(&mut h, caster, STEALTH);
    assert_source_failure(
        cast_npc(&mut h, SHADOW_DANCE, caster),
        "SPELL_FAILED_CASTER_AURASTATE",
    );
    assert!(!has_aura(h.app.world(), caster, SHADOW_DANCE));
}

#[test]
fn native_creature_trigger_excludes_caster_stun_type_without_generic_cc_check() {
    let mut h = native_fixture();
    stun_npc(&mut h);
    let target = h.caster;
    assert_source_failure(
        cast_npc(&mut h, DIGESTIVE_SPITTLE, target),
        "SPELL_FAILED_CASTER_AURASTATE",
    );
}

#[test]
fn native_creature_trigger_excludes_actual_target_no_feather_fall() {
    let mut h = native_fixture();
    let target = h.npc;
    apply_native_self(&mut h, target, NO_FEATHER_FALL);
    assert_source_failure(
        cast_npc(&mut h, SLOW_FALL, target),
        "SPELL_FAILED_TARGET_AURASTATE",
    );
    assert!(!has_aura(h.app.world(), target, SLOW_FALL));
}

#[test]
fn native_creature_trigger_requires_actual_target_immunity_type() {
    let mut h = native_fixture();
    let target = h.caster;
    assert_source_failure(
        cast_npc(&mut h, SHATTERING_HIGHLIGHT, target),
        "SPELL_FAILED_TARGET_AURASTATE",
    );
}

#[test]
fn native_creature_trigger_excludes_actual_target_divine_shield_type() {
    let mut h = native_fixture();
    let target = h.caster;
    apply_native_self(&mut h, target, DIVINE_SHIELD);
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(target)
            .unwrap()
            .has_type(485)
    );
    assert_source_failure(
        cast_npc(&mut h, HEADLOCK, target),
        "SPELL_FAILED_TARGET_AURASTATE",
    );
}

#[test]
fn native_creature_trigger_valid_producers_and_clear_exclusions_admit_all_six_fields() {
    for (spell, producer, target_player) in [
        (SHEILUN_GIFT, Some(SHEILUN_CLOUD), false),
        (SHADOW_DANCE, None, false),
        (DIGESTIVE_SPITTLE, None, true),
        (SLOW_FALL, None, false),
        (SHATTERING_HIGHLIGHT, Some(DIVINE_SHIELD), true),
        (HEADLOCK, None, true),
    ] {
        let mut h = native_fixture();
        let target = if target_player { h.caster } else { h.npc };
        if let Some(producer) = producer {
            apply_native_self(&mut h, target, producer);
        }
        cast_npc(&mut h, spell, target).unwrap();
    }
}

#[test]
fn native_creature_trigger_keeps_generic_stun_exemption_for_unrestricted_frostbolt() {
    let mut h = native_fixture();
    stun_npc(&mut h);
    let target = h.caster;
    cast_npc(&mut h, FROSTBOLT, target).unwrap();
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(h.npc)
            .unwrap()
            .has_type(12)
    );
}
