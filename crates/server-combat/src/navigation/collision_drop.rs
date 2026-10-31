//! Owned movement to a supported edge followed by ballistic MoveFall.
//! FOLLOWER_NPC_FALL_DAMAGE_UNMODELED: native NPC MoveFall applies no landing damage.
use super::{Goal, MoveRequest, Navigation, Step, point, position};
use crate::maps::InMap;
use crate::scripted_ai::{fall_movement, movement};
use bevy::prelude::*;
use shared::components::{Health, MovementControl, Position};

const MOVEMENT_ID: u32 = 0;

#[derive(Clone, Copy)]
enum Phase {
    Approach(Vec3),
    Falling,
    Blocked,
}

#[derive(Component)]
pub struct CollisionDropMovement {
    map: InMap,
    landing: Vec3,
    phase: Phase,
    control_epoch: Option<u32>,
}

pub(crate) fn advance_collision_drops(world: &mut World) {
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

/// A route-owned drop is already at its supported edge when navigation requests it.
pub(crate) fn start_departure(
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
        error!(?unit, map_id, "collision drop lost its source map");
        return;
    };
    world.entity_mut(unit).insert(CollisionDropMovement {
        map,
        landing: drop.landing,
        phase: Phase::Blocked,
        control_epoch: None,
    });
    start_fall(world, unit);
}

/// Own a passage participant until it walks to the edge and lands.
pub(crate) fn start_passage_drop(
    world: &mut World,
    unit: Entity,
    map: InMap,
    edge: Vec3,
    landing: Vec3,
) {
    world.resource_mut::<Navigation>().cancel_route(unit);
    let control_epoch = world.get_mut::<MovementControl>(unit).map(|mut control| {
        control.epoch = control.epoch.wrapping_add(1);
        control.controlled = true;
        control.epoch
    });
    world.entity_mut(unit).insert(CollisionDropMovement {
        map,
        landing,
        phase: Phase::Approach(edge),
        control_epoch,
    });
    crate::spell_cast::mark_dirty(world, unit);
}

fn advance_owned_drop(world: &mut World, unit: Entity) {
    let drop = world
        .get::<CollisionDropMovement>(unit)
        .expect("owned drop");
    let map_changed = world.get::<InMap>(unit) != Some(&drop.map);
    let dead = world
        .get::<Health>(unit)
        .is_some_and(|health| health.current <= 0.0);
    let phase = drop.phase;
    if map_changed || dead {
        finish_control(world, unit);
        world.entity_mut(unit).remove::<(
            CollisionDropMovement,
            movement::PointMovement,
            fall_movement::FallMovement,
        )>();
        return;
    }
    match phase {
        Phase::Approach(edge) => advance_approach(world, unit, edge),
        Phase::Falling if world.get::<fall_movement::FallMovement>(unit).is_none() => {
            finish_landing(world, unit)
        }
        _ => {}
    }
}

fn advance_approach(world: &mut World, unit: Entity, edge: Vec3) {
    let Some(from) = world.get::<Position>(unit).copied() else {
        block_drop(world, unit, "position missing");
        return;
    };
    let map_id = world
        .get::<CollisionDropMovement>(unit)
        .expect("owned drop")
        .map
        .map_id;
    let distance = crate::creature_ai::BASE_RUN_SPEED * world.resource::<Time>().delta_secs();
    let request = MoveRequest {
        map_id,
        goal: Goal::Point,
        from,
        to: position(edge),
        distance,
        reach: 0.0,
    };
    match world.resource_mut::<Navigation>().advance(unit, request) {
        Step::Pending => {}
        Step::Blocked | Step::Unbaked => block_drop(world, unit, "edge is unreachable"),
        Step::Moved {
            position: next,
            reached,
        } => {
            *world.get_mut::<Position>(unit).expect("position checked") = next;
            crate::interest::mark_interest_position_changed(
                world.get_resource::<crate::interest::InterestDirtyQueue>(),
                unit,
            );
            crate::spell_cast::mark_dirty(world, unit);
            if reached {
                world.resource_mut::<Navigation>().cancel_route(unit);
                start_fall(world, unit);
            }
        }
    }
}

fn start_fall(world: &mut World, unit: Entity) {
    let landing = world
        .get::<CollisionDropMovement>(unit)
        .expect("owned drop")
        .landing;
    fall_movement::move_fall_towards(world, unit, MOVEMENT_ID, landing);
    if world.get::<fall_movement::FallMovement>(unit).is_some() {
        world
            .get_mut::<CollisionDropMovement>(unit)
            .expect("owned drop")
            .phase = Phase::Falling;
    } else {
        block_drop(world, unit, "native MoveFall could not start");
    }
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
        finish_control(world, unit);
        world.entity_mut(unit).remove::<CollisionDropMovement>();
    } else {
        block_drop(world, unit, "fall ended outside qualified landing");
    }
}

fn block_drop(world: &mut World, unit: Entity, reason: &str) {
    error!(?unit, reason, "collision drop blocked");
    finish_control(world, unit);
    world
        .get_mut::<CollisionDropMovement>(unit)
        .expect("owned drop")
        .phase = Phase::Blocked;
}

fn finish_control(world: &mut World, unit: Entity) {
    let epoch = world
        .get::<CollisionDropMovement>(unit)
        .and_then(|drop| drop.control_epoch);
    if let Some(mut control) = world.get_mut::<MovementControl>(unit)
        && Some(control.epoch) == epoch
    {
        control.controlled = false;
    }
    crate::spell_cast::mark_dirty(world, unit);
}
