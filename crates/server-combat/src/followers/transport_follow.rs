//! A follower independently uses a transport its owner used when walking cannot reunite them.
use std::collections::VecDeque;

use bevy::prelude::*;
use shared::components::Position;

use crate::creature_ai::{CreatureState, FollowOwner};
use crate::maps::InMap;
use crate::navigation::{Goal, Navigation};
use ::navigation::worker::{Output, Query};

#[derive(Clone, Copy)]
pub(crate) enum TransportKind {
    AzureBook,
    NalorakkPyre,
    Petalwing,
    AltarDrop,
}

#[derive(Clone, Copy)]
struct Request {
    source: Entity,
    destination: Vec3,
    kind: TransportKind,
    phase: Phase,
}

#[derive(Clone, Copy)]
enum Phase {
    CheckDestination,
    CheckSource,
    Approach,
}

#[derive(Component, Default)]
struct Pending(VecDeque<Request>);

/// Record the actual interaction; each follower decides whether it needs the same transport.
pub(crate) fn record_use(world: &mut World, owner: Entity, source: Entity, kind: TransportKind) {
    let Some(destination) = world.get::<Position>(owner).copied() else {
        return;
    };
    record_use_to(world, owner, source, kind, point(destination));
}

pub(crate) fn record_use_to(
    world: &mut World,
    owner: Entity,
    source: Entity,
    kind: TransportKind,
    destination: Vec3,
) {
    let Some(copy) = world.get::<InMap>(owner).copied() else {
        return;
    };
    let followers: Vec<_> = world
        .query::<(Entity, &super::Follower, &InMap)>()
        .iter(world)
        .filter_map(|(unit, follower, map)| {
            (follower.owner == owner
                && *map == copy
                && crate::spell_cast::targets::is_alive(world, unit))
            .then_some(unit)
        })
        .collect();
    for unit in followers {
        if let Some(mut navigation) = world.get_resource_mut::<Navigation>() {
            navigation.cancel_route(unit);
        }
        let mut actor = world.entity_mut(unit);
        actor.remove::<FollowOwner>();
        actor.insert((CreatureState::Idle, crate::networking::CombatTarget(None)));
        let request = Request {
            source,
            destination,
            kind,
            phase: Phase::CheckDestination,
        };
        if let Some(mut pending) = actor.get_mut::<Pending>() {
            pending.0.push_back(request);
        } else {
            actor.insert(Pending(VecDeque::from([request])));
        }
    }
}

fn point(position: Position) -> Vec3 {
    Vec3::new(position.x, position.y, position.z)
}

/// None is a pending native navigation query; false is a disconnected route.
fn can_walk(
    world: &mut World,
    unit: Entity,
    map: InMap,
    target: Entity,
    from: Vec3,
    to: Vec3,
) -> Result<Option<bool>, String> {
    #[cfg(any(test, feature = "test-support"))]
    if let Some(space) = world.get_resource::<crate::test_support::Space>() {
        return Ok(Some(space.route(from, to).is_ok()));
    }
    let Some(result) = world
        .get_resource_mut::<Navigation>()
        .ok_or("follower transport requires navigation")?
        .query_deferred(
            unit,
            map.map_id,
            Goal::Approach(target.to_bits()),
            Query::CompleteRoute { from, to },
        )
    else {
        return Ok(None);
    };
    match result? {
        Some(Output::CompleteRoute(route)) => Ok(Some(route.complete)),
        None => Err("follower transport has no navigation bake".into()),
        Some(_) => Err("follower transport received an invalid navigation response".into()),
    }
}

fn finish_request(world: &mut World, unit: Entity) {
    let mut actor = world.entity_mut(unit);
    actor.remove::<FollowOwner>();
    if let Some(mut pending) = actor.get_mut::<Pending>() {
        pending.0.pop_front();
        if pending.0.is_empty() {
            actor.remove::<Pending>();
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
pub(crate) fn in_progress(world: &World, unit: Entity) -> bool {
    world.get::<Pending>(unit).is_some()
}

/// True while transport routing owns the follower's movement.
pub(super) fn update(world: &mut World, unit: Entity, follower: &super::Follower) -> bool {
    let Some(request) = world
        .get::<Pending>(unit)
        .and_then(|pending| pending.0.front())
        .copied()
    else {
        return false;
    };
    let Some(map) = world.get::<InMap>(unit).copied() else {
        finish_request(world, unit);
        return true;
    };
    let Some(from) = world.get::<Position>(unit).copied().map(point) else {
        finish_request(world, unit);
        return true;
    };
    let Some(source) = world.get::<Position>(request.source).copied().map(point) else {
        warn!(?unit, "follower transport source disappeared");
        finish_request(world, unit);
        return true;
    };
    if world.get::<InMap>(request.source) != Some(&map)
        || world.get::<InMap>(follower.owner) != Some(&map)
    {
        warn!(?unit, "follower transport left its map copy");
        finish_request(world, unit);
        return true;
    }
    match request.phase {
        Phase::CheckDestination | Phase::CheckSource => {
            let (target, to) = match request.phase {
                Phase::CheckDestination => (follower.owner, request.destination),
                _ => (request.source, source),
            };
            match can_walk(world, unit, map, target, from, to) {
                Ok(None) => {}
                Ok(Some(reachable)) => {
                    if matches!(request.phase, Phase::CheckDestination) && reachable {
                        finish_request(world, unit);
                    } else if matches!(request.phase, Phase::CheckSource) && !reachable {
                        warn!(?unit, "follower cannot reach the registered transport");
                        finish_request(world, unit);
                    } else if let Some(mut pending) = world.get_mut::<Pending>(unit) {
                        pending.0.front_mut().expect("pending request").phase =
                            if matches!(request.phase, Phase::CheckDestination) {
                                Phase::CheckSource
                            } else {
                                Phase::Approach
                            };
                    }
                }
                Err(error) => {
                    warn!(?unit, %error, "follower transport route failed");
                    finish_request(world, unit);
                }
            }
        }
        Phase::Approach => {
            if from.distance(source) <= crate::npc_interaction::INTERACTION_DISTANCE {
                let result = match request.kind {
                    TransportKind::AzureBook => {
                        crate::npc_interaction::azure_translocation::traverse(
                            world,
                            unit,
                            request.source,
                        )
                        .map(|_| ())
                    }
                    TransportKind::NalorakkPyre => {
                        crate::npc_interaction::nalorakk_pyre::traverse(world, unit, request.source)
                    }
                    TransportKind::Petalwing => {
                        crate::npc_interaction::petalwing::traverse(world, unit, request.source)
                    }
                    TransportKind::AltarDrop => {
                        crate::scripted_ai::altar_drop::traverse(world, unit, request.source)
                    }
                };
                if let Err(error) = result {
                    warn!(?unit, %error, "follower transport interaction failed");
                }
                finish_request(world, unit);
            } else {
                world
                    .entity_mut(unit)
                    .insert((CreatureState::Idle, FollowOwner::new(request.source)));
            }
        }
    }
    true
}
