//! Select a fight position away from unrelated idle hostiles.
use bevy::ecs::system::SystemState;
use bevy::prelude::*;

use super::{Follower, ai, owner_trail, positioning, pull_route, tank_planner};
use crate::creature_aggro::AggroDistance;
use crate::maps::InMap;
use crate::networking::CombatTarget;
use crate::spell_cast::targets;

#[derive(Component)]
struct PullBack {
    target: Entity,
    candidates: Vec<Vec3>,
    exclusions: Vec<(Vec3, f32)>,
    next: usize,
    goal: Option<Vec3>,
    owns_position: bool,
}

#[derive(Component)]
pub(super) struct WaitingForPullBack;

pub(super) fn party_should_wait(world: &World, owner: Entity) -> bool {
    world.get::<WaitingForPullBack>(owner).is_some()
}

pub(super) fn active(world: &World, tank: Entity) -> bool {
    world.get::<PullBack>(tank).is_some()
}

pub(super) fn read_exclusions(
    world: &mut World,
    tank: Entity,
    target: Entity,
    reach: f32,
) -> Result<Vec<(Vec3, f32)>, String> {
    let map = *world.get::<InMap>(tank).ok_or("Pull-back tank lacks map")?;
    if !reach.is_finite() || reach < 0.0 {
        return Err("Pull-back party reach is invalid".into());
    }
    let watchers: Vec<_> = world
        .resource::<crate::maps::units::MapUnits>()
        .in_map(map)
        .filter(|&actor| {
            positioning::is_unpulled_hostile(world, tank, actor)
                && !world
                    .get::<shared::components::CombatStatus>(actor)
                    .is_some_and(|status| status.0)
                && !positioning::in_target_formation(world, target, actor)
        })
        .collect();
    let mut geometry = SystemState::<AggroDistance>::new(world);
    let mut exclusion = Vec::new();
    for watcher in watchers {
        let radius = pull_route::read_native_radius(world, &mut geometry, tank, watcher)? + reach;
        let center = targets::position(world, watcher).ok_or("Idle hostile lacks position")?;
        exclusion.push((center, radius));
    }
    Ok(exclusion)
}

pub(super) fn outside_exclusions(point: Vec3, exclusions: &[(Vec3, f32)]) -> bool {
    point.is_finite()
        && exclusions
            .iter()
            .all(|(center, radius)| point.distance_squared(*center) >= radius * radius)
}

/// Recent ground the owner actually traversed, newest first, plus the tank's current position.
pub(super) fn safe_candidates(
    world: &World,
    tank: Entity,
    owner: Entity,
    map: InMap,
    exclusions: &[(Vec3, f32)],
) -> Vec<Vec3> {
    targets::position(world, tank)
        .into_iter()
        .chain(
            owner_trail::combat_points(world, owner, map)
                .into_iter()
                .rev(),
        )
        .filter(|point| outside_exclusions(*point, exclusions))
        .collect()
}

fn clear(world: &mut World, tank: Entity, owner: Entity) {
    if world
        .entity_mut(tank)
        .take::<PullBack>()
        .is_some_and(|state| state.owns_position)
    {
        world
            .entity_mut(tank)
            .remove::<tank_planner::AdmittedPosition>();
        if let Some(mut navigation) = world.get_resource_mut::<crate::navigation::Navigation>() {
            navigation.cancel_route(tank);
            navigation.cancel_query(tank, crate::navigation::Goal::PullAdmission);
        }
    }
    world.entity_mut(owner).remove::<WaitingForPullBack>();
}

/// Called once per tank before party actions. `reach` comes from the party's native offensive abilities.
pub(super) fn update(
    world: &mut World,
    tank: Entity,
    follower: &Follower,
    party: &[Entity],
    reach: f32,
) {
    let enemies = ai::engaged_enemies(world, follower, tank, party);
    let existing = world.get::<PullBack>(tank).map(|state| state.target);
    if existing.is_some_and(|target| !enemies.iter().any(|(enemy, _)| *enemy == target)) {
        clear(world, tank, follower.owner);
    }
    let target = world
        .get::<PullBack>(tank)
        .map(|state| state.target)
        .or_else(|| {
            enemies.iter().find_map(|(enemy, _)| {
                let position = targets::position(world, *enemy)?;
                match read_exclusions(world, tank, *enemy, reach) {
                    Ok(exclusions)
                        if !outside_exclusions(position, &exclusions)
                            || party
                                .iter()
                                .copied()
                                .filter(|member| *member != follower.owner)
                                .filter(|member| targets::is_alive(world, *member))
                                .any(|member| {
                                    targets::position(world, member)
                                        .is_none_or(|point| !outside_exclusions(point, &exclusions))
                                }) =>
                    {
                        Some(*enemy)
                    }
                    Ok(_) => None,
                    Err(error) => {
                        warn!(?tank, ?enemy, %error, "Pull-back safety geometry unavailable");
                        Some(*enemy)
                    }
                }
            })
        });
    let Some(target) = target else {
        clear(world, tank, follower.owner);
        return;
    };
    let exclusions = match read_exclusions(world, tank, target, reach) {
        Ok(exclusions) => exclusions,
        Err(error) => {
            warn!(?tank, %error, "Pull-back safety geometry unavailable");
            world.entity_mut(follower.owner).insert(WaitingForPullBack);
            if world.get::<PullBack>(tank).is_none() {
                world.entity_mut(tank).insert(PullBack {
                    target,
                    candidates: Vec::new(),
                    exclusions: Vec::new(),
                    next: 0,
                    goal: None,
                    owns_position: false,
                });
            }
            if let Some(here) = targets::position(world, tank) {
                world
                    .entity_mut(tank)
                    .insert(tank_planner::AdmittedPosition(here));
                world
                    .get_mut::<PullBack>(tank)
                    .expect("pull-back state")
                    .owns_position = true;
                tank_planner::apply_admitted_position(world, tank, Some(target));
            }
            return;
        }
    };
    if world.get::<PullBack>(tank).is_none() {
        let Some(map) = world.get::<InMap>(tank).copied() else {
            return;
        };
        let candidates = safe_candidates(world, tank, follower.owner, map, &exclusions);
        super::tank_frontal::clear_owned_position(world, tank);
        world.entity_mut(tank).insert(PullBack {
            target,
            candidates,
            exclusions: exclusions.clone(),
            next: 0,
            goal: None,
            owns_position: false,
        });
        world.entity_mut(follower.owner).insert(WaitingForPullBack);
    }
    let Some(here) = targets::position(world, tank) else {
        return;
    };
    let reached = outside_exclusions(here, &exclusions)
        && world
            .get::<PullBack>(tank)
            .and_then(|state| state.goal)
            .is_some_and(|goal| here.distance(goal) <= 0.75);
    let party_safe = party
        .iter()
        .copied()
        .filter(|member| *member != follower.owner)
        .filter(|member| targets::is_alive(world, *member))
        .all(|member| {
            targets::position(world, member)
                .is_some_and(|point| outside_exclusions(point, &exclusions))
        });
    if reached
        && party_safe
        && enemies.iter().all(|(enemy, _)| {
            targets::position(world, *enemy).is_some_and(|point| {
                outside_exclusions(point, &exclusions)
                    && crate::scripted_ai::model_reach::melee_range(world, *enemy, tank)
                        .is_some_and(|range| point.distance(here) <= range)
            }) && world
                .get::<CombatTarget>(*enemy)
                .and_then(|victim| victim.0)
                == Some(tank)
        })
    {
        clear(world, tank, follower.owner);
        return;
    }
    if world
        .get::<CombatTarget>(target)
        .and_then(|victim| victim.0)
        != Some(tank)
    {
        let owns_position = world
            .get::<PullBack>(tank)
            .is_some_and(|state| state.owns_position);
        if owns_position {
            world
                .entity_mut(tank)
                .remove::<tank_planner::AdmittedPosition>();
            world
                .get_mut::<PullBack>(tank)
                .expect("pull-back state")
                .owns_position = false;
        }
        return;
    }
    let Some(mut state) = world.entity_mut(tank).take::<PullBack>() else {
        return;
    };
    if state.exclusions != exclusions || (state.candidates.is_empty() && state.goal.is_none()) {
        state.candidates = safe_candidates(
            world,
            tank,
            follower.owner,
            *world.get::<InMap>(tank).expect("pull-back map"),
            &exclusions,
        );
        state.exclusions.clone_from(&exclusions);
        state.next = 0;
        if state
            .goal
            .is_some_and(|point| !outside_exclusions(point, &exclusions))
        {
            state.goal = None;
        }
    }
    if !state.owns_position {
        if let Some(mut navigation) = world.get_resource_mut::<crate::navigation::Navigation>() {
            navigation.cancel_route(tank);
            navigation.cancel_query(tank, crate::navigation::Goal::Approach(target.to_bits()));
        }
    }
    while state.goal.is_none() && state.next < state.candidates.len() {
        let point = state.candidates[state.next];
        if !outside_exclusions(point, &exclusions) {
            state.next += 1;
            continue;
        }
        let admission = if here.distance(point) <= 0.75 {
            Ok(Some(true))
        } else {
            tank_planner::read_movement_admission(world, tank, target, follower, Some(point))
        };
        match admission {
            Ok(Some(true)) => state.goal = Some(point),
            Ok(None) => break,
            Ok(Some(false)) => state.next += 1,
            Err(error) => {
                debug!(?tank, ?point, %error, "Pull-back route unavailable");
                state.next += 1;
            }
        }
    }
    if let Some(point) = state.goal {
        if !outside_exclusions(point, &exclusions) {
            state.goal = None;
            state.candidates = safe_candidates(
                world,
                tank,
                follower.owner,
                *world.get::<InMap>(tank).expect("pull-back map"),
                &exclusions,
            );
            state.next = 0;
        }
    }
    // Keep the tank in place while route admission is pending or no safe point exists.
    let point = state.goal.unwrap_or(here);
    state.owns_position = true;
    world
        .entity_mut(tank)
        .insert(tank_planner::AdmittedPosition(point));
    tank_planner::apply_admitted_position(world, tank, Some(target));
    world.entity_mut(tank).insert(state);
}
