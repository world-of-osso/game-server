//! Native Prayer of Mending admission; TC a352b1fa spell_priest.cpp3636–3806.
use super::test_harness::{Harness, harness};
use crate::auras::has_aura;
use shared::casting::CastState;
use shared::protocol::CombatLogKind;

const PRAYER_OF_MENDING: u32 = 33_076;
const MENDING_WARD: u32 = 41_635;
const MENDING_HEAL: u32 = 33_110;
const MENDING_JUMP: u32 = 155_793;
/// Native800 Fire, no coefficient or variance; leaves the existing1000HP player at200.
const ANNIHILATION: u32 = 209_140;

fn fixture() -> Harness {
    harness(
        Vec::new(),
        &[
            PRAYER_OF_MENDING,
            MENDING_WARD,
            MENDING_HEAL,
            MENDING_JUMP,
            ANNIHILATION,
        ],
        5,
    )
    .unwrap()
}

fn ordinary_damage(h: &mut Harness) {
    crate::spell_cast::creature_cast(
        h.app.world_mut(),
        h.npc,
        ANNIHILATION,
        Some(h.caster),
        false,
    )
    .unwrap();
    let dt = h.app.world().resource::<bevy::prelude::Time>().delta_secs();
    let frames = h.app.world().get::<CastState>(h.npc).map_or(0, |cast| {
        ((cast.duration - cast.elapsed).max(0.0) / dt).ceil() as usize
    });
    h.advance(frames + 3);
}

#[test]
fn prayer_of_mending_native_unprotected_damage_does_not_heal_or_jump() {
    let mut h = fixture();
    ordinary_damage(&mut h);
    assert_eq!(h.health(h.caster), 200.0);
    assert!(!has_aura(h.app.world(), h.caster, MENDING_WARD));
    assert!(!h.inbox.log.iter().any(|event| {
        event.spell_id == Some(MENDING_HEAL) && event.kind == CombatLogKind::Heal
    }));
}

#[test]
#[ignore = "Missing native33076->41635 ward; run explicitly for behavioral RED before implementation"]
fn prayer_of_mending_native_normal_player_cast_applies_real_ward() {
    let mut h = fixture();
    h.cast_and_deliver(PRAYER_OF_MENDING, Some(h.caster));
    h.advance(3);
    assert!(
        has_aura(h.app.world(), h.caster, MENDING_WARD),
        "ordinary33076 must create real41635; cast failures: {:?}",
        h.inbox.failed
    );
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let ward = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == MENDING_WARD)
        .unwrap();
    // Source initial native jump count4 plus the initial recipient.
    assert_eq!(ward.stacks, 5);
    assert_eq!(ward.caster, h.caster);
    assert!(h.inbox.log.iter().any(|event| {
        event.kind == CombatLogKind::AuraApplied
            && event.spell_id == Some(MENDING_WARD)
            && event.source == Some(h.caster.to_bits())
            && event.target == Some(h.caster.to_bits())
    }));
}
