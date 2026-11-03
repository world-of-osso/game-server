//! Simulated ground movers follow complete native navigation paths.
use super::{campaign::MoveIntent, native_movement::NativeMovement};
use bevy::prelude::*;
use shared::components::{Health, Position, Rotation};

#[path = "movement_audit_tests.rs"]
mod audit_tests;
#[path = "movement_edge_tests.rs"]
mod edge_tests;
#[path = "movement_reach_tests.rs"]
mod reach_tests;

#[derive(Component)]
struct BlockedWalk {
    reason: String,
}

#[derive(Component, Clone)]
struct Travel {
    destination: Vec3,
    legs: std::collections::VecDeque<(Vec3, f32)>,
}

#[derive(Component, Clone, Copy)]
pub(super) struct SafeOwnerApproach(pub Entity);

#[derive(Component)]
struct FlightTravel {
    phase: FlightPhase,
    travel: Travel,
}

#[derive(Clone, Copy)]
enum FlightPhase {
    Mounting,
    Flying,
    Landing,
}

pub(super) fn flight_in_progress(world: &World, owner: Entity) -> bool {
    world.get::<FlightTravel>(owner).is_some()
}

pub(super) fn begin_flight(
    world: &mut World,
    owner: Entity,
    points: &[[f32; 3]],
) -> Result<(), String> {
    let position = point(
        world
            .get::<Position>(owner)
            .ok_or("flight leader lacks position")?,
    );
    if points.len() < 2 {
        return Err("flight path needs at least two points".into());
    }
    let first = Vec3::from_array(points[0]);
    if position.distance(first) > 0.001 {
        return Err(format!(
            "flight leader is not at the validated takeoff: {position:?} vs {first:?}"
        ));
    }
    let positions: Vec<_> = points.iter().copied().map(Vec3::from_array).collect();
    let legs = positions
        .windows(2)
        .map(|pair| (pair[1], pair[0].distance(pair[1])))
        .collect();
    let destination = *positions.last().unwrap();
    world.entity_mut(owner).remove::<(Travel, MoveIntent)>();
    world.entity_mut(owner).insert(FlightTravel {
        phase: FlightPhase::Mounting,
        travel: Travel { destination, legs },
    });
    crate::creature_ai::publish_planned_destination(world, owner, Some(destination));
    Ok(())
}

pub(super) fn forget_party_travel(world: &mut World, owner: Entity) {
    let mut units: Vec<_> = world
        .query::<(Entity, &crate::followers::Follower)>()
        .iter(world)
        .filter(|(_, follower)| follower.owner == owner)
        .map(|(unit, _)| unit)
        .collect();
    units.push(owner);
    let pets: Vec<_> = units
        .iter()
        .flat_map(|unit| crate::hunter_pet::active_owned_pets(world, *unit))
        .collect();
    units.extend(pets);
    for unit in units {
        world.entity_mut(unit).remove::<(Travel, FlightTravel)>();
        crate::creature_ai::publish_planned_destination(world, unit, None);
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum RouteGoal {
    Point(Vec3),
    Unit(Entity, Vec3),
}

pub(super) fn remaining_route(world: &World, unit: Entity) -> Option<(RouteGoal, f32)> {
    if let Some(flight) = world.get::<FlightTravel>(unit) {
        return Some((
            RouteGoal::Point(flight.travel.destination),
            flight.travel.legs.iter().map(|(_, yards)| yards).sum(),
        ));
    }
    let travel = world.get::<Travel>(unit)?;
    let goal = if world.get::<MoveIntent>(unit).is_some()
        || world.get::<crate::creature_ai::CreatureState>(unit)
            == Some(&crate::creature_ai::CreatureState::Returning)
    {
        RouteGoal::Point(travel.destination)
    } else if let Some(crate::creature_ai::CreatureState::Pursuing { target }) =
        world.get::<crate::creature_ai::CreatureState>(unit)
    {
        RouteGoal::Unit(Entity::try_from_bits(*target)?, travel.destination)
    } else if let Some(follow) = world
        .get::<crate::creature_ai::FollowOwner>(unit)
        .filter(|follow| follow.owner != unit)
    {
        RouteGoal::Unit(follow.owner, travel.destination)
    } else {
        RouteGoal::Point(travel.destination)
    };
    Some((goal, travel.legs.iter().map(|(_, yards)| yards).sum()))
}

pub(super) fn inspect_travel(world: &World, unit: Entity) -> serde_json::Value {
    let Some(travel) = world.get::<Travel>(unit) else {
        return world.get::<BlockedWalk>(unit).map_or(
            serde_json::Value::Null,
            |blocked| serde_json::json!({"blocked": blocked.reason}),
        );
    };
    serde_json::json!({
        "destination": travel.destination.to_array(),
        "legs": travel.legs.iter().map(|(point, remaining)| {
            serde_json::json!({"to": point.to_array(), "remaining_yards": remaining})
        }).collect::<Vec<_>>(),
    })
}

fn point(position: &Position) -> Vec3 {
    Vec3::new(position.x, position.y, position.z)
}

pub(super) fn move_owner_to_tank(
    world: &mut World,
    owner: Entity,
    tank: Entity,
) -> Result<(), String> {
    let from = point(
        world
            .get::<Position>(owner)
            .ok_or("cohesion owner absent")?,
    );
    let target = point(world.get::<Position>(tank).ok_or("cohesion tank absent")?);
    let destination = world
        .get_resource::<NativeMovement>()
        .ok_or("native movement map is not installed")?
        .cohesion_destination(from, target)
        .map_err(|error| format!("cohesion approach from {from:?} toward {target:?}: {error}"))?;
    world.entity_mut(owner).insert(MoveIntent(destination));
    Ok(())
}

fn destination(world: &World, unit: Entity, from: Vec3) -> Option<(Vec3, f32)> {
    if world.get::<crate::creature_ai::CreatureState>(unit)
        == Some(&crate::creature_ai::CreatureState::Returning)
    {
        let home = crate::creature_ai::return_home_position(
            world.get::<crate::creature_ai::SpawnPoint>(unit)?,
            world.get::<crate::creature_ai::HomePosition>(unit),
            world.get::<crate::scripted_ai::movement::ScriptedPath>(unit),
        );
        return Some((point(&home), 0.0));
    }
    if let Some(intent) = world.get::<MoveIntent>(unit) {
        return Some((intent.0, 0.0));
    }
    if let Some(crate::creature_ai::CreatureState::Pursuing { target }) =
        world.get::<crate::creature_ai::CreatureState>(unit)
    {
        let target = Entity::try_from_bits(*target)?;
        let to = point(world.get::<Position>(target)?);
        let fixture_melee_target = world
            .get::<crate::simc_apl_native::FixtureMovement>(unit)
            .is_some()
            && world
                .get::<crate::combat::AutoAttack>(unit)
                .is_some_and(|attack| attack.victim == target);
        let reach = if fixture_melee_target {
            crate::scripted_ai::model_reach::melee_range(world, unit, target)?
        } else {
            world
                .get::<crate::followers::FollowerReach>(unit)
                .map_or(crate::creature_ai::CHASE_REACH, |reach| reach.0)
        };
        return Some((to, reach));
    }
    if let Some(follow) = world.get::<crate::creature_ai::FollowOwner>(unit) {
        if follow.owner != unit {
            return Some((point(world.get::<Position>(follow.owner)?), 4.0));
        }
    }
    match world.get::<crate::creature_ai::PetMotion>(unit)? {
        crate::creature_ai::PetMotion::Chase(target) => Some((
            point(world.get::<Position>(*target)?),
            crate::creature_ai::CHASE_REACH,
        )),
        crate::creature_ai::PetMotion::Point(to) => Some((point(to), 0.0)),
        crate::creature_ai::PetMotion::Idle => Some((from, 0.0)),
    }
}

pub(super) fn read_speed(world: &World, unit: Entity) -> f32 {
    world
        .get::<crate::creature_ai::SpawnPoint>(unit)
        .map_or(1.0, |spawn| spawn.speed_run)
        * crate::creature_ai::BASE_RUN_SPEED
        * world
            .get::<crate::auras::AuraStatMods>(unit)
            .map_or(1.0, |mods| mods.movement_speed)
}

pub(super) fn can_move(world: &World, unit: Entity) -> bool {
    world
        .get::<Health>(unit)
        .is_some_and(|health| health.current > 0.0)
        && !server::test_support::transport::in_progress(world, unit)
        && world
            .get::<shared::casting::CastState>(unit)
            .is_none_or(|cast| cast.cast_while_moving)
        && world.get::<crate::auras::Confused>(unit).is_none()
        && world.get::<crate::auras::Feared>(unit).is_none()
}

fn movement_label(world: &World, unit: Entity) -> &'static str {
    if world.get::<MoveIntent>(unit).is_some() {
        "intent"
    } else if matches!(
        world.get::<crate::creature_ai::CreatureState>(unit),
        Some(crate::creature_ai::CreatureState::Pursuing { .. })
    ) {
        "pursuit"
    } else if world.get::<crate::creature_ai::FollowOwner>(unit).is_some() {
        "follow"
    } else {
        "other"
    }
}

fn advance(mut from: Vec3, travel: &mut Travel, mut yards: f32, reach: f32) -> Vec3 {
    while !travel.legs.is_empty() {
        let last = travel.legs.len() == 1;
        let (to, remaining) = travel.legs.front_mut().unwrap();
        let available = if last && reach > 0.0 {
            let horizontal = (to.x - from.x).hypot(to.z - from.z);
            let distance =
                server::test_support::distance_to_reach(from, *to, travel.destination, reach);
            if horizontal > 0.0 {
                *remaining * (distance / horizontal)
            } else {
                0.0
            }
        } else {
            *remaining
        };
        let step = yards.min(available);
        let fraction = if *remaining > 0.0 {
            step / *remaining
        } else {
            1.0
        };
        from = from.lerp(*to, fraction);
        *remaining -= step;
        yards -= step;
        if *remaining <= 0.0001 {
            travel.legs.pop_front();
        } else {
            break;
        }
        if yards <= 0.0 {
            break;
        }
    }
    from
}

fn handle_unreachable_movement(
    world: &mut World,
    unit: Entity,
    from: Vec3,
    to: Vec3,
    error: String,
) -> Result<(), String> {
    if let Some(crate::creature_ai::CreatureState::Pursuing { target }) =
        world.get::<crate::creature_ai::CreatureState>(unit)
    {
        crate::creature_ai::mark_chase_blocked(world, unit, *target);
        let position = *world.get::<Position>(unit).unwrap();
        crate::creature_ai::move_follower(world, unit, position, 0.0);
        crate::creature_ai::publish_planned_destination(world, unit, None);
        return Ok(());
    }
    if (world.get::<MoveIntent>(unit).is_none()
        && world.get::<crate::creature_ai::FollowOwner>(unit).is_some())
        || matches!(
            world.get::<crate::creature_ai::PetMotion>(unit),
            Some(crate::creature_ai::PetMotion::Chase(_))
        )
    {
        let reason = format!("{error}; from={from:?} to={to:?}");
        if world.get::<BlockedWalk>(unit).is_none() {
            eprintln!("NATIVE_WALK_BLOCKED actor={unit:?} {reason}");
        }
        world.entity_mut(unit).insert(BlockedWalk { reason });
        let position = *world.get::<Position>(unit).unwrap();
        crate::creature_ai::move_follower(world, unit, position, 0.0);
        crate::creature_ai::publish_planned_destination(world, unit, None);
        return Ok(());
    }
    Err(format!(
        "{error}; source={:?} from={from:?} to={to:?} state={:?}",
        world
            .get::<crate::zones::CreatureOrigin>(unit)
            .map(|origin| origin.guid),
        world.get::<crate::creature_ai::CreatureState>(unit),
    ))
}

fn move_one(world: &mut World, unit: Entity, dt: f32) -> Result<(), String> {
    let apl_owns_movement = world
        .get::<crate::simc_apl_native::AplMovementIntent>(unit)
        .is_some();
    if apl_owns_movement
        && matches!(
            world.get::<crate::creature_ai::CreatureState>(unit),
            Some(crate::creature_ai::CreatureState::Pursuing { .. })
        )
    {
        world.entity_mut(unit).remove::<Travel>();
        return Ok(());
    }
    if flight_in_progress(world, unit) {
        return advance_flight(world, unit, dt);
    }
    if world
        .get::<crate::creature_ai::flight_follow::FlightFollow>(unit)
        .is_some()
    {
        return Ok(());
    }
    if let Some(mut follow) = world.get_mut::<crate::creature_ai::FollowOwner>(unit)
        && follow.owner != unit
    {
        follow.set_moving(false);
    }
    if server::test_support::transport::in_progress(world, unit) {
        world.entity_mut(unit).remove::<(Travel, MoveIntent)>();
        crate::creature_ai::publish_planned_destination(world, unit, None);
        return Ok(());
    }
    if server_follower_apl::literal_rotation::melee_intent::forced_movement(world, unit) {
        world.entity_mut(unit).remove::<Travel>();
        return Ok(());
    }
    if !can_move(world, unit) {
        return Ok(());
    }
    let from = point(world.get::<Position>(unit).unwrap());
    let Some((to, reach)) = destination(world, unit, from) else {
        // Hazard escape or another driver took the unit's goal over.
        crate::creature_ai::publish_planned_destination(world, unit, None);
        return Ok(());
    };
    let in_reach = server::test_support::within_reach(from, to, reach);
    let mut travel = world.entity_mut(unit).take::<Travel>().filter(|travel| {
        !travel.legs.is_empty()
            && travel.destination.distance(to) <= server_combat::navigation::REPATH_DISTANCE
            && !in_reach
    });
    if travel.is_none() {
        let legs = if in_reach {
            Default::default()
        } else {
            let label = movement_label(world, unit);
            let walker = world
                .get_resource::<NativeMovement>()
                .ok_or("native movement map is not installed")?;
            match walker.walk_legs(from, to, label) {
                Ok(legs) => legs,
                Err(error) => return handle_unreachable_movement(world, unit, from, to, error),
            }
        };
        travel = Some(Travel {
            destination: to,
            legs,
        });
    }
    world
        .entity_mut(unit)
        .remove::<(crate::creature_ai::CannotReachTarget, BlockedWalk)>();
    let mut travel = travel.unwrap();
    let speed = read_speed(world, unit);
    let next =
        if let Some(SafeOwnerApproach(target)) = world.get::<SafeOwnerApproach>(unit).copied() {
            let mut proposed = travel.clone();
            let next = advance(from, &mut proposed, speed * dt, reach);
            if crate::followers::simulator::owner_step_enters_unpulled_sight(
                world, unit, target, from, next,
            )? {
                world.entity_mut(unit).insert(travel);
                return Ok(());
            }
            travel = proposed;
            next
        } else {
            advance(from, &mut travel, speed * dt, reach)
        };
    crate::creature_ai::move_follower(
        world,
        unit,
        Position {
            x: next.x,
            y: next.y,
            z: next.z,
        },
        speed,
    );
    let moved = world
        .get::<Position>(unit)
        .is_some_and(|position| point(position) != from);
    if let Some(mut follow) = world.get_mut::<crate::creature_ai::FollowOwner>(unit)
        && follow.owner != unit
    {
        follow.set_moving(moved);
    }
    if let Some(facing) = crate::creature_ai::facing_along(to.x - next.x, to.z - next.z) {
        world.entity_mut(unit).insert(Rotation {
            x: 0.0,
            y: facing,
            z: 0.0,
        });
    }
    let returned_home = travel.legs.is_empty()
        && world.get::<crate::creature_ai::CreatureState>(unit)
            == Some(&crate::creature_ai::CreatureState::Returning);
    if returned_home {
        if let Some(home) = world.get::<crate::creature_ai::HomePosition>(unit) {
            let facing = home.orientation;
            world.entity_mut(unit).insert(Rotation {
                x: 0.0,
                y: facing,
                z: 0.0,
            });
        }
        crate::creature_ai::finish_return_home(world, unit);
    } else if !in_reach {
        world.entity_mut(unit).insert(travel);
    }
    let goal = (!returned_home && !in_reach).then_some(to);
    crate::creature_ai::publish_planned_destination(world, unit, goal);
    Ok(())
}

fn advance_flight(world: &mut World, owner: Entity, dt: f32) -> Result<(), String> {
    if !world
        .get::<Health>(owner)
        .is_some_and(|health| health.current > 0.0)
    {
        return Ok(());
    }
    let mut flight = world.entity_mut(owner).take::<FlightTravel>().unwrap();
    match flight.phase {
        FlightPhase::Mounting => {
            if world.get::<shared::components::Mounted>(owner).is_some() {
                server::test_support::flight::enter_flight(world, owner)?;
                flight.phase = FlightPhase::Flying;
            } else if world.get::<shared::casting::CastState>(owner).is_none() {
                server::test_support::flight::request_mount(world, owner)?;
            }
        }
        FlightPhase::Flying => {
            if !can_move(world, owner) {
                world.entity_mut(owner).insert(flight);
                return Ok(());
            }
            server::test_support::flight::enter_flight(world, owner)?;
            let followers: Vec<_> = world
                .query::<(Entity, &crate::followers::Follower, &Health)>()
                .iter(world)
                .filter(|(_, follower, health)| follower.owner == owner && health.current > 0.0)
                .map(|(unit, _, _)| unit)
                .collect();
            if followers.iter().any(|unit| {
                world
                    .get::<crate::creature_ai::flight_follow::FlightFollow>(*unit)
                    .is_none()
            }) {
                return Err("a living follower did not enter native flight".into());
            }
            let from = point(
                world
                    .get::<Position>(owner)
                    .ok_or("flight leader lacks position")?,
            );
            let speed = shared::movement::FLIGHT_SPEED
                * world
                    .get::<crate::auras::AuraStatMods>(owner)
                    .map_or(1.0, |mods| mods.flight_speed);
            let next = advance(from, &mut flight.travel, speed * dt, 0.0);
            world
                .entity_mut(owner)
                .insert(shared::components::MovementSpeed(speed));
            crate::creature_ai::move_follower(
                world,
                owner,
                Position {
                    x: next.x,
                    y: next.y,
                    z: next.z,
                },
                speed,
            );
            if flight.travel.legs.is_empty() {
                server::test_support::flight::land(world, owner)?;
                flight.phase = FlightPhase::Landing;
            }
        }
        FlightPhase::Landing => {
            let flying = world
                .query::<(
                    &crate::followers::Follower,
                    &crate::creature_ai::flight_follow::FlightFollow,
                )>()
                .iter(world)
                .any(|(follower, _)| follower.owner == owner);
            if !flying {
                crate::creature_ai::publish_planned_destination(world, owner, None);
                return Ok(());
            }
        }
    }
    world.entity_mut(owner).insert(flight);
    Ok(())
}

#[test]
fn travel_updates_follower_motion_flag_on_movement_and_stop() {
    use crate::creature_ai::FollowOwner;
    let mut world = World::new();
    let walker = reach_tests::altar_walker();
    let (from, to) = reach_tests::short_walk(&walker);
    world.insert_resource(walker);
    let owner = world
        .spawn(Position {
            x: to.x,
            y: to.y,
            z: to.z,
        })
        .id();
    let follower = world
        .spawn((
            Position {
                x: from.x,
                y: from.y,
                z: from.z,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            FollowOwner::new(owner),
        ))
        .id();
    move_one(&mut world, follower, 0.5).unwrap();
    assert_ne!(point(world.get::<Position>(follower).unwrap()), from);
    assert!(world.get::<FollowOwner>(follower).unwrap().is_moving());
    let position = *world.get::<Position>(follower).unwrap();
    world.entity_mut(owner).insert(position);
    move_one(&mut world, follower, 0.5).unwrap();
    assert_eq!(*world.get::<Position>(follower).unwrap(), position);
    assert!(!world.get::<FollowOwner>(follower).unwrap().is_moving());
}

#[cfg(test)]
fn read_apl_distance(world: &World, actor: Entity) -> Result<f64, String> {
    use server::simc_apl::Facts;
    use server::simc_apl_native::{Bindings, Encounter, NativeFacts};
    NativeFacts::new(
        world,
        actor,
        &Bindings::default(),
        Encounter::dungeon(0.0),
        1,
    )
    .value("movement.distance")
}

#[test]
fn travel_publishes_its_goal_while_walking_and_clears_it_on_arrival() {
    use crate::creature_ai::FollowOwner;
    use server::simc_apl_native::MovementDestination;
    let mut world = World::new();
    let walker = reach_tests::altar_walker();
    let (from, to) = reach_tests::short_walk(&walker);
    world.insert_resource(walker);
    let owner = world
        .spawn(Position {
            x: to.x,
            y: to.y,
            z: to.z,
        })
        .id();
    let follower = world
        .spawn((
            Position {
                x: from.x,
                y: from.y,
                z: from.z,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            shared::components::CreatureMotion::Still,
            FollowOwner::new(owner),
        ))
        .id();
    assert_eq!(read_apl_distance(&world, follower).unwrap(), 0.0, "idle");
    move_one(&mut world, follower, 0.5).unwrap();
    assert_eq!(
        world.get::<MovementDestination>(follower),
        Some(&MovementDestination(to))
    );
    let here = point(world.get::<Position>(follower).unwrap());
    assert_eq!(
        read_apl_distance(&world, follower).unwrap(),
        f64::from(here.distance(to))
    );
    assert!(
        here.distance(to) > 4.0,
        "still walking toward the follow reach"
    );
    let position = *world.get::<Position>(follower).unwrap();
    world.entity_mut(owner).insert(position);
    move_one(&mut world, follower, 0.5).unwrap();
    assert!(world.get::<MovementDestination>(follower).is_none());
    assert_eq!(read_apl_distance(&world, follower).unwrap(), 0.0);
}

#[test]
fn flight_starts_only_at_takeoff_and_tracks_the_full_three_dimensional_route() {
    let mut world = World::new();
    let leader = world
        .spawn(Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        })
        .id();
    let path = [[1.0, 0.0, 0.0], [1.0, 3.0, 0.0], [5.0, 3.0, 0.0]];
    assert!(begin_flight(&mut world, leader, &path).is_err());
    assert!(!flight_in_progress(&world, leader));
    world.entity_mut(leader).insert(Position {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    begin_flight(&mut world, leader, &path).unwrap();
    assert!(flight_in_progress(&world, leader));
    let (goal, remaining) = remaining_route(&world, leader).unwrap();
    assert!(goal == RouteGoal::Point(Vec3::new(5.0, 3.0, 0.0)));
    assert_eq!(remaining, 7.0);
}

#[test]
fn travel_without_a_goal_drops_its_destination() {
    use crate::creature_ai::FollowOwner;
    use server::simc_apl_native::MovementDestination;
    let mut world = World::new();
    let walker = reach_tests::altar_walker();
    let (from, to) = reach_tests::short_walk(&walker);
    world.insert_resource(walker);
    let owner = world
        .spawn(Position {
            x: to.x,
            y: to.y,
            z: to.z,
        })
        .id();
    let follower = world
        .spawn((
            Position {
                x: from.x,
                y: from.y,
                z: from.z,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            shared::components::CreatureMotion::Still,
            FollowOwner::new(owner),
        ))
        .id();
    move_one(&mut world, follower, 0.5).unwrap();
    assert!(world.get::<MovementDestination>(follower).is_some());
    // Hazard escape removes the follow goal and moves through a navigation route.
    world.entity_mut(follower).remove::<FollowOwner>();
    move_one(&mut world, follower, 0.5).unwrap();
    assert!(world.get::<MovementDestination>(follower).is_none());
}

#[test]
fn flight_publishes_its_landing_point_until_the_party_has_landed() {
    use server::simc_apl_native::MovementDestination;
    use shared::components::CreatureMotion;
    let mut world = World::new();
    let leader = world
        .spawn((
            Position {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            CreatureMotion::Still,
        ))
        .id();
    assert_eq!(read_apl_distance(&world, leader).unwrap(), 0.0, "idle");
    let path = [[1.0, 0.0, 0.0], [1.0, 3.0, 0.0], [5.0, 3.0, 0.0]];
    begin_flight(&mut world, leader, &path).unwrap();
    let landing = Vec3::new(5.0, 3.0, 0.0);
    assert_eq!(
        world.get::<MovementDestination>(leader),
        Some(&MovementDestination(landing))
    );
    world.entity_mut(leader).insert(CreatureMotion::Run);
    assert_eq!(read_apl_distance(&world, leader).unwrap(), 5.0);
    world.get_mut::<FlightTravel>(leader).unwrap().phase = FlightPhase::Landing;
    world.entity_mut(leader).insert(CreatureMotion::Still);
    advance_flight(&mut world, leader, 0.1).unwrap();
    assert!(!flight_in_progress(&world, leader));
    assert!(world.get::<MovementDestination>(leader).is_none());
    assert_eq!(read_apl_distance(&world, leader).unwrap(), 0.0);
}

#[test]
fn dead_flight_leader_waits_for_native_release() {
    let mut world = World::new();
    let position = Position {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    };
    let leader = world
        .spawn((
            position,
            Health {
                current: 0.0,
                max: 100.0,
            },
            FlightTravel {
                phase: FlightPhase::Flying,
                travel: Travel {
                    destination: Vec3::new(10.0, 20.0, 30.0),
                    legs: [(Vec3::new(10.0, 20.0, 30.0), 30.0)].into(),
                },
            },
        ))
        .id();
    advance_flight(&mut world, leader, 1.0).unwrap();
    assert_eq!(*world.get::<Position>(leader).unwrap(), position);
    assert!(flight_in_progress(&world, leader));
}

#[test]
fn pursuit_arrives_inside_three_dimensional_melee_reach() {
    let target = Vec3::ZERO;
    let corner = Vec3::new(0.0, 0.2, 0.0);
    let mut travel = Travel {
        destination: target,
        legs: [(corner, 10.0)].into(),
    };
    let position = advance(Vec3::new(10.0, 0.2, 0.0), &mut travel, 10.0, 5.0);
    assert!(
        position.distance(target) <= 5.0,
        "stopped outside melee: {position:?}"
    );
}

#[test]
fn frozen_edge_time_is_not_straight_line_distance() {
    let to = Vec3::new(10.0, 0.0, 0.0);
    let mut travel = Travel {
        destination: to,
        legs: [(to, 100.0)].into(),
    };
    let position = advance(Vec3::ZERO, &mut travel, 7.0, 0.0);
    assert!((position.x - 0.7).abs() < 0.0001);
    let position = advance(position, &mut travel, 93.0, 0.0);
    assert_eq!(position, to);
}

pub(super) fn update(world: &mut World) {
    let dt = world.resource::<Time>().delta_secs();
    let units: Vec<_> = world
        .query::<(Entity, &Position, &Health)>()
        .iter(world)
        .map(|(unit, _, _)| unit)
        .collect();
    for unit in units {
        if let Err(error) = move_one(world, unit, dt) {
            let ledger = world
                .get::<server::simc_apl::FactErrors>(unit)
                .cloned()
                .unwrap_or_default();
            ledger.record("movement.native", unit.to_bits(), None, &error);
            if let Ok(mut entity) = world.get_entity_mut(unit) {
                entity.remove::<(Travel, MoveIntent)>();
                entity.insert((ledger, BlockedWalk { reason: error }));
            }
            crate::creature_ai::publish_planned_destination(world, unit, None);
        }
    }
}
