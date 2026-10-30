//! DAWNBREAKER_FLIGHT_TRAVERSAL: follower flight follows the player's 3D position.
use bevy::prelude::*;
use shared::components::{Health, Mounted, MovementSpeed, PlayerMotion, Position};

use crate::followers::Follower;
use crate::maps::InMap;
use crate::navigation::{Goal, Navigation};

#[derive(Component)]
pub(super) struct FlightFollow;

pub(super) fn update(world: &mut World) {
    let followers: Vec<_> = world
        .query::<(Entity, &Follower, &Position, &InMap, &Health)>()
        .iter(world)
        .filter(|(_, _, _, _, health)| health.current > 0.0)
        .map(|(unit, follower, position, map, _)| (unit, follower.owner, *position, *map))
        .collect();
    let dt = world.resource::<Time>().delta_secs();
    for (unit, owner, position, map) in followers {
        let Some((leader, leader_map)) = world
            .get::<Position>(owner)
            .copied()
            .zip(world.get::<InMap>(owner).copied())
        else {
            continue;
        };
        if leader_map != map {
            continue;
        }
        let flying = world
            .get::<PlayerMotion>(owner)
            .is_some_and(|motion| motion.contains(PlayerMotion::FLYING));
        let mounted = world.get::<Mounted>(owner).cloned();
        let active = world.get::<FlightFollow>(unit).is_some();
        if flying && !active {
            let feet = Vec3::new(leader.x, leader.y, leader.z);
            let area = world
                .get_resource_mut::<crate::ground::GroundMap>()
                .and_then(|mut ground| ground.area_at(map.map_id, feet));
            let allowed = area.is_some_and(|area| {
                world
                    .get_resource::<crate::area_table::AreaTable>()
                    .is_some_and(|table| table.permits_flying(area, map.map_id))
            });
            let Some(mut mount) = mounted.filter(|_| allowed) else {
                continue;
            };
            mount.vehicle_id = 0;
            mount.seats = [None; 8];
            if let Some(mut nav) = world.get_resource_mut::<Navigation>() {
                nav.cancel_route(unit);
            }
            world.entity_mut(unit).insert((FlightFollow, mount));
        } else if !active {
            continue;
        }
        let speed = world
            .get::<MovementSpeed>(owner)
            .map_or(7.0, |speed| speed.0.max(7.0));
        let here = Vec3::new(position.x, position.y, position.z);
        let target = Vec3::new(leader.x, leader.y, leader.z);
        if flying {
            move_toward(world, unit, here, target, speed, dt);
        } else if here.distance(target) > 1.0 {
            move_toward(world, unit, here, target, speed, dt);
        } else {
            land_on_leader_floor(world, unit, map, here, target, speed, dt);
        }
    }
}

fn move_toward(world: &mut World, unit: Entity, here: Vec3, target: Vec3, speed: f32, dt: f32) {
    let next = here + (target - here).clamp_length_max(speed * dt);
    super::move_flying_follower(
        world,
        unit,
        Position {
            x: next.x,
            y: next.y,
            z: next.z,
        },
        speed,
    );
}

fn land_on_leader_floor(
    world: &mut World,
    unit: Entity,
    map: InMap,
    here: Vec3,
    target: Vec3,
    speed: f32,
    dt: f32,
) {
    use navigation::worker::{Output, Query};
    let Some(result) = world.get_resource_mut::<Navigation>().and_then(|mut nav| {
        nav.query_deferred(
            unit,
            map.map_id,
            Goal::FollowerFlightLanding,
            Query::Collision {
                from: target,
                to: target,
            },
        )
    }) else {
        return;
    };
    let floor = match result {
        Ok(Some(Output::Collision(floor))) => floor,
        other => {
            error!(
                ?unit,
                ?map,
                ?other,
                "follower flight landing could not resolve the leader's navmesh floor"
            );
            return;
        }
    };
    if (floor.y - target.y).abs() > navigation::AGENT_CLIMB {
        error!(
            ?unit,
            ?map,
            ?target,
            ?floor,
            "follower flight landing floor is not at the leader"
        );
        return;
    }
    if here.distance(floor) > 0.05 {
        move_toward(world, unit, here, floor, speed, dt);
        return;
    }
    if let Some(mut nav) = world.get_resource_mut::<Navigation>() {
        nav.cancel_query(unit, Goal::FollowerFlightLanding);
    }
    world.entity_mut(unit).remove::<(FlightFollow, Mounted)>();
}
