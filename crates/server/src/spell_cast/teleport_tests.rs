use super::test_harness::*;
use crate::maps::InMap;
use bevy::prelude::*;
use shared::components::{MovementControl, Position, Rotation};

const BLINK: u32 = 1953;

#[test]
fn spell_teleport_blink_lands_twenty_yards_forward_on_ground() {
    let mut h = harness(Vec::new(), &[BLINK], 8).unwrap();
    h.app
        .insert_resource(crate::terrain_height::test_helpers::flat_ground(
            33.0, 33.0, 2.0,
        ));
    let dir = std::path::Path::new("/home/osso/Projects/world-of-osso/game-engine/data/los");
    h.app
        .insert_resource(crate::line_of_sight::LineOfSight::load(dir).unwrap());
    h.app.world_mut().entity_mut(h.caster).insert((
        Position {
            x: 10.0,
            y: 2.0,
            z: -10.0,
        },
        Rotation {
            x: 0.0,
            y: std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
        MovementControl::default(),
    ));
    h.cast(BLINK, None);
    h.advance(4);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let pos = h.app.world().get::<Position>(h.caster).unwrap();
    assert!((pos.x - 30.0).abs() < 0.01, "{pos:?}");
    assert_eq!(pos.y, 2.0);
    assert_eq!(
        h.app
            .world()
            .get::<MovementControl>(h.caster)
            .unwrap()
            .epoch,
        1
    );
}

#[test]
fn spell_teleport_blink_stops_before_real_map_zero_abbey_wall() {
    let mut h = harness(Vec::new(), &[BLINK], 8).unwrap();
    let dir = std::path::Path::new("/home/osso/Projects/world-of-osso/game-engine/data");
    h.app
        .insert_resource(crate::line_of_sight::LineOfSight::load(&dir.join("los")).unwrap());
    h.app.insert_resource(crate::ground::GroundMap::new(
        dir,
        std::collections::HashMap::from([(0, 775971)]),
    ));
    // Existing real-geometry LOS fixture: the doorway line shifted four yards into its wall.
    let (x, y, z) = crate::terrain_height::wow_to_bevy(-8905.575, -144.91, 81.96);
    let start = Vec3::new(x, y, z);
    let direction = Vec3::new(12.7, 0.0, 27.2).normalize();
    let yaw = direction.x.atan2(direction.z);
    let los = h
        .app
        .world()
        .resource::<crate::line_of_sight::LineOfSight>();
    assert!(!los.clear(
        InMap::continent(0),
        start + Vec3::Y,
        start + direction * 20.0 + Vec3::Y
    ));
    h.app.world_mut().entity_mut(h.caster).insert((
        Position { x, y, z },
        Rotation {
            x: 0.0,
            y: yaw,
            z: 0.0,
        },
        MovementControl::default(),
    ));
    h.cast(BLINK, None);
    h.advance(4);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let p = h.app.world().get::<Position>(h.caster).unwrap();
    let end = Vec3::new(p.x, p.y, p.z);
    let distance = Vec2::new(end.x - start.x, end.z - start.z).length();
    assert!(distance > 0.5 && distance < 19.0, "moved {distance}: {p:?}");
    assert!(
        h.app
            .world()
            .resource::<crate::line_of_sight::LineOfSight>()
            .clear(InMap::continent(0), start + Vec3::Y, end + Vec3::Y)
    );
}
