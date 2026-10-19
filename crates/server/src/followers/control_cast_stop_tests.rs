//! TC a352b1fa Unit.cpp11609–11620/11706–11720: the first stun stops the current cast.
//! A player caster has no baseline physical miss, so the native stun always lands.
use super::kit_tests::level32;
use super::utility_tests;
use crate::spell_cast::test_harness::Harness;
use bevy::time::TimeUpdateStrategy;
use shared::casting::CastState;
use shared::components::Position;
use shared::protocol::CombatLogKind;

pub(super) const FIREBALL: u32 = 133;
const PARALYSIS: u32 = 115078;
const LEG_SWEEP: u32 = 119381;

/// Frames until the enemy's current cast would have completed.
pub(super) fn remaining_cast_frames(h: &Harness) -> usize {
    let cast = h.app.world().get::<CastState>(h.npc).unwrap();
    let frame_ms = match h.app.world().resource::<TimeUpdateStrategy>() {
        TimeUpdateStrategy::ManualDuration(step) => step.as_secs_f32() * 1000.0,
        _ => panic!("native harness must use a known manual frame step"),
    };
    ((cast.duration - cast.elapsed) * 1000.0 / frame_ms).ceil() as usize + 2
}

pub(super) fn logged(
    h: &Harness,
    from: usize,
    source: u64,
    spell: u32,
    kind: CombatLogKind,
) -> bool {
    h.inbox.log[from..].iter().any(|event| {
        event.source == Some(source) && event.spell_id == Some(spell) && event.kind == kind
    })
}

fn assert_native_stun_stops_enemy_cast(stun: u32, explicit: bool) {
    let (mut h, unit) = level32(227734);
    utility_tests::start_enemy_fireball(&mut h, unit);
    let enemy = h.npc;
    let at = *h.app.world().get::<Position>(enemy).unwrap();
    h.app.world_mut().entity_mut(h.caster).insert(Position {
        x: at.x - 2.0,
        ..at
    });
    let frames = remaining_cast_frames(&h);
    let from = h.inbox.log.len();
    crate::spell_cast::creature_cast(
        h.app.world_mut(),
        h.caster,
        stun,
        explicit.then_some(enemy),
        true,
    )
    .unwrap();
    h.advance(1);
    let owner = h.caster.to_bits();
    assert!(logged(&h, from, owner, stun, CombatLogKind::AuraApplied));
    assert!(
        h.app.world().get::<CastState>(enemy).is_none(),
        "stunned enemy kept casting Fireball"
    );
    h.advance(frames);
    assert!(!logged(
        &h,
        from,
        enemy.to_bits(),
        FIREBALL,
        CombatLogKind::CastSuccess
    ));
}

#[test]
fn native_paralysis_stops_enemy_cast() {
    assert_native_stun_stops_enemy_cast(PARALYSIS, true);
}

#[test]
fn native_leg_sweep_stops_enemy_cast() {
    assert_native_stun_stops_enemy_cast(LEG_SWEEP, false);
}
