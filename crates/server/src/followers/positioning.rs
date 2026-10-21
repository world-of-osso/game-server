//! Project positioning policy: non-tanks do not approach through idle native sight circles.
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use shared::components::Health;
use shared::group::GroupRole;

use super::{Follower, pull_route};
use crate::creature_aggro::AggroDistance;
use crate::maps::InMap;
use crate::spell_cast::targets;

pub(super) fn hold_unsafe_approach(world: &mut World, unit: Entity, target: Entity) -> bool {
    let Some(follower) = world.get::<Follower>(unit) else {
        return false;
    };
    if follower.profile.role == GroupRole::Tank {
        return false;
    }
    let unsafe_approach = match approach_enters_unpulled_sight(world, unit, target) {
        Ok(unsafe_approach) => unsafe_approach,
        Err(error) => {
            warn!(?unit, %error, "Follower positioning lacks native sight geometry");
            true
        }
    };
    if unsafe_approach {
        world
            .entity_mut(unit)
            .remove::<crate::creature_ai::FollowOwner>()
            .insert(crate::creature_ai::CreatureState::Idle);
    }
    unsafe_approach
}

fn approach_enters_unpulled_sight(
    world: &mut World,
    unit: Entity,
    target: Entity,
) -> Result<bool, String> {
    let map = *world
        .get::<InMap>(unit)
        .ok_or("Follower positioning lacks map")?;
    let from = targets::position(world, unit).ok_or("Follower positioning lacks origin")?;
    let to =
        targets::position(world, target).ok_or("Follower positioning lacks target position")?;
    let watchers: Vec<_> = world
        .resource::<crate::maps::units::MapUnits>()
        .in_map(map)
        .filter(|watcher| {
            world
                .get::<crate::creature_aggro::AggroRange>(*watcher)
                .is_some()
        })
        .filter(|watcher| {
            world
                .get::<Health>(*watcher)
                .is_some_and(|health| health.current > 0.0)
        })
        .filter(|watcher| {
            world
                .get::<crate::creature_ai::CreatureState>(*watcher)
                .is_some_and(|state| state.is_idle())
        })
        .filter(|watcher| {
            targets::relation_matches(world, unit, *watcher, targets::Relation::Enemy)
        })
        .collect();
    let mut geometry = SystemState::<AggroDistance>::new(world);
    for watcher in watchers {
        let radius = pull_route::read_native_radius(world, &mut geometry, unit, watcher)?;
        let center = targets::position(world, watcher).ok_or("Unpulled watcher lacks position")?;
        if pull_route::segment_distance_squared(center, from, to) < radius * radius {
            return Ok(true);
        }
    }
    Ok(false)
}
