//! Resolve client ship-local movement against the authoritative current ship pose.
use crate::maps::InMap;
use shared::{
    components::TransportPassenger, protocol::PlayerInput, transport_route::TransportPose,
};

pub(crate) fn resolve_input(
    input: &PlayerInput,
    passenger: Option<TransportPassenger>,
    ship: Option<(u64, InMap, TransportPose)>,
    map: InMap,
) -> Result<PlayerInput, String> {
    let Some(frame) = input.transport else {
        return Ok(input.clone());
    };
    let current = passenger.ok_or("movement reports a ship without an authoritative attachment")?;
    let (entity, ship_map, pose) = ship.ok_or("attached ship movement frame is unavailable")?;
    if frame.transport != current.transport || entity != current.transport || ship_map != map {
        return Err("movement reports a different ship or match copy".into());
    }
    if !frame
        .position
        .iter()
        .all(|coordinate| coordinate.is_finite())
        || !frame.orientation.is_finite()
    {
        return Err("ship-local movement contains a non-finite coordinate".into());
    }
    let world = pose.local_to_world(TransportPose {
        position: frame.position,
        orientation: frame.orientation,
    });
    Ok(PlayerInput {
        position: [world.position[0], world.position[2], -world.position[1]],
        facing_yaw: world.orientation + std::f32::consts::FRAC_PI_2,
        ..input.clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(frame: Option<TransportPassenger>) -> PlayerInput {
        PlayerInput {
            direction: [1.0, 0.0, 0.0],
            facing_yaw: 0.0,
            jumping: false,
            running: true,
            swimming: false,
            flying: false,
            position: [9000.0, 0.0, 9000.0],
            epoch: 3,
            transport: frame,
        }
    }

    #[test]
    fn attached_report_uses_current_source_ship_pose_not_stale_world_position() {
        let catalog = crate::transport_catalog::fixture_catalog();
        let ship = &catalog.0[&35];
        let pose = ship.route.pose_at(30_000);
        let map = InMap {
            map_id: 1803,
            instance_id: 8,
        };
        let frame = TransportPassenger {
            transport: 200,
            position: [-28.8623, 0.520232, 20.5534],
            orientation: 0.25,
        };
        let report = resolve_input(
            &input(Some(frame)),
            Some(frame),
            Some((200, map, pose)),
            map,
        )
        .unwrap();
        let expected = pose.local_to_world(TransportPose {
            position: frame.position,
            orientation: frame.orientation,
        });
        assert_eq!(
            report.position,
            [
                expected.position[0],
                expected.position[2],
                -expected.position[1]
            ]
        );
        assert!(
            (report.facing_yaw - expected.orientation - std::f32::consts::FRAC_PI_2).abs() < 0.0001
        );
        assert_eq!(report.transport, Some(frame));
        assert_eq!(report.epoch, 3);
    }

    #[test]
    fn a_different_copy_or_ship_cannot_replace_an_existing_frame() {
        let map = InMap {
            map_id: 1803,
            instance_id: 8,
        };
        let other = InMap {
            map_id: 1803,
            instance_id: 9,
        };
        let pose = TransportPose {
            position: [1193.9063, 2677.0313, 297.08664],
            orientation: 0.0,
        };
        let frame = TransportPassenger {
            transport: 200,
            position: [-29.8623, 0.520232, 20.5534],
            orientation: 0.0,
        };
        assert!(
            resolve_input(
                &input(Some(frame)),
                Some(frame),
                Some((200, other, pose)),
                map
            )
            .is_err()
        );
        let wrong = TransportPassenger {
            transport: 201,
            ..frame
        };
        assert!(
            resolve_input(
                &input(Some(wrong)),
                Some(frame),
                Some((200, map, pose)),
                map
            )
            .is_err()
        );
        assert!(resolve_input(&input(Some(frame)), None, Some((200, map, pose)), map).is_err());
    }

    #[test]
    fn disembark_and_ordinary_movement_keep_the_world_report() {
        let map = InMap {
            map_id: 1803,
            instance_id: 8,
        };
        let frame = TransportPassenger {
            transport: 200,
            position: [-29.8623, 0.520232, 20.5534],
            orientation: 0.0,
        };
        for previous in [Some(frame), None] {
            let report = resolve_input(&input(None), previous, None, map).unwrap();
            assert_eq!(report.position, [9000.0, 0.0, 9000.0]);
            assert_eq!(report.transport, None);
        }
    }
}
