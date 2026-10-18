//! Native target aura-type metadata; TC SpellInfo.cpp2513–2517.
//! Highlight rows are internal passive indicators, not client-learnable abilities.
use bevy::prelude::Entity;
use shared::spell_data::CastFailReason;

use super::test_harness::*;
use crate::auras::{ActiveAuras, RemoveReason, remove_auras};

const WRECKING_HIGHLIGHT: u32 = 1293603;
const SHATTERING_HIGHLIGHT: u32 = 1293610;
const HEADLOCK: u32 = 440490;
const HEADLOCK_VARIANT: u32 = 448026;
const POWER_WORD_SHIELD: u32 = 17;
const DIVINE_SHIELD: u32 = 642;
const NO_FEATHER_FALL: u32 = 79636;
const PRIEST: u32 = 137030;
const DISCIPLINE_PRIEST: u32 = 137032;
const SCHOOL_ABSORB: u32 = 69;
const SCHOOL_IMMUNITY: u32 = 39;
const HEADLOCK_EXCLUDED_TYPE: u32 = 485;

fn native_fixture() -> Harness {
    let data = world_db().unwrap();
    let internal = [
        WRECKING_HIGHLIGHT,
        SHATTERING_HIGHLIGHT,
        HEADLOCK,
        HEADLOCK_VARIANT,
        DIVINE_SHIELD,
        NO_FEATHER_FALL,
    ]
    .into_iter()
    .map(|id| data.store.get(id).expect("native spell").clone())
    .collect();
    harness(internal, &[POWER_WORD_SHIELD, PRIEST, DISCIPLINE_PRIEST], 5).unwrap()
}

fn has_type(h: &Harness, target: Entity, aura_type: u32) -> bool {
    h.app
        .world()
        .get::<ActiveAuras>(target)
        .is_some_and(|auras| auras.has_type(aura_type))
}

fn cast_at(h: &mut Harness, spell_id: u32, target: Entity) -> Result<(), super::CastFailure> {
    let caster = h.npc;
    super::creature::creature_cast(h.app.world_mut(), caster, spell_id, Some(target), false)
}

fn apply_native_self_spell(h: &mut Harness, unit: Entity, spell_id: u32) {
    super::creature::creature_cast(h.app.world_mut(), unit, spell_id, Some(unit), false).unwrap();
    h.advance(31); // Drain normal delivery and the existing1.5s player GCD.
    assert!(crate::auras::has_aura(h.app.world(), unit, spell_id));
    h.inbox = Inbox::default();
}

fn assert_target_type_failure(result: Result<(), super::CastFailure>) {
    let failure = result.expect_err("native target aura-type restriction");
    let (reason, detail) = failure.to_contract().unwrap();
    assert_eq!(reason, CastFailReason::InvalidTarget);
    assert_eq!(detail.as_deref(), Some("SPELL_FAILED_TARGET_AURASTATE"));
}

#[test]
fn native_required_target_aura_types_refuse_both_unshielded_highlights() {
    for (spell, required) in [
        (WRECKING_HIGHLIGHT, SCHOOL_ABSORB),
        (SHATTERING_HIGHLIGHT, SCHOOL_IMMUNITY),
    ] {
        let mut h = native_fixture();
        let target = h.caster;
        assert!(!has_type(&h, target, required));
        assert_target_type_failure(cast_at(&mut h, spell, target));
        h.advance(3);
        assert!(!h.inbox.go.iter().any(|go| go.spell_id == spell));
    }
}

#[test]
fn native_required_target_aura_types_accept_the_matching_actual_shields() {
    for (spell, shield, required) in [
        (WRECKING_HIGHLIGHT, POWER_WORD_SHIELD, SCHOOL_ABSORB),
        (SHATTERING_HIGHLIGHT, DIVINE_SHIELD, SCHOOL_IMMUNITY),
    ] {
        let mut h = native_fixture();
        let target = h.caster;
        apply_native_self_spell(&mut h, target, shield);
        assert!(has_type(&h, target, required));
        cast_at(&mut h, spell, target).unwrap();
        h.advance(3);
        assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
    }
}

#[test]
fn native_required_target_aura_types_do_not_accept_an_unrelated_dummy_type() {
    for spell in [WRECKING_HIGHLIGHT, SHATTERING_HIGHLIGHT] {
        let mut h = native_fixture();
        let target = h.caster;
        apply_native_self_spell(&mut h, target, NO_FEATHER_FALL);
        assert_target_type_failure(cast_at(&mut h, spell, target));
    }
}

#[test]
fn native_required_target_aura_types_do_not_borrow_caster_or_other_unit_shields() {
    let mut h = native_fixture();
    let caster = h.npc;
    let target = h.caster;
    apply_native_self_spell(&mut h, caster, DIVINE_SHIELD);
    assert!(has_type(&h, caster, SCHOOL_IMMUNITY));
    assert!(!has_type(&h, target, SCHOOL_IMMUNITY));
    assert_target_type_failure(cast_at(&mut h, SHATTERING_HIGHLIGHT, target));

    let mut h = native_fixture();
    let other = spawn_player(&mut h.app, 5, 2.0);
    let target = h.caster;
    apply_native_self_spell(&mut h, other, POWER_WORD_SHIELD);
    assert!(has_type(&h, other, SCHOOL_ABSORB));
    assert!(!has_type(&h, target, SCHOOL_ABSORB));
    assert_target_type_failure(cast_at(&mut h, WRECKING_HIGHLIGHT, target));
}

#[test]
fn native_excluded_target_aura_type_refuses_both_headlock_variants() {
    for spell in [HEADLOCK, HEADLOCK_VARIANT] {
        let mut h = native_fixture();
        let target = h.caster;
        apply_native_self_spell(&mut h, target, DIVINE_SHIELD);
        assert!(has_type(&h, target, HEADLOCK_EXCLUDED_TYPE));
        assert_target_type_failure(cast_at(&mut h, spell, target));
        h.advance(3);
        assert!(!h.inbox.go.iter().any(|go| go.spell_id == spell));
        assert!(has_type(&h, target, HEADLOCK_EXCLUDED_TYPE));
    }
}

#[test]
fn native_excluded_target_aura_type_on_caster_does_not_refuse_unshielded_target() {
    let mut h = native_fixture();
    let caster = h.npc;
    let target = h.caster;
    apply_native_self_spell(&mut h, caster, DIVINE_SHIELD);
    assert!(has_type(&h, caster, HEADLOCK_EXCLUDED_TYPE));
    assert!(!has_type(&h, target, HEADLOCK_EXCLUDED_TYPE));
    cast_at(&mut h, HEADLOCK, target).unwrap();
    h.advance(3);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == HEADLOCK));
}

#[test]
fn native_target_aura_type_removal_and_expiry_reverse_both_conditions() {
    for expire in [false, true] {
        let mut h = native_fixture();
        let target = h.caster;
        apply_native_self_spell(&mut h, target, DIVINE_SHIELD);
        cast_at(&mut h, SHATTERING_HIGHLIGHT, target).unwrap();
        if expire {
            h.advance(161); // Exact native eight-second Divine Shield expires.
        } else {
            remove_auras(h.app.world_mut(), target, RemoveReason::Cancelled, |aura| {
                aura.spell_id == DIVINE_SHIELD
            });
        }
        assert!(!has_type(&h, target, SCHOOL_IMMUNITY));
        assert!(!has_type(&h, target, HEADLOCK_EXCLUDED_TYPE));
        h.inbox = Inbox::default();
        assert_target_type_failure(cast_at(&mut h, SHATTERING_HIGHLIGHT, target));
        cast_at(&mut h, HEADLOCK, target).unwrap();
        h.advance(3);
        assert!(h.inbox.go.iter().any(|go| go.spell_id == HEADLOCK));
    }
}
