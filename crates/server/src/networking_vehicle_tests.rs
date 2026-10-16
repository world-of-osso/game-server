use super::*;

fn source_demolisher() -> NpcMover {
    NpcMover {
        entity: World::new().spawn_empty().id(),
        map: InMap::continent(0),
        movement: VehicleMovement {
            flags: 1_594_726_007,
            turn_speed: 1.04719758,
        },
        // Source34775 template walk1.2/run0.98571; Vehicle509 DB2 metadata.
        walk: DEFAULT_WALK_SPEED * 1.2,
        run: DEFAULT_RUN_SPEED * 0.98571,
    }
}

fn report(position: [f32; 3], yaw: f32) -> PlayerInput {
    PlayerInput {
        transport: None,
        direction: [yaw.sin(), 0.0, yaw.cos()],
        facing_yaw: yaw,
        jumping: false,
        running: true,
        swimming: false,
        flying: false,
        position,
        epoch: 1,
    }
}

#[test]
fn vehicle_driver_burst_uses_source_speed_and_one_shared_time_bank() {
    let mover = source_demolisher();
    let mut clocks = MovementClocks::default();
    let mut position = Position {
        x: 0.0,
        y: 5.0,
        z: 0.0,
    };
    for _ in 0..20 {
        position = move_vehicle(
            &report([0.0, 5.0, 100.0], 0.0),
            &mover,
            mover.entity,
            &mut clocks,
            None,
            position,
            0.0,
            None,
        )
        .position;
    }
    assert!((position.z - mover.run * MOVEMENT_BANK_SECS).abs() < 0.001);
    assert_eq!(position.y, 5.0);
    clocks.advance(0.1);
    let next = move_vehicle(
        &report([0.0, 5.0, 100.0], 0.0),
        &mover,
        mover.entity,
        &mut clocks,
        None,
        position,
        0.0,
        None,
    );
    assert!((next.position.z - position.z - mover.run * 0.1).abs() < 0.001);
}

#[test]
fn vehicle_driver_stationary_turns_are_bounded_by_authored_turn_speed() {
    let mover = source_demolisher();
    let mut clocks = MovementClocks::default();
    let position = Position {
        x: 0.0,
        y: 5.0,
        z: 0.0,
    };
    let mut yaw = 0.0;
    for _ in 0..20 {
        let step = move_vehicle(
            &report([0.0, 5.0, 0.0], 1.5),
            &mover,
            mover.entity,
            &mut clocks,
            None,
            position,
            yaw,
            None,
        );
        yaw = step.yaw;
        assert_eq!(step.position, position);
    }
    assert!((yaw - mover.movement.turn_speed * MOVEMENT_BANK_SECS).abs() < 0.001);
}

#[test]
fn vehicle_driver_source_flags_refuse_strafe_and_reported_flight_height() {
    let mover = source_demolisher();
    let mut clocks = MovementClocks::default();
    let input = PlayerInput {
        flying: true,
        jumping: true,
        swimming: true,
        ..report([10.0, 80.0, 10.0], 0.0)
    };
    let position = Position {
        x: 0.0,
        y: 5.0,
        z: 0.0,
    };
    let step = move_vehicle(
        &input,
        &mover,
        mover.entity,
        &mut clocks,
        None,
        position,
        0.0,
        None,
    );
    assert_eq!(step.position.x, 0.0);
    assert_eq!(step.position.y, 5.0);
    assert!(step.position.z > 0.0 && step.position.z <= mover.run * MAX_INPUT_STEP_SECS);
}

#[test]
fn vehicle_driver_no_jump_vehicle_cannot_hover_above_lower_supported_ground() {
    let mover = source_demolisher();
    let mut clocks = MovementClocks::default();
    let mut ground = crate::terrain_height::test_helpers::flat_ground(100.0, 100.0, 48.0);
    let position = Position {
        x: 90.0,
        y: 50.0,
        z: -90.0,
    };
    let input = PlayerInput {
        flying: true,
        jumping: true,
        ..report([90.0, 80.0, -89.9], 0.0)
    };
    let step = move_vehicle(
        &input,
        &mover,
        mover.entity,
        &mut clocks,
        Some((&mut ground, mover.map)),
        position,
        0.0,
        None,
    );
    assert_eq!(
        step.position.y, 48.0,
        "the grounded source vehicle follows the supported NPC floor, not a flight/hover report"
    );
}
