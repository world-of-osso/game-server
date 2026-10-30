//! Project positioning policy: non-tanks do not approach through idle native sight circles.
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use shared::components::{Health, UnitFactionTemplate};

use super::{kit_ai, pull_route};
use crate::creature_aggro::AggroDistance;
use crate::faction::{FactionTemplates, Reaction};
use crate::maps::InMap;
use crate::navigation::{Goal, Navigation};
use crate::spell_cast::targets;
use ::navigation::worker::{Output, Query as NavQuery};

const HOLD_TIMEOUT_SECONDS: f64 = 5.0;

#[derive(Component)]
struct HoldTimeout {
    target: Entity,
    since: f64,
}

fn bounded_hold(world: &mut World, unit: Entity, target: Entity) -> bool {
    let now = world.resource::<Time>().elapsed_secs_f64();
    if world
        .get::<HoldTimeout>(unit)
        .is_none_or(|hold| hold.target != target)
    {
        world
            .entity_mut(unit)
            .insert(HoldTimeout { target, since: now });
    }
    let hold =
        now - world.get::<HoldTimeout>(unit).expect("hold timer").since < HOLD_TIMEOUT_SECONDS;
    if hold {
        hold_position(world, unit);
    }
    hold
}

fn hold_position(world: &mut World, unit: Entity) {
    world
        .entity_mut(unit)
        .remove::<crate::creature_ai::FollowOwner>()
        .insert(crate::creature_ai::CreatureState::Idle);
}

fn hold_pending_approach(world: &mut World, unit: Entity, target: Entity, is_owner: bool) -> bool {
    if is_owner {
        return bounded_hold(world, unit, target);
    }
    hold_position(world, unit);
    true
}

pub(super) fn hold_unsafe_approach(world: &mut World, unit: Entity, target: Entity) -> bool {
    let is_owner = world
        .get::<super::Follower>(unit)
        .is_some_and(|follower| follower.owner == target);
    if !is_owner {
        super::trail_follow::clear(world, unit);
    }
    if kit_ai::is_tank(world, unit) && !is_owner {
        return false;
    }
    if is_owner && super::trail_follow::active(world, unit, target) {
        return super::trail_follow::advance(world, unit, target, Vec::new());
    }
    let blockers = match approach_enters_unpulled_sight(world, unit, target) {
        Ok(None) => {
            debug!(
                ?unit,
                ?target,
                "Follower approach waiting for a navigation result"
            );
            return hold_pending_approach(world, unit, target, is_owner);
        }
        Ok(Some(blockers)) => blockers,
        Err(error) => {
            warn!(?unit, %error, "Follower positioning lacks native sight geometry");
            return hold_pending_approach(world, unit, target, is_owner);
        }
    };
    if blockers.is_empty() {
        super::trail_follow::clear(world, unit);
        world.entity_mut(unit).remove::<HoldTimeout>();
        return false;
    }
    hold_position(world, unit);
    if is_owner {
        return super::trail_follow::advance(world, unit, target, blockers);
    }
    true
}

fn approach_enters_unpulled_sight(
    world: &mut World,
    unit: Entity,
    target: Entity,
) -> Result<Option<Vec<Entity>>, String> {
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
        return Ok(Some(Vec::new()));
    }
    let Some(mut path) = read_approach_path(world, unit, target, map, from, to)? else {
        return Ok(None);
    };
    path.insert(0, from);
    let mut geometry = SystemState::<AggroDistance>::new(world);
    let mut blockers = Vec::new();
    for watcher in watchers {
        let radius = pull_route::read_native_radius(world, &mut geometry, unit, watcher)?;
        let center = targets::position(world, watcher).ok_or("Unpulled watcher lacks position")?;
        if path.windows(2).any(|leg| {
            pull_route::segment_distance_squared(center, leg[0], leg[1]) < radius * radius
        }) {
            debug!(
                ?unit,
                ?target,
                ?watcher,
                ?center,
                radius,
                "Follower approach held by unpulled watcher"
            );
            blockers.push(watcher);
        }
    }
    Ok(Some(blockers))
}

fn read_approach_path(
    world: &mut World,
    unit: Entity,
    target: Entity,
    map: InMap,
    from: Vec3,
    to: Vec3,
) -> Result<Option<Vec<Vec3>>, String> {
    #[cfg(any(test, feature = "test-support"))]
    if let Some(space) = world.get_resource::<crate::test_support::Space>() {
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
        Some(Output::Path(path)) => {
            let points = path.into_complete()?;
            if points.is_empty() {
                return Err("Follower positioning route is empty".into());
            }
            Ok(Some(points))
        }
        None => Err("Follower positioning has no authored navigation bake".into()),
        Some(_) => Err("Follower positioning received a non-path result".into()),
    }
}

pub(super) fn is_unpulled_hostile(world: &World, unit: Entity, watcher: Entity) -> bool {
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
    // Creature::CanStartAttack: aggro needs a hostile reaction, not just attackability.
    let hostile = world
        .get::<UnitFactionTemplate>(watcher)
        .is_some_and(|faction| {
            let follower = world.get::<UnitFactionTemplate>(unit).map_or(0, |f| f.0);
            world
                .resource::<FactionTemplates>()
                .reaction(faction.0, follower)
                == Reaction::Hostile
        });
    aggressive
        && alive
        && idle
        && hostile
        && targets::relation_matches(world, unit, watcher, targets::Relation::Enemy)
}
