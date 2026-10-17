//! Native70391 excluded aura-spell refs; TC Spell.cpp5883/SpellInfo.cpp2510.
use bevy::prelude::Entity;
use shared::components::{CombatStatus, Mana};
use shared::spell_data::CastFailReason;

use super::test_harness::*;

const STEALTH: u32 = 1784;
const STEALTH_AURA: u32 = 158185;
const STEALTH_SHAPESHIFT: u32 = 158188;
const SANCTUARY: u32 = 98877;
const SHADOW_DANCE: u32 = 185313;
const SLOW_FALL: u32 = 130;
const NO_FEATHER_FALL: u32 = 79636;
const CUTLASS_SLASH: u32 = 214794;
const BLOOD_FRENZY: u32 = 214789;

fn rogue() -> Harness {
    // Keep the real parent's stealth, shapeshift and sanctuary child closure.
    let mut h = harness(
        Vec::new(),
        &[
            STEALTH,
            STEALTH_AURA,
            STEALTH_SHAPESHIFT,
            SANCTUARY,
            SHADOW_DANCE,
        ],
        4,
    )
    .unwrap();
    h.app
        .world_mut()
        .get_mut::<CombatStatus>(h.caster)
        .unwrap()
        .0 = false;
    h
}

fn mage() -> Harness {
    harness(Vec::new(), &[SLOW_FALL, NO_FEATHER_FALL], 8).unwrap()
}

fn cast_self_aura(h: &mut Harness, spell_id: u32) {
    h.cast_and_deliver(spell_id, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, spell_id));
    h.advance(31); // Existing ordinary 1.5s GCD expires; native aura stays active.
    h.inbox = Inbox::default();
}

fn cast_marker_on_unit(h: &mut Harness, unit: Entity) {
    super::creature::creature_cast(h.app.world_mut(), unit, NO_FEATHER_FALL, Some(unit), false)
        .unwrap();
    h.advance(31);
    assert!(crate::auras::has_aura(h.app.world(), unit, NO_FEATHER_FALL));
    h.inbox = Inbox::default();
}

fn assert_refused_without_launch(h: &Harness, source_failure: &str) {
    assert_eq!(h.inbox.failed.len(), 1, "{:?}", h.inbox.failed);
    assert_eq!(h.inbox.failed[0].reason, CastFailReason::InvalidTarget);
    assert_eq!(h.inbox.failed[0].detail.as_deref(), Some(source_failure));
    assert!(h.inbox.go.is_empty(), "{:?}", h.inbox.go);
    assert!(h.inbox.cooldowns.is_empty());
    assert!(h.inbox.charges.is_empty());
}

#[test]
fn native_excluded_caster_aura_spells_refuse_both_stealth_dance_orders() {
    for (first, refused) in [(STEALTH, SHADOW_DANCE), (SHADOW_DANCE, STEALTH)] {
        let mut h = rogue();
        cast_self_aura(&mut h, first);
        h.cast_and_deliver(refused, None);
        assert_refused_without_launch(&h, "SPELL_FAILED_CASTER_AURASTATE");
        assert!(!crate::auras::has_aura(h.app.world(), h.caster, refused));
        assert!(crate::auras::has_aura(h.app.world(), h.caster, first));
    }
}

#[test]
fn native_excluded_target_aura_spell_refuses_slow_fall_auto_self_before_mana_payment() {
    let mut h = mage();
    cast_self_aura(&mut h, NO_FEATHER_FALL);
    let mana = h.app.world().get::<Mana>(h.caster).unwrap().current;
    h.cast_and_deliver(SLOW_FALL, None);
    assert_refused_without_launch(&h, "SPELL_FAILED_TARGET_AURASTATE");
    assert_eq!(h.app.world().get::<Mana>(h.caster).unwrap().current, mana);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, SLOW_FALL));
}

#[test]
fn native_excluded_target_aura_spell_checks_selected_ally_not_caster() {
    let mut h = mage();
    let ally = spawn_player(&mut h.app, 8, 2.0);
    cast_marker_on_unit(&mut h, ally);
    h.cast_and_deliver(SLOW_FALL, Some(ally));
    assert_refused_without_launch(&h, "SPELL_FAILED_TARGET_AURASTATE");
    assert!(!crate::auras::has_aura(h.app.world(), ally, SLOW_FALL));
    h.inbox = Inbox::default();
    h.cast_and_deliver(SLOW_FALL, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, SLOW_FALL));
}

#[test]
fn native_excluded_target_aura_on_caster_does_not_refuse_unmarked_ally() {
    let mut h = mage();
    let ally = spawn_player(&mut h.app, 8, 2.0);
    cast_self_aura(&mut h, NO_FEATHER_FALL);
    h.cast_and_deliver(SLOW_FALL, Some(ally));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), ally, SLOW_FALL));
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, SLOW_FALL));
}

#[test]
fn native_excluded_aura_spell_removal_and_expiry_restore_ordinary_cast() {
    for expire in [false, true] {
        let mut h = mage();
        cast_self_aura(&mut h, NO_FEATHER_FALL);
        if expire {
            h.advance(201); // Exact native ten-second No Feather Fall expires.
        } else {
            crate::auras::remove_auras(
                h.app.world_mut(),
                h.caster,
                crate::auras::RemoveReason::Cancelled,
                |aura| aura.spell_id == NO_FEATHER_FALL,
            );
        }
        assert!(!crate::auras::has_aura(
            h.app.world(),
            h.caster,
            NO_FEATHER_FALL
        ));
        h.inbox = Inbox::default();
        h.cast_and_deliver(SLOW_FALL, None);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert!(crate::auras::has_aura(h.app.world(), h.caster, SLOW_FALL));
    }
}

#[test]
fn native_triggered_target_aura_exclusion_is_not_the_existing_caster_bypass() {
    let mut h = rogue();
    cast_self_aura(&mut h, STEALTH);
    super::cast_triggered(h.app.world_mut(), h.caster, SHADOW_DANCE, None, 0);
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.caster,
        SHADOW_DANCE
    ));
    let mut h = mage();
    cast_self_aura(&mut h, NO_FEATHER_FALL);
    assert!(!super::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        SLOW_FALL,
        Some(h.caster),
        0,
    ));
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, SLOW_FALL));
}

#[test]
fn native_excluded_caster_aura_spell_applies_to_ordinary_npc_casts() {
    let mut h = rogue();
    super::cast_triggered(h.app.world_mut(), h.npc, STEALTH, None, 0);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, STEALTH));
    let result =
        super::creature::creature_cast(h.app.world_mut(), h.npc, SHADOW_DANCE, Some(h.npc), false);
    let failure = result.expect_err("ordinary NPC must respect native caster exclusion");
    let (reason, detail) = failure.to_contract();
    assert_eq!(reason, CastFailReason::InvalidTarget);
    assert_eq!(detail.as_deref(), Some("SPELL_FAILED_CASTER_AURASTATE"));
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, SHADOW_DANCE));
}

#[test]
fn native_excluded_caster_aura_gained_during_cast_refuses_completion() {
    let mut h = harness(Vec::new(), &[CUTLASS_SLASH, BLOOD_FRENZY], 1).unwrap();
    let health = h.health(h.npc);
    h.cast_and_deliver(CUTLASS_SLASH, Some(h.npc));
    assert!(h.app.world().get::<super::CastState>(h.caster).is_some());
    super::cast_triggered(h.app.world_mut(), h.caster, BLOOD_FRENZY, None, 0);
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.caster,
        BLOOD_FRENZY
    ));
    h.advance(12); // Native450ms cast finishes after the blocker appeared.
    assert_refused_without_launch(&h, "SPELL_FAILED_CASTER_AURASTATE");
    assert_eq!(h.health(h.npc), health);
}

#[test]
fn native_excluded_caster_aura_removed_before_completion_allows_cast() {
    let mut h = harness(Vec::new(), &[CUTLASS_SLASH, BLOOD_FRENZY], 1).unwrap();
    h.cast_and_deliver(CUTLASS_SLASH, Some(h.npc));
    assert!(h.app.world().get::<super::CastState>(h.caster).is_some());
    super::cast_triggered(h.app.world_mut(), h.caster, BLOOD_FRENZY, None, 0);
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.caster,
        BLOOD_FRENZY
    ));
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == BLOOD_FRENZY,
    );
    h.advance(12);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == CUTLASS_SLASH));
}
