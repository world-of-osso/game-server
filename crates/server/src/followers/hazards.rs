//! Approved bridge: leave active hostile bound fields using complete baked routes.
use crate::maps::InMap;
use crate::navigation::{Goal, MoveRequest, Navigation, Step};
use bevy::prelude::*;
use shared::components::Position;

#[derive(Clone, Copy)]
struct Field {
    entity: Entity,
    center: Vec3,
    radius: f32,
}

fn point(position: Position) -> Vec3 {
    Vec3::new(position.x, position.y, position.z)
}
fn position(point: Vec3) -> Position {
    Position {
        x: point.x,
        y: point.y,
        z: point.z,
    }
}
fn contains(field: &Field, point: Vec3) -> bool {
    field.center.distance(point) <= field.radius
}

fn hostile_fields(world: &mut World, unit: Entity) -> Vec<Field> {
    let mut fields: Vec<_> = crate::spell_cast::scripts::hostile_ground_fields(world, unit)
        .into_iter()
        .map(|(entity, center, radius)| Field {
            entity,
            center,
            radius,
        })
        .collect();
    let map = world.get::<InMap>(unit).copied();
    let now = crate::auras::aura_now(world);
    fields.extend(
        world
            .query::<(
                Entity,
                &crate::spell_cast::area_trigger::SpellAreaTrigger,
                &Position,
                &InMap,
            )>()
            .iter(world)
            .filter(|(_, field, _, field_map)| {
                field.trigger_spell != 0
                    && field.radius > 0.0
                    && field.relation == crate::spell_cast::targets::Relation::Enemy
                    && field.expires_at.is_none_or(|expiry| expiry > now)
                    && Some(**field_map) == map
                    && crate::spell_cast::targets::relation_matches(
                        world,
                        field.caster,
                        unit,
                        field.relation,
                    )
            })
            .filter_map(|(entity, _, anchor, _)| {
                let Some(radius) =
                    crate::spell_cast::area_trigger::avoidance_radius(world, entity, unit)
                else {
                    warn!(
                        ?unit,
                        ?entity,
                        "Follower native field avoidance requires source combat reach"
                    );
                    return None;
                };
                Some(Field {
                    entity,
                    center: point(*anchor),
                    radius,
                })
            }),
    );
    fields
}

pub(super) fn target_is_hazardous(world: &mut World, unit: Entity, target: Entity) -> bool {
    let Some(at) = world.get::<Position>(target).copied() else {
        return false;
    };
    hostile_fields(world, unit)
        .iter()
        .any(|field| contains(field, point(at)))
}

pub(super) fn path_crosses_hostile_field(world: &mut World, unit: Entity, target: Entity) -> bool {
    let fields = hostile_fields(world, unit);
    if fields.is_empty() {
        return false;
    }
    let (Some(from), Some(to), Some(map)) = (
        world.get::<Position>(unit).copied(),
        world.get::<Position>(target).copied(),
        world.get::<InMap>(unit).copied(),
    ) else {
        return false;
    };
    let Some(nav) = world.get_resource::<Navigation>() else {
        return false;
    };
    match nav.query_path(map.map_id, point(from), point(to)) {
        Ok(Some(mut path)) => {
            path.insert(0, point(from));
            !route_leaves_fields(&path, &fields)
        }
        Ok(None) => false,
        Err(error) => {
            debug!("Follower {unit} field-aware approach query: {error}");
            false
        }
    }
}

pub(super) fn escape(world: &mut World, unit: Entity) -> bool {
    let Some(from) = world.get::<Position>(unit).copied() else {
        return false;
    };
    let fields = hostile_fields(world, unit);
    let Some(field) = fields.iter().find(|field| contains(field, point(from))) else {
        return false;
    };
    crate::spell_cast::cancel_cast(world, unit);
    world
        .entity_mut(unit)
        .remove::<crate::creature_ai::FollowOwner>()
        .insert(crate::creature_ai::CreatureState::Idle);
    let Some(map) = world.get::<InMap>(unit).copied() else {
        error!("Follower {unit} hazard escape lacks map");
        return true;
    };
    let goal = Goal::Hazard(field.entity.to_bits());
    let Some(to) = escape_destination(world, unit, map.map_id, from, goal, field, &fields) else {
        warn!(
            "Follower {unit} cannot leave bound field {} through authored navigation",
            field.entity
        );
        return true;
    };
    advance_escape(world, unit, map.map_id, from, position(to), goal);
    true
}

fn escape_destination(
    world: &mut World,
    unit: Entity,
    map: u32,
    from: Position,
    goal: Goal,
    field: &Field,
    fields: &[Field],
) -> Option<Vec3> {
    let nav = world.get_resource::<Navigation>()?;
    if let Some(to) = nav
        .route_destination(unit, goal)
        .filter(|to| fields.iter().all(|field| !contains(field, *to)))
    {
        return Some(to);
    }
    let radius = field.radius + shared::los::body::BODY_RADIUS * 2.0;
    let mut best: Option<(f32, Vec3)> = None;
    // Eight compass candidates are movement decisions, not authored spawn coordinates.
    // Every candidate must have real support and a complete baked corridor.
    for index in 0..8 {
        let angle = index as f32 * std::f32::consts::FRAC_PI_4;
        let mut to = field.center + Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
        to.y = from.y;
        let Some((length, to)) = query_escape_candidate(world, unit, map, from, to, fields) else {
            continue;
        };
        if best.is_none_or(|(previous, _)| length < previous) {
            best = Some((length, to));
        }
    }
    best.map(|(_, to)| to)
}

fn query_escape_candidate(
    world: &mut World,
    unit: Entity,
    map: u32,
    from: Position,
    mut to: Vec3,
    fields: &[Field],
) -> Option<(f32, Vec3)> {
    let crate::ground::GroundProbe::Supported(height) = world
        .get_resource_mut::<crate::ground::GroundMap>()?
        .ground_at(map, to)
    else {
        return None;
    };
    to.y = height;
    if fields.iter().any(|field| contains(field, to)) {
        return None;
    }
    let route = match world
        .resource::<Navigation>()
        .query_path(map, point(from), to)
    {
        Ok(Some(route)) => route,
        Ok(None) => return None,
        Err(error) => {
            debug!("Follower {unit} escape candidate {to:?}: {error}");
            return None;
        }
    };
    if !route_leaves_fields(&route, fields) {
        return None;
    }
    let length = route.windows(2).map(|pair| pair[0].distance(pair[1])).sum();
    Some((length, to))
}

fn route_leaves_fields(route: &[Vec3], fields: &[Field]) -> bool {
    fields.iter().all(|field| {
        route.windows(2).all(|pair| {
            if contains(field, pair[0]) {
                return true;
            }
            let delta = pair[1] - pair[0];
            let fraction = if delta.length_squared() > 0.0 {
                ((field.center - pair[0]).dot(delta) / delta.length_squared()).clamp(0.0, 1.0)
            } else {
                0.0
            };
            !contains(field, pair[0] + delta * fraction)
        })
    })
}

fn advance_escape(
    world: &mut World,
    unit: Entity,
    map: u32,
    from: Position,
    to: Position,
    goal: Goal,
) {
    let scale = world
        .get::<crate::auras::AuraStatMods>(unit)
        .map_or(1.0, |mods| mods.movement_speed);
    let base = world
        .get::<crate::creature_ai::SpawnPoint>(unit)
        .map_or(1.0, |spawn| spawn.speed_run);
    let speed = shared::movement::RUN_SPEED * base * scale;
    let distance = speed * world.resource::<Time>().delta_secs();
    if distance <= 0.0 {
        return;
    }
    let step = world.resource_mut::<Navigation>().advance(
        unit,
        MoveRequest {
            map_id: map,
            goal,
            from,
            to,
            distance,
            reach: 0.0,
        },
    );
    if let Step::Moved { mut position, .. } = step {
        let crate::ground::GroundProbe::Supported(height) = world
            .resource_mut::<crate::ground::GroundMap>()
            .ground_at(map, point(position))
        else {
            error!("Follower {unit} escape route lost ground support");
            return;
        };
        position.y = height;
        crate::creature_ai::move_follower(world, unit, position, speed);
    }
}
