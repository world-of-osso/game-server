//! Time along frozen edges replaces physical navigation, for every simulated mover.
use super::{Space, campaign::MoveIntent};
use bevy::prelude::*;
use shared::components::{Health, Position, Rotation};

#[derive(Component)]
struct Travel {
    destination: Vec3,
    legs: std::collections::VecDeque<(Vec3, f32)>,
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
        .filter_map(|unit| crate::hunter_pet::summoned_pet(world, *unit))
        .collect();
    units.extend(pets);
    for unit in units {
        world.entity_mut(unit).remove::<Travel>();
    }
}

fn point(position: &Position) -> Vec3 {
    Vec3::new(position.x, position.y, position.z)
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
        let reach = world
            .get::<crate::followers::FollowerReach>(unit)
            .map_or(crate::creature_ai::CHASE_REACH, |reach| reach.0);
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

fn read_speed(world: &World, unit: Entity) -> f32 {
    world
        .get::<crate::creature_ai::SpawnPoint>(unit)
        .map_or(1.0, |spawn| spawn.speed_run)
        * crate::creature_ai::BASE_RUN_SPEED
        * world
            .get::<crate::auras::AuraStatMods>(unit)
            .map_or(1.0, |mods| mods.movement_speed)
}

fn can_move(world: &World, unit: Entity) -> bool {
    world
        .get::<Health>(unit)
        .is_some_and(|health| health.current > 0.0)
        && world
            .get::<shared::casting::CastState>(unit)
            .is_none_or(|cast| cast.cast_while_moving)
        && world.get::<crate::auras::Confused>(unit).is_none()
        && world.get::<crate::auras::Feared>(unit).is_none()
}

fn read_legs(
    space: &Space,
    from: Vec3,
    destination: Vec3,
) -> Result<std::collections::VecDeque<(Vec3, f32)>, String> {
    let points = space.route(from, destination)?;
    let mut previous = from;
    points
        .into_iter()
        .map(|next| {
            let nearest = |point: Vec3| {
                space
                    .0
                    .packs
                    .iter()
                    .min_by(|a, b| {
                        let distance =
                            |pack: &super::graph::Pack| pack.point().distance_squared(point);
                        distance(a).total_cmp(&distance(b))
                    })
                    .unwrap()
                    .id
            };
            let a = nearest(previous);
            let b = nearest(next);
            let yards = if a == b {
                previous.distance(next)
            } else {
                space
                    .0
                    .edges
                    .iter()
                    .find(|edge| {
                        (edge.from == a && edge.to == b) || (edge.from == b && edge.to == a)
                    })
                    .ok_or_else(|| format!("route leg pack{a}->pack{b} lacks a frozen edge"))?
                    .yards
            };
            previous = next;
            Ok((next, yards))
        })
        .collect()
}

fn advance(mut from: Vec3, travel: &mut Travel, mut yards: f32, reach: f32) -> Vec3 {
    while !travel.legs.is_empty() {
        let last = travel.legs.len() == 1;
        let (to, remaining) = travel.legs.front_mut().unwrap();
        let available = if last {
            (*remaining - reach).max(0.0)
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

fn move_one(world: &mut World, unit: Entity, dt: f32) -> Result<(), String> {
    if !can_move(world, unit) {
        return Ok(());
    }
    let from = point(world.get::<Position>(unit).unwrap());
    let Some((to, reach)) = destination(world, unit, from) else {
        return Ok(());
    };
    let mut travel = world
        .entity_mut(unit)
        .take::<Travel>()
        .filter(|travel| travel.destination == to);
    if travel.is_none() {
        travel = Some(Travel {
            destination: to,
            legs: read_legs(world.resource::<Space>(), from, to)?,
        });
    }
    let mut travel = travel.unwrap();
    let speed = read_speed(world, unit);
    let next = advance(from, &mut travel, speed * dt, reach);
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
    } else {
        world.entity_mut(unit).insert(travel);
    }
    Ok(())
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
        move_one(world, unit, dt)
            .unwrap_or_else(|error| panic!("simulated movement {unit:?}: {error}"));
    }
}
