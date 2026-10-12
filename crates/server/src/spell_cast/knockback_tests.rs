//! Real source58873, fixed-tick lifetime, owner input and landing.
use super::test_harness::*;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{MovementControl, MovementSpeed, PlayerMotion, Position, Rotation};
use shared::protocol::{InputChannel, PlayerInput};

#[test]
fn arena_knockback_source_effect_drives_ballistic_player_motion_and_rejects_owner_input() {
    let mut h = harness_with_plugins(Vec::new(), &[58873], 1, false, |app| {
        app.init_resource::<crate::friends::AutoAfkScheduler>();
        app.add_systems(FixedUpdate, crate::networking::process_player_inputs);
    }).unwrap();
    h.app.insert_resource(crate::terrain_height::test_helpers::flat_ground(33.0, 33.0, 0.0));
    let los_dir = crate::ground::ground_data_dir().join("los");
    h.app.insert_resource(crate::line_of_sight::LineOfSight::load(&los_dir).unwrap());
    h.app.world_mut().entity_mut(h.caster).insert((
        Position { x: 5.0, y: 0.0, z: -5.0 },
        Rotation { x: 0.0, y: 0.0, z: 0.0 },
        MovementSpeed(0.0), PlayerMotion::default(),
        crate::networking::FallTracker::grounded(0.0),
    ));
    h.app.world_mut().entity_mut(h.npc).insert(Position { x: 3.0, y: 0.0, z: -5.0 });
    h.app.world_mut().entity_mut(h.connection).insert((MessageSender::<PlayerInput>::default(), MessageReceiver::<PlayerInput>::default()));
    let before = *h.app.world().get::<Position>(h.caster).unwrap();
    super::cast_triggered(h.app.world_mut(), h.npc, 58873, None, 0);
    h.app.world_mut().get_mut::<MessageSender<PlayerInput>>(h.connection).unwrap().send::<InputChannel>(PlayerInput {
        direction: [1.0, 0.0, 0.0], facing_yaw: 0.0, jumping: false, running: true, swimming: false, flying: false,
        position: [1000.0, 1000.0, 1000.0], epoch: 0,
    });
    h.advance(1);
    let control = *h.app.world().get::<MovementControl>(h.caster).unwrap();
    assert!(control.controlled, "source knockback takes movement ownership");
    assert_eq!(control.epoch, 1);
    let airborne = *h.app.world().get::<Position>(h.caster).unwrap();
    assert!((airborne.x - before.x - 1.0).abs() < 0.001, "20yd/s from source MiscValue200");
    let expected_y = 3.5 * 0.05 - 0.5 * shared::skyriding::GRAVITY * 0.05 * 0.05;
    assert!((airborne.y - expected_y).abs() < 0.001, "source BasePoints35 upward3.5yd/s");
    assert_eq!(airborne.z, before.z);
    h.advance(20);
    let landed = *h.app.world().get::<Position>(h.caster).unwrap();
    assert_eq!(landed.y, 0.0);
    assert!(landed.x > 12.0 && landed.x <= 13.1);
    let control = *h.app.world().get::<MovementControl>(h.caster).unwrap();
    assert!(!control.controlled);
    assert_eq!(control.epoch, 2, "landing invalidates pre-launch reports");
    h.app.world_mut().get_mut::<MessageSender<PlayerInput>>(h.connection).unwrap().send::<InputChannel>(PlayerInput {
        direction: [1.0, 0.0, 0.0], facing_yaw: 0.0, jumping: false, running: true, swimming: false, flying: false,
        position: [1000.0, 1000.0, 1000.0], epoch: 1,
    });
    h.advance(2);
    assert_eq!(*h.app.world().get::<Position>(h.caster).unwrap(), landed, "stale in-flight owner reports never undo landing");
}
