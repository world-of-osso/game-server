//! One owned departure followed by native MoveFall, never a ground-snapped chord.
//! FOLLOWER_NPC_FALL_DAMAGE_UNMODELED: native NPC MoveFall applies no landing damage.
use super::{Navigation, point, position};
use crate::maps::InMap;
use crate::scripted_ai::{fall_movement, movement};
use bevy::prelude::*;
use shared::components::{Health, Position};

// No authored AI callback is associated with the navigation-owned movement.
const MOVEMENT_ID: u32 = 0;

#[derive(Clone, Copy)]
enum Phase {
    Departure,
    Falling,
    Blocked,
}

#[derive(Component)]
pub struct CollisionDropMovement {
    map: InMap,
    landing: Vec3,
    phase: Phase,
}

pub(super) fn advance_collision_drops(world: &mut World) {
    let requests = std::mem::take(&mut world.resource_mut::<Navigation>().drop_requests);
    for (unit, (map_id, drop)) in requests {
        start_departure(world, unit, map_id, drop);
    }
    let units: Vec<_> = world
        .query_filtered::<Entity, With<CollisionDropMovement>>()
        .iter(world)
        .collect();
    for unit in units {
        advance_owned_drop(world, unit);
    }
}

fn start_departure(
    world: &mut World,
    unit: Entity,
    map_id: u32,
    drop: ::navigation::CollisionDrop,
) {
    world.resource_mut::<Navigation>().cancel_route(unit);
    let Some(map) = world
        .get::<InMap>(unit)
        .copied()
        .filter(|map| map.map_id == map_id)
    else {
        error!(
            ?unit,
            map_id, "NAV_ONE_WAY_DROP_FROM_COLLISION lost its source map"
        );
        return;
    };
    world.entity_mut(unit).insert(CollisionDropMovement {
        map,
        landing: drop.landing,
        phase: Phase::Departure,
    });
    movement::move_point(world, unit, MOVEMENT_ID, position(drop.departure));
}

fn advance_owned_drop(world: &mut World, unit: Entity) {
    let movement = world
        .get::<CollisionDropMovement>(unit)
        .expect("owned drop");
    let map_changed = world.get::<InMap>(unit) != Some(&movement.map);
    let dead = world
        .get::<Health>(unit)
        .is_some_and(|health| health.current <= 0.0);
    if map_changed || dead {
        world.entity_mut(unit).remove::<(
            CollisionDropMovement,
            movement::PointMovement,
            fall_movement::FallMovement,
        )>();
        return;
    }
    match movement.phase {
        Phase::Departure if world.get::<movement::PointMovement>(unit).is_none() => {
            start_fall(world, unit)
        }
        Phase::Falling if world.get::<fall_movement::FallMovement>(unit).is_none() => {
            finish_landing(world, unit)
        }
        _ => {}
    }
}

fn start_fall(world: &mut World, unit: Entity) {
    fall_movement::move_fall(world, unit, MOVEMENT_ID);
    let phase = if world.get::<fall_movement::FallMovement>(unit).is_some() {
        Phase::Falling
    } else {
        error!(
            ?unit,
            "NAV_ONE_WAY_DROP_FROM_COLLISION could not start native MoveFall"
        );
        Phase::Blocked
    };
    world
        .get_mut::<CollisionDropMovement>(unit)
        .expect("owned drop")
        .phase = phase;
}

fn finish_landing(world: &mut World, unit: Entity) {
    let landing = world
        .get::<CollisionDropMovement>(unit)
        .expect("owned drop")
        .landing;
    let arrived = world
        .get::<Position>(unit)
        .is_some_and(|feet| point(*feet).distance(landing) <= ::navigation::CELL_HEIGHT);
    if arrived {
        world.entity_mut(unit).remove::<CollisionDropMovement>();
    } else {
        error!(
            ?unit,
            ?landing,
            "NAV_ONE_WAY_DROP_FROM_COLLISION landed outside qualified target support"
        );
        world
            .get_mut::<CollisionDropMovement>(unit)
            .expect("owned drop")
            .phase = Phase::Blocked;
    }
}
