//! Auto-attack ticks and owner errors against the real map-0 LOS bake used by
//! spell_cast::line_of_sight_tests (Northshire Abbey).

use super::*;
use crate::line_of_sight::{LineOfSight, sees_in_world};
use crate::spell_cast::test_harness::{Harness, harness};
use shared::components::{ModelDisplay, Rotation};
use shared::spell_data::CastFailReason;

const KOBOLD_DISPLAY: u32 = 10913;

fn wow_position(x: f32, y: f32, z: f32) -> Position {
    let (x, y, z) = crate::terrain_height::wow_to_bevy(x, y, z);
    Position { x, y, z }
}

fn place(h: &mut Harness, from: Position, to: Position) {
    let yaw = (to.x - from.x).atan2(to.z - from.z);
    h.app.world_mut().entity_mut(h.caster).insert((
        from,
        Rotation {
            x: 0.0,
            y: yaw,
            z: 0.0,
        },
    ));
    h.app.world_mut().entity_mut(h.npc).insert((
        to,
        Rotation {
            x: 0.0,
            y: yaw + std::f32::consts::PI,
            z: 0.0,
        },
        ModelDisplay {
            display_id: KOBOLD_DISPLAY,
        },
    ));
}

fn los_harness() -> Harness {
    let dir = crate::ground::ground_data_dir().join("los");
    crate::test_data::require(dir.join(los::eye::EYE_TABLE_FILE));
    crate::test_data::require(dir.join("0/32_48.los"));
    let mut h = harness(Vec::new(), &[], 1).unwrap();
    // No hit-table randomness: the unarmed player always deals 10 damage.
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .remove::<CharacterStats>();
    h.app.insert_resource(LineOfSight::load(&dir).unwrap());
    h
}

fn place_across_wall(h: &mut Harness) {
    // First 4 yards of the spell fixture's ray beside the Abbey doorway.
    place(
        h,
        wow_position(-8905.575, -144.91, 81.96),
        wow_position(-8903.883, -148.53409, 81.90937),
    );
    let world = h.app.world();
    assert!(in_melee_range(
        world.get::<Position>(h.caster).unwrap(),
        world.get::<Position>(h.npc).unwrap(),
        0.0
    ));
    assert!(!sees_in_world(world, h.caster, h.npc));
    assert!(!sees_in_world(world, h.npc, h.caster));
}

fn place_in_open_ground(h: &mut Harness) {
    place(
        h,
        wow_position(-8909.2, -146.6, 81.95),
        wow_position(-8907.9, -149.3, 81.95),
    );
    assert!(sees_in_world(h.app.world(), h.caster, h.npc));
    assert!(sees_in_world(h.app.world(), h.npc, h.caster));
}

#[test]
#[ignore = "requires the real map-0 LOS bake and world.db"]
fn player_swing_across_wall_reports_los_holds_both_timers_and_resumes() {
    let mut h = los_harness();
    place_across_wall(&mut h);
    // Both hands ready: neither may reset or postpone the other across a wall.
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CharacterStats {
            off_hand: Some(WeaponDamage::UNARMED),
            ..Default::default()
        });
    let npc = h.npc;
    h.attack(npc);
    h.advance(10);
    assert_eq!(h.health(npc), 5000.0, "wall blocks swing damage");
    assert!(h.inbox.log.is_empty(), "no blocked swing events");
    assert!(!h.inbox.failed.is_empty(), "owner gets LOS error");
    assert!(h.inbox.failed.iter().all(|error| {
        error.reason == CastFailReason::LineOfSight
            && error.spell_id == 6603
            && error.detail.is_none()
    }));
    let attack = h.app.world().get::<AutoAttack>(h.caster).unwrap();
    assert!(attack.timer.is_zero() && attack.off_hand_timer.is_zero());
    assert_eq!(attack.victim, npc);
    place_in_open_ground(&mut h);
    h.inbox.failed.clear();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .remove::<CharacterStats>();
    h.advance(1);
    assert_eq!(h.health(npc), 4990.0, "ready swing lands immediately");
    assert!(h.inbox.failed.is_empty());
    assert!(
        !h.app
            .world()
            .get::<AutoAttack>(h.caster)
            .unwrap()
            .timer
            .is_zero()
    );
}

#[test]
#[ignore = "requires the real map-0 LOS bake and world.db"]
fn creature_swing_across_wall_holds_timer_and_resumes_without_player_error() {
    let mut h = los_harness();
    place_across_wall(&mut h);
    h.app.world_mut().entity_mut(h.npc).insert((
        AutoAttack::new(h.caster),
        AuraStatMods {
            melee_hit_chance_pct: 100.0,
            ..Default::default()
        },
    ));
    h.advance(10);
    assert_eq!(h.health(h.caster), 1000.0);
    assert!(h.inbox.log.is_empty());
    assert!(
        h.inbox.failed.is_empty(),
        "creature error is not sent to victim"
    );
    assert!(
        h.app
            .world()
            .get::<AutoAttack>(h.npc)
            .unwrap()
            .timer
            .is_zero()
    );
    place_in_open_ground(&mut h);
    h.advance(1);
    assert_eq!(
        h.health(h.caster),
        990.0,
        "ready creature swing lands immediately"
    );
    assert!(
        !h.app
            .world()
            .get::<AutoAttack>(h.npc)
            .unwrap()
            .timer
            .is_zero()
    );
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.source == Some(h.npc.to_bits()))
    );
    assert!(h.inbox.failed.is_empty());
}

#[test]
#[ignore = "requires the real map-0 LOS bake and world.db"]
fn player_swing_in_open_ground_lands() {
    let mut h = los_harness();
    place_in_open_ground(&mut h);
    let npc = h.npc;
    h.attack(npc);
    h.advance(3);
    assert_eq!(h.health(npc), 4990.0);
    assert!(h.inbox.failed.is_empty());
}
