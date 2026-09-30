//! Spell travel time on real world.db spells (skipped when data/world.db is absent).

use shared::components::{Health, Position};
use shared::protocol::CombatLogKind;

use crate::spell_cast::test_harness::{Harness, harness};

/// Frostbolt: 1.75 s cast, missile `Speed` 35 yd/s; its damage is the triggered 228597.
const FROSTBOLT: u32 = 116;
const FROSTBOLT_DAMAGE: u32 = 228_597;
/// Mortal Strike: instant, `Speed` 0.
const MORTAL_STRIKE: u32 = 12_294;
const FRAME_SECS: f64 = 0.05;

fn place_npc(h: &mut Harness, x: f32) {
    *h.app.world_mut().get_mut::<Position>(h.npc).unwrap() = Position { x, y: 0.0, z: 0.0 };
}

/// Frames from the spell's CastSuccess line to its damage line, both logged to the
/// caster's connection (the same delivery delay), or `None` without damage in `limit`.
fn frames_from_launch_to_damage(
    h: &mut Harness,
    spell_id: u32,
    damage_spell: u32,
    limit: usize,
    mut in_flight: impl FnMut(&mut Harness),
) -> Option<usize> {
    let mut launched = None;
    for frame in 0..limit {
        h.advance(1);
        let log = std::mem::take(&mut h.inbox.log);
        if launched.is_none()
            && log
                .iter()
                .any(|e| e.kind == CombatLogKind::CastSuccess && e.spell_id == Some(spell_id))
        {
            launched = Some(frame);
            in_flight(h);
        }
        let damaged = log
            .iter()
            .any(|e| e.kind == CombatLogKind::Damage && e.spell_id == Some(damage_spell));
        if damaged {
            return Some(frame - launched.expect("damage after the launch"));
        }
    }
    assert!(launched.is_some(), "{:?}", h.inbox.failed);
    None
}

fn mage() -> Option<Harness> {
    harness(Vec::new(), &[FROSTBOLT, FROSTBOLT_DAMAGE], 8)
}

#[test]
fn frostbolt_at_20_yards_lands_its_flight_time_after_the_launch() {
    let Some(mut h) = mage() else {
        return;
    };
    place_npc(&mut h, 20.0);
    h.cast(FROSTBOLT, Some(h.npc));
    let frames = frames_from_launch_to_damage(&mut h, FROSTBOLT, FROSTBOLT_DAMAGE, 80, |_| {})
        .expect("the Frostbolt lands");
    // 20 yd / 35 yd/s = 0.571 s.
    let secs = frames as f64 * FRAME_SECS;
    assert!((0.55..=0.65).contains(&secs), "{secs} s");
}

#[test]
fn a_target_that_moves_in_flight_is_hit_at_the_launch_time_delay() {
    let Some(mut h) = mage() else {
        return;
    };
    place_npc(&mut h, 20.0);
    h.cast(FROSTBOLT, Some(h.npc));
    let frames = frames_from_launch_to_damage(&mut h, FROSTBOLT, FROSTBOLT_DAMAGE, 80, |h| {
        place_npc(h, 29.0)
    })
    .expect("the Frostbolt still lands");
    let secs = frames as f64 * FRAME_SECS;
    assert!((0.55..=0.65).contains(&secs), "{secs} s");
}

#[test]
fn a_target_that_dies_or_leaves_in_flight_is_not_hit() {
    let Some(mut h) = mage() else {
        return;
    };
    place_npc(&mut h, 20.0);
    let npc = h.npc;
    h.cast(FROSTBOLT, Some(npc));
    let frames = frames_from_launch_to_damage(&mut h, FROSTBOLT, FROSTBOLT_DAMAGE, 80, |h| {
        h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = 0.0;
    });
    assert_eq!(frames, None, "no damage on a unit that died in flight");

    let Some(mut h) = mage() else {
        return;
    };
    place_npc(&mut h, 20.0);
    h.cast(FROSTBOLT, Some(h.npc));
    let frames = frames_from_launch_to_damage(&mut h, FROSTBOLT, FROSTBOLT_DAMAGE, 80, |h| {
        h.app.world_mut().despawn(h.npc);
    });
    assert_eq!(frames, None, "no damage on a unit that left the world");
}

#[test]
fn a_spell_without_missile_speed_lands_in_its_launch_tick() {
    let Some(mut h) = harness(Vec::new(), &[MORTAL_STRIKE], 1) else {
        return;
    };
    h.set_power(shared::components::PowerType::Rage, 1000.0);
    h.cast(MORTAL_STRIKE, Some(h.npc));
    let frames = frames_from_launch_to_damage(&mut h, MORTAL_STRIKE, MORTAL_STRIKE, 20, |_| {});
    assert_eq!(frames, Some(0));
}

fn in_combat(h: &Harness, unit: bevy::prelude::Entity) -> bool {
    h.app
        .world()
        .get::<shared::components::CombatStatus>(unit)
        .is_some_and(|status| status.0)
}

/// `PreprocessSpellLaunch` (Spell.cpp:8538-8541) puts the caster in combat and holds
/// the target's side suppressed; `PreprocessTarget` (Spell.cpp:2763-2764) engages the
/// target when the Frostbolt lands.
#[test]
fn a_missile_target_enters_combat_when_the_missile_lands_not_at_launch() {
    let Some(mut h) = mage() else {
        return;
    };
    for unit in [h.caster, h.npc] {
        h.app
            .world_mut()
            .entity_mut(unit)
            .insert(shared::components::CombatStatus(false));
    }
    place_npc(&mut h, 20.0);
    h.cast(FROSTBOLT, Some(h.npc));
    let mut in_flight = None;
    let frames = frames_from_launch_to_damage(&mut h, FROSTBOLT, FROSTBOLT_DAMAGE, 80, |h| {
        in_flight = Some((in_combat(h, h.caster), in_combat(h, h.npc)));
    })
    .expect("the Frostbolt lands");
    assert!(frames > 0);
    assert_eq!(in_flight, Some((true, false)), "(caster, target) in flight");
    assert!(in_combat(&h, h.caster) && in_combat(&h, h.npc));
}
