//! Native449285 excludes caster type12 even with a zero-amount stun effect.
use shared::spell_data::CastFailReason;

use super::test_harness::*;
use crate::auras::{ActiveAuras, RemoveReason, has_aura, remove_auras};

const DIGESTIVE_SPITTLE: u32 = 449285;
const HAMMER_OF_JUSTICE: u32 = 853;
const MOD_STUN: u32 = 12;

fn has_stun_type(h: &Harness) -> bool {
    h.app
        .world()
        .get::<ActiveAuras>(h.npc)
        .is_some_and(|auras| auras.has_type(MOD_STUN))
}

fn stunned_fixture() -> Harness {
    let data = world_db().unwrap();
    let extra = vec![data.store.get(DIGESTIVE_SPITTLE).unwrap().clone()];
    let mut h = harness(extra, &[HAMMER_OF_JUSTICE], 2).unwrap();
    super::creature::creature_cast(
        h.app.world_mut(),
        h.caster,
        HAMMER_OF_JUSTICE,
        Some(h.npc),
        false,
    )
    .unwrap();
    h.advance(3);
    assert!(has_aura(h.app.world(), h.npc, HAMMER_OF_JUSTICE));
    assert!(has_stun_type(&h));
    h
}

#[test]
fn native_excluded_caster_type_has_source_priority_before_generic_stun_refusal() {
    let mut h = stunned_fixture();
    let error = super::creature::creature_cast(
        h.app.world_mut(),
        h.npc,
        DIGESTIVE_SPITTLE,
        Some(h.caster),
        false,
    )
    .expect_err("native caster type exclusion");
    let (reason, detail) = error.to_contract();
    assert_eq!(reason, CastFailReason::InvalidTarget);
    assert_eq!(detail.as_deref(), Some("SPELL_FAILED_CASTER_AURASTATE"));
}

#[test]
fn native_excluded_caster_type_is_not_ignored_by_full_trigger_crowd_control_flags() {
    let mut h = stunned_fixture();
    assert!(!super::try_cast_triggered(
        h.app.world_mut(),
        h.npc,
        DIGESTIVE_SPITTLE,
        Some(h.caster),
        0,
    ));
    assert!(has_stun_type(&h));
}

#[test]
fn native_excluded_caster_type_does_not_borrow_the_targets_stun() {
    let mut h = stunned_fixture();
    assert!(super::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        DIGESTIVE_SPITTLE,
        Some(h.npc),
        0,
    ));
    assert!(has_stun_type(&h));
}

#[test]
fn native_excluded_caster_type_removal_and_expiry_restore_both_cast_routes() {
    for expire in [false, true] {
        let mut h = stunned_fixture();
        if expire {
            h.advance(121); // Native Hammer of Justice has a six-second duration.
        } else {
            remove_auras(h.app.world_mut(), h.npc, RemoveReason::Cancelled, |aura| {
                aura.spell_id == HAMMER_OF_JUSTICE
            });
        }
        assert!(!has_stun_type(&h));
        assert!(super::try_cast_triggered(
            h.app.world_mut(),
            h.npc,
            DIGESTIVE_SPITTLE,
            Some(h.caster),
            0,
        ));
        super::creature::creature_cast(
            h.app.world_mut(),
            h.npc,
            DIGESTIVE_SPITTLE,
            Some(h.caster),
            false,
        )
        .unwrap();
    }
}
