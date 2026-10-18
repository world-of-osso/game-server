//! Native Guardian Spirit player casts followed by actual ordinary NPC spells.
//! TC a352b1fa spell_priest.cpp2136–2176; native70448 carries overkill aura316.
use crate::auras::has_aura;
use crate::spell_cast::test_harness::{Harness, harness};
use shared::casting::CastState;
use shared::protocol::CombatLogKind;

pub(super) const GUARDIAN_SPIRIT: u32 = 47_788;
pub(super) const GUARDIAN_HEAL: u32 = 48_153;
/// Native single-effect Fire damage800, no coefficient/variance/power cost.
pub(super) const ANNIHILATION: u32 = 209_140;
/// Native single-effect Shadow damage1000, instant/no coefficient/variance/cost.
pub(super) const DEATH_BY_DARKNESS: u32 = 314_616;

#[test]
fn guardian_spirit_native_shared_ardent_reports_actual_post_rescue_damage() {
    let mut h = harness(Vec::new(), &[31_850, ANNIHILATION, DEATH_BY_DARKNESS], 2).unwrap();
    h.cast_and_deliver(31_850, Some(h.caster));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(has_aura(h.app.world(), h.caster, 31_850));
    // Unlike Annihilation's native attr4 0x100, this spell respects damage-taken modifiers.
    cast_native_attack(&mut h, DEATH_BY_DARKNESS);
    assert_eq!(h.health(h.caster), 300.0);
    cast_native_attack(&mut h, DEATH_BY_DARKNESS);
    assert_eq!(h.health(h.caster), 200.0);
    // Native target floor20%: the second700 hit loses100 health, absorbing600.
    let hit = h
        .inbox
        .log
        .iter()
        .rfind(|event| {
            event.kind == CombatLogKind::Damage && event.spell_id == Some(DEATH_BY_DARKNESS)
        })
        .unwrap();
    assert_eq!((hit.amount, hit.absorbed, hit.overflow), (100, 600, 0));
    assert!(!has_aura(h.app.world(), h.caster, 31_850));
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Death)
    );
}

pub(super) fn fixture() -> Harness {
    harness(
        Vec::new(),
        &[
            GUARDIAN_SPIRIT,
            GUARDIAN_HEAL,
            ANNIHILATION,
            DEATH_BY_DARKNESS,
        ],
        5,
    )
    .unwrap()
}

pub(super) fn cast_native_attack(h: &mut Harness, spell: u32) {
    crate::spell_cast::creature_cast(h.app.world_mut(), h.npc, spell, Some(h.caster), false)
        .unwrap();
    let frame_seconds = h.app.world().resource::<bevy::prelude::Time>().delta_secs();
    let frames = h.app.world().get::<CastState>(h.npc).map_or(0, |cast| {
        ((cast.duration - cast.elapsed).max(0.0) / frame_seconds).ceil() as usize
    });
    h.advance(frames + 3);
}

pub(super) fn cast_guardian(h: &mut Harness) {
    h.cast_and_deliver(GUARDIAN_SPIRIT, Some(h.caster));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(has_aura(h.app.world(), h.caster, GUARDIAN_SPIRIT));
}

#[test]
fn guardian_spirit_native_unprotected_control_dies_to_the_same_attacks() {
    let mut h = fixture();
    cast_native_attack(&mut h, ANNIHILATION);
    assert_eq!(h.health(h.caster), 200.0);
    cast_native_attack(&mut h, DEATH_BY_DARKNESS);
    assert_eq!(h.health(h.caster), 0.0);
}

#[test]
fn guardian_spirit_native_player_cast_consumes_before_real_rescue_heal() {
    let mut h = fixture();
    cast_native_attack(&mut h, ANNIHILATION);
    assert_eq!(h.health(h.caster), 200.0);
    cast_guardian(&mut h);
    cast_native_attack(&mut h, DEATH_BY_DARKNESS);
    // Native effect1=40% of1000, added to surviving200. Removing the parent
    // first prevents its separate +60% healing-taken effect amplifying48153.
    assert_eq!(h.health(h.caster), 600.0);
    assert!(!has_aura(h.app.world(), h.caster, GUARDIAN_SPIRIT));
    assert!(h.inbox.log.iter().any(|event| {
        event.kind == CombatLogKind::Damage
            && event.spell_id == Some(DEATH_BY_DARKNESS)
            && event.amount == 0
            && event.absorbed == 1000
            && event.overflow == 0
    }));
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Death)
    );
    let removed = h
        .inbox
        .log
        .iter()
        .position(|event| {
            event.kind == CombatLogKind::AuraRemoved && event.spell_id == Some(GUARDIAN_SPIRIT)
        })
        .unwrap();
    let healed = h
        .inbox
        .log
        .iter()
        .position(|event| {
            event.kind == CombatLogKind::Heal && event.spell_id == Some(GUARDIAN_HEAL)
        })
        .unwrap();
    assert!(removed < healed);
    assert!(h.inbox.log.iter().any(|event| {
        event.kind == CombatLogKind::Heal
            && event.spell_id == Some(GUARDIAN_HEAL)
            && event.source == Some(h.caster.to_bits())
            && event.target == Some(h.caster.to_bits())
            && event.amount == 400
    }));
}
