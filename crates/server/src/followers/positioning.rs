//! Project positioning policy: non-tanks do not approach through idle native sight circles.
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use shared::components::Health;

use super::{kit_ai, pull_route};
use crate::creature_aggro::AggroDistance;
use crate::maps::InMap;
use crate::navigation::{Goal, Navigation};
use crate::spell_cast::targets;
use ::navigation::worker::{Output, Query as NavQuery};

pub(super) fn hold_unsafe_approach(world: &mut World, unit: Entity, target: Entity) -> bool {
    if kit_ai::is_tank(world, unit) {
        return false;
    }
    let unsafe_approach = match approach_enters_unpulled_sight(world, unit, target) {
        Ok(unsafe_approach) => unsafe_approach.unwrap_or(true),
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
) -> Result<Option<bool>, String> {
    let map = *world
        .get::<InMap>(unit)
        .ok_or("Follower positioning lacks map")?;
    let from = targets::position(world, unit).ok_or("Follower positioning lacks origin")?;
    let to =
        targets::position(world, target).ok_or("Follower positioning lacks target position")?;
    let watchers: Vec<_> = world
        .resource::<crate::maps::units::MapUnits>()
        .in_map(map)
        .filter(|watcher| is_unpulled_hostile(world, unit, *watcher))
        .collect();
    if watchers.is_empty() {
        return Ok(Some(false));
    }
    let Some(mut path) = read_approach_path(world, unit, target, map, from, to)? else {
        return Ok(None);
    };
    path.insert(0, from);
    let mut geometry = SystemState::<AggroDistance>::new(world);
    for watcher in watchers {
        let radius = pull_route::read_native_radius(world, &mut geometry, unit, watcher)?;
        let center = targets::position(world, watcher).ok_or("Unpulled watcher lacks position")?;
        if path.windows(2).any(|leg| {
            pull_route::segment_distance_squared(center, leg[0], leg[1]) < radius * radius
        }) {
            return Ok(Some(true));
        }
    }
    Ok(Some(false))
}

fn read_approach_path(
    world: &mut World,
    unit: Entity,
    target: Entity,
    map: InMap,
    from: Vec3,
    to: Vec3,
) -> Result<Option<Vec<Vec3>>, String> {
    #[cfg(test)]
    if let Some(space) = world.get_resource::<crate::dungeon_simulator::Space>() {
        return space.route(from, to).map(Some);
    }
    let Some(result) = world
        .get_resource_mut::<Navigation>()
        .ok_or("Follower positioning requires authored navigation")?
        .query_deferred(
            unit,
            map.map_id,
            Goal::Approach(target.to_bits()),
            NavQuery::Path { from, to },
        )
    else {
        return Ok(None);
    };
    match result? {
        Some(Output::Path(path)) if !path.is_empty() => Ok(Some(path)),
        Some(Output::Path(_)) => Err("Follower positioning route is empty".into()),
        None => Err("Follower positioning has no authored navigation bake".into()),
        Some(_) => Err("Follower positioning received a non-path result".into()),
    }
}

fn is_unpulled_hostile(world: &World, unit: Entity, watcher: Entity) -> bool {
    // Match CreatureSight's optional react state: absent means Aggressive.
    let aggressive = world
        .get::<crate::creature_aggro::AggroRange>(watcher)
        .is_some()
        && world
            .get::<crate::scripted_ai::ReactState>(watcher)
            .is_none_or(|react| *react == crate::scripted_ai::ReactState::Aggressive);
    let alive = world
        .get::<Health>(watcher)
        .is_some_and(|health| health.current > 0.0);
    let idle = world
        .get::<crate::creature_ai::CreatureState>(watcher)
        .is_some_and(|state| state.is_idle());
    aggressive
        && alive
        && idle
        && targets::relation_matches(world, unit, watcher, targets::Relation::Enemy)
}
