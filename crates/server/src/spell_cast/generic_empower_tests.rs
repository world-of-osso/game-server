//! Native70448 empower stages do not require a registered completion callback.
//! TC a352b1fa Spell.cpp4333–4346/9057–9067: empty hook list still completes.
use super::test_harness::{Harness, harness, world_db};
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::casting::{CastState, CastType};
use shared::protocol::{CombatChannel, CombatLogKind, ReleaseEmpowerSpell, StopSpellCast};

const CONSUMPTION: u32 = 1_263_824;
const SLICING_WINDS: u32 = 1_217_413;
const UPHEAVAL: u32 = 396_286;

fn fixture(spell_id: u32, class: u8) -> Harness {
    let mut h = harness(Vec::new(), &[spell_id], class).unwrap();
    h.app.insert_resource(world_db().unwrap().store.clone());
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<ReleaseEmpowerSpell>::default(),
        MessageReceiver::<ReleaseEmpowerSpell>::default(),
    ));
    let target = if spell_id == UPHEAVAL {
        h.npc
    } else {
        h.caster
    };
    h.cast_and_deliver(spell_id, Some(target));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.app.world().get::<CastState>(h.caster).is_some(),
        "native empower{spell_id} must admit rather than reject an absent callback"
    );
    h
}

fn release(h: &mut Harness, spell_id: u32) {
    h.app
        .world_mut()
        .get_mut::<MessageSender<ReleaseEmpowerSpell>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(ReleaseEmpowerSpell { spell_id });
}

fn assert_channel(h: &Harness, spell_id: u32, duration: f32) {
    let cast = h.app.world().get::<CastState>(h.caster).unwrap();
    assert_eq!(cast.spell_id, spell_id);
    assert_eq!(cast.cast_type, CastType::Channel);
    assert!((cast.duration - duration).abs() < 0.001, "{cast:?}");
}

fn advance_until_finished(h: &mut Harness, frames: usize) {
    for _ in 0..frames {
        h.advance(1);
        if h.app.world().get::<CastState>(h.caster).is_none() {
            assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
            return;
        }
    }
    panic!("native empower did not finish by its authored stage/hold limit");
}

fn early_release(spell_id: u32, class: u8, stage_one_frames: usize, total_duration: f32) {
    let mut h = fixture(spell_id, class);
    assert_channel(&h, spell_id, total_duration);
    release(&mut h, spell_id);
    // Delivery is50ms; remain well before every native first-stage minimum.
    h.advance(1);
    assert_channel(&h, spell_id, total_duration);
    advance_until_finished(&mut h, stage_one_frames + 3);
    assert!(h.inbox.log.iter().any(|event| {
        event.spell_id == Some(spell_id) && event.kind == CombatLogKind::CastSuccess
    }));
    assert!(
        !crate::auras::has_aura(h.app.world(), h.caster, spell_id),
        "released parent channel auras must be removed"
    );
}

#[test]
fn census_empower_native_consumption_three_stages_and_early_release() {
    // Duration39 is2000ms: native last stage receives the remaining500ms.
    early_release(CONSUMPTION, 6, 15, 3.0);
}

#[test]
fn census_empower_native_slicing_winds_four_stages_and_early_release() {
    early_release(SLICING_WINDS, 10, 7, 2.4);
}

#[test]
fn census_empower_native_upheaval_three_stages_and_early_release() {
    early_release(UPHEAVAL, 13, 20, 3.5);
}

#[test]
fn census_empower_native_unhandled_release_waits_all_stages_and_maximum_hold() {
    let mut h = fixture(SLICING_WINDS, 10);
    // Four350ms native stages plus TC1000ms hold; no automatic stage-one shortcut.
    h.advance(30);
    assert_channel(&h, SLICING_WINDS, 2.4);
    advance_until_finished(&mut h, 20);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.caster,
        SLICING_WINDS
    ));
}

#[test]
fn census_empower_native_cancellation_is_not_an_early_release() {
    let mut h = fixture(CONSUMPTION, 6);
    h.app
        .world_mut()
        .get_mut::<MessageSender<StopSpellCast>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(StopSpellCast);
    h.advance(3);
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.caster,
        CONSUMPTION
    ));
    let completed = h.inbox.log.len();
    release(&mut h, CONSUMPTION);
    h.advance(70);
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    assert!(
        !h.inbox.log[completed..].iter().any(|event| {
            event.source == Some(h.caster.to_bits()) && event.kind == CombatLogKind::Damage
        }),
        "late release cannot revive a cancelled empower"
    );
}
