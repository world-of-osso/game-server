//! Route a held follower along the owner's recorded safe turns.
use super::{owner_trail, positioning, pull_route};
use crate::creature_aggro::AggroDistance;
use crate::maps::InMap;
use crate::navigation::{Goal, MoveRequest, Navigation};
use ::navigation::worker::{Output, Query};
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use shared::components::Position;

const HOLD_SECONDS: f64 = 5.0;
const ARRIVAL_DISTANCE: f32 = 0.75;

#[derive(Component)]
struct TrailFollow {
    owner: Entity,
    blockers: Vec<Entity>,
    since: f64,
    candidates: Vec<Vec3>,
    destination: Option<Vec3>,
}

pub(super) fn active(world: &World, unit: Entity, owner: Entity) -> bool {
    world
        .get::<TrailFollow>(unit)
        .is_some_and(|state| state.owner == owner)
}

pub(super) fn advance(
    world: &mut World,
    unit: Entity,
    owner: Entity,
    blockers: Vec<Entity>,
) -> bool {
    let now = world.resource::<Time>().elapsed_secs_f64();
    let Some(map) = world.get::<InMap>(unit).copied() else {
        return false;
    };
    let Some(from) = world.get::<Position>(unit).copied() else {
        return false;
    };
    let origin = Vec3::new(from.x, from.y, from.z);
    if !active(world, unit, owner) {
        let candidates = owner_trail::points(world, owner, map);
        world.entity_mut(unit).insert(TrailFollow {
            owner,
            blockers,
            since: now,
            candidates,
            destination: None,
        });
    }
    let blocked = world
        .get::<TrailFollow>(unit)
        .expect("trail state")
        .blockers
        .iter()
        .any(|&watcher| positioning::is_unpulled_hostile(world, unit, watcher));
    if !blocked {
        clear(world, unit);
        return true;
    }
    let (since, destination) = {
        let state = world.get::<TrailFollow>(unit).expect("trail state");
        (state.since, state.destination)
    };
    if now - since >= HOLD_SECONDS {
        debug!(
            ?unit,
            ?owner,
            "Follower trail hold timed out; resume ordinary follow"
        );
        if let Some(mut nav) = world.get_resource_mut::<Navigation>() {
            nav.cancel_query(unit, Goal::OwnerTrail(owner.to_bits()));
        }
        return false;
    }
    if let Some(to) = destination {
        if origin.distance(to) <= ARRIVAL_DISTANCE {
            let candidates = owner_trail::points(world, owner, map);
            let mut state = world.get_mut::<TrailFollow>(unit).expect("trail state");
            state.destination = None;
            state.candidates = candidates;
            return true;
        }
        let goal = Goal::OwnerTrail(owner.to_bits());
        super::hazards::advance_escape(
            world,
            unit,
            map.map_id,
            from,
            Position {
                x: to.x,
                y: to.y,
                z: to.z,
            },
            goal,
        );
        if world
            .get::<Position>(unit)
            .is_some_and(|after| *after != from)
        {
            world
                .get_mut::<TrailFollow>(unit)
                .expect("trail state")
                .since = now;
        }
        return true;
    }
    let mut candidates = world
        .get::<TrailFollow>(unit)
        .expect("trail state")
        .candidates
        .clone();
    if candidates.is_empty() {
        candidates = owner_trail::points(world, owner, map);
        world
            .get_mut::<TrailFollow>(unit)
            .expect("trail state")
            .candidates = candidates.clone();
    }
    if candidates.is_empty() {
        return true;
    }
    let Some(mut nav) = world.get_resource_mut::<Navigation>() else {
        error!(?unit, "Follower owner trail requires navigation");
        return true;
    };
    let goal = Goal::OwnerTrail(owner.to_bits());
    let result = nav.query_deferred(
        unit,
        map.map_id,
        goal,
        Query::Candidates {
            from: origin,
            targets: candidates.clone(),
        },
    );
    let paths = match result {
        None => return true,
        Some(Ok(Some(Output::Candidates(paths)))) => paths,
        Some(other) => {
            error!(?unit, ?other, "Follower owner trail navigation failed");
            return true;
        }
    };
    let Some((to, path)) = choose_route(world, unit, map, origin, candidates, paths) else {
        world
            .get_mut::<TrailFollow>(unit)
            .expect("trail state")
            .candidates
            .clear();
        world.resource_mut::<Navigation>().cancel_query(unit, goal);
        return true;
    };
    world.resource_mut::<Navigation>().adopt_candidate_route(
        unit,
        MoveRequest {
            map_id: map.map_id,
            goal,
            from,
            to: Position {
                x: to.x,
                y: to.y,
                z: to.z,
            },
            distance: 0.0,
            reach: 0.0,
        },
        path,
    );
    let mut state = world.get_mut::<TrailFollow>(unit).expect("trail state");
    state.destination = Some(to);
    state.since = now;
    true
}

fn choose_route(
    world: &mut World,
    unit: Entity,
    map: InMap,
    origin: Vec3,
    candidates: Vec<Vec3>,
    paths: Vec<Result<Vec<Vec3>, String>>,
) -> Option<(Vec3, Vec<Vec3>)> {
    let watchers: Vec<_> = world
        .resource::<crate::maps::units::MapUnits>()
        .in_map(map)
        .filter(|&watcher| positioning::is_unpulled_hostile(world, unit, watcher))
        .collect();
    let mut geometry = SystemState::<AggroDistance>::new(world);
    let mut circles = Vec::new();
    for watcher in watchers {
        let radius = pull_route::read_native_radius(world, &mut geometry, unit, watcher).ok()?;
        let center = crate::spell_cast::targets::position(world, watcher)?;
        circles.push((center, radius));
    }
    candidates
        .into_iter()
        .zip(paths)
        .rev()
        .find_map(|(to, path)| {
            let path = path.ok()?;
            if origin.distance(to) <= ARRIVAL_DISTANCE {
                return None;
            }
            let mut points = vec![origin];
            points.extend(path.iter().copied());
            let safe = circles.iter().all(|(center, radius)| {
                points.windows(2).all(|leg| {
                    pull_route::segment_distance_squared(*center, leg[0], leg[1]) >= radius * radius
                })
            });
            safe.then_some((to, path))
        })
}

pub(super) fn clear(world: &mut World, unit: Entity) {
    let Some(state) = world.entity_mut(unit).take::<TrailFollow>() else {
        return;
    };
    let goal = Goal::OwnerTrail(state.owner.to_bits());
    if let Some(mut nav) = world.get_resource_mut::<Navigation>() {
        nav.cancel_query(unit, goal);
        if nav.route_destination(unit, goal).is_some() {
            nav.cancel_route(unit);
        }
    }
}
