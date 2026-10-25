//! TC a352b1fa Unit.cpp3298 / Spell.cpp7175: aura330 bypasses movement interrupts by mask.
use super::test_harness::*;
use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;
use shared::spell_data::CastFailReason;

const GRACE: u32 = 79_206;
const LIGHTNING_BOLT: u32 = 188_196;
const FIREBALL: u32 = 133;
const HOVER: u32 = 358_267;
const DISINTEGRATE: u32 = 356_995;
const DISINTEGRATE_SLOW: u32 = 370_898;
const NO_DISABLE_MOVEMENT_INTERRUPT: u32 = 0x0800_0000;

fn shaman() -> Harness {
    harness(Vec::new(), &[GRACE, LIGHTNING_BOLT, FIREBALL], 7).unwrap()
}

fn apply_mobility(h: &mut Harness, id: u32) {
    let effect = world_db()
        .unwrap()
        .store
        .get(id)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.aura == 330)
        .unwrap();
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: id,
            effect_mask: 1 << effect.index,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

fn set_moving(h: &mut Harness) {
    h.app
        .world_mut()
        .get_mut::<crate::networking::Moving>(h.caster)
        .unwrap()
        .0 = true;
}

fn remove_mobility(h: &mut Harness, id: u32) {
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == id,
    );
}

#[test]
fn class_completeness_native_grace_allows_real_lightning_bolt_while_moving() {
    let mut h = shaman();
    apply_mobility(&mut h, GRACE);
    set_moving(&mut h);
    h.cast_and_deliver(LIGHTNING_BOLT, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.app
            .world()
            .get::<CastState>(h.caster)
            .unwrap()
            .cast_while_moving
    );
    h.advance(60);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage
                && event.spell_id == Some(LIGHTNING_BOLT)
                && event.amount > 0)
    );
}

#[test]
fn class_completeness_native_grace_gain_and_removal_update_an_active_cast() {
    let mut h = shaman();
    h.cast_and_deliver(LIGHTNING_BOLT, Some(h.npc));
    assert!(
        !h.app
            .world()
            .get::<CastState>(h.caster)
            .unwrap()
            .cast_while_moving
    );
    apply_mobility(&mut h, GRACE);
    assert!(
        h.app
            .world()
            .get::<CastState>(h.caster)
            .unwrap()
            .cast_while_moving
    );
    set_moving(&mut h);
    h.advance(2);
    assert!(h.app.world().get::<CastState>(h.caster).is_some());
    remove_mobility(&mut h, GRACE);
    assert!(
        !h.app
            .world()
            .get::<CastState>(h.caster)
            .unwrap()
            .cast_while_moving
    );
    h.advance(1);
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    assert!(
        h.inbox
            .go
            .iter()
            .all(|event| event.spell_id != LIGHTNING_BOLT)
    );
}

#[test]
fn class_completeness_native_grace_mask_does_not_allow_foreign_family_fireball() {
    let mut h = shaman();
    apply_mobility(&mut h, GRACE);
    set_moving(&mut h);
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    assert_eq!(
        h.inbox.failed[0].reason,
        CastFailReason::CantCastWhileMoving
    );
}

#[test]
fn class_completeness_native_grace_requires_a_matching_mask_within_its_family() {
    let data = world_db().unwrap();
    let mut spell = data.store.get(LIGHTNING_BOLT).unwrap().clone();
    // Native Far Sight has the same family and an empty mask; isolate that selector input.
    let empty = data
        .store
        .get(6196)
        .unwrap()
        .class_options
        .as_ref()
        .unwrap();
    assert_eq!(
        spell.class_options.as_ref().unwrap().spell_class_set,
        empty.spell_class_set
    );
    spell.class_options.as_mut().unwrap().spell_class_mask = empty.spell_class_mask;
    let mut h = harness(vec![spell], &[GRACE], 7).unwrap();
    apply_mobility(&mut h, GRACE);
    set_moving(&mut h);
    h.cast_and_deliver(LIGHTNING_BOLT, Some(h.npc));
    assert_eq!(
        h.inbox.failed[0].reason,
        CastFailReason::CantCastWhileMoving
    );
}

#[test]
fn class_completeness_native_grace_on_another_recipient_does_not_allow_the_caster_to_move() {
    let mut h = shaman();
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.npc,
            target: h.npc,
            spell_id: GRACE,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    set_moving(&mut h);
    h.cast_and_deliver(LIGHTNING_BOLT, Some(h.npc));
    assert_eq!(
        h.inbox.failed[0].reason,
        CastFailReason::CantCastWhileMoving
    );
}

#[test]
fn class_completeness_native_grace_expiry_cancels_a_still_moving_cast() {
    let mut h = shaman();
    apply_mobility(&mut h, GRACE);
    let native = world_db().unwrap().store.get(GRACE).unwrap();
    let duration = native.misc.as_ref().unwrap().duration.unwrap().duration_ms;
    h.advance((duration as usize - 1000) / 50);
    set_moving(&mut h);
    h.cast_and_deliver(LIGHTNING_BOLT, Some(h.npc));
    assert!(
        h.app.world().get::<CastState>(h.caster).is_some(),
        "{:?}",
        h.inbox.failed
    );
    h.advance(25);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, GRACE));
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    assert!(
        h.inbox
            .go
            .iter()
            .all(|event| event.spell_id != LIGHTNING_BOLT)
    );
}

#[test]
fn class_completeness_native_hover_keeps_a_real_disintegrate_channel_moving() {
    let mut h = harness(Vec::new(), &[HOVER, DISINTEGRATE, DISINTEGRATE_SLOW], 13).unwrap();
    h.set_power(PowerType::Essence, 5.0);
    apply_mobility(&mut h, HOVER);
    set_moving(&mut h);
    h.cast_and_deliver(DISINTEGRATE, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.app
            .world()
            .get::<CastState>(h.caster)
            .unwrap()
            .cast_while_moving
    );
    h.advance(24);
    assert!(h.app.world().get::<CastState>(h.caster).is_some());
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage
                && event.spell_id == Some(DISINTEGRATE)
                && event.amount > 0)
    );
    remove_mobility(&mut h, HOVER);
    h.advance(1);
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
}

#[test]
fn class_completeness_native_movement_prohibition_flag_overrides_a_matching_aura() {
    let mut spell = world_db()
        .unwrap()
        .store
        .get(LIGHTNING_BOLT)
        .unwrap()
        .clone();
    // Native Attr13 flag isolated on a real cast; empower-only examples are not certified here.
    spell.misc.as_mut().unwrap().attributes[13] |= NO_DISABLE_MOVEMENT_INTERRUPT;
    let mut h = harness(vec![spell], &[GRACE], 7).unwrap();
    apply_mobility(&mut h, GRACE);
    set_moving(&mut h);
    h.cast_and_deliver(LIGHTNING_BOLT, Some(h.npc));
    assert_eq!(
        h.inbox.failed[0].reason,
        CastFailReason::CantCastWhileMoving
    );
}
