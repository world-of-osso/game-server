//! Approved bridge: leave active hostile bound fields using complete baked routes.
use crate::maps::InMap;
use crate::navigation::{Goal, MoveRequest, Navigation, Step};
use ::navigation::worker::{Output, Query as NavQuery};
use bevy::prelude::*;
use shared::components::Position;

#[derive(Clone, Copy)]
struct Field {
    entity: Entity,
    center: Vec3,
    radius: f32,
}

#[derive(Component)]
struct EscapeProbe {
    map: u32,
    goal: Goal,
    targets: Vec<Vec3>,
}

fn clear_escape_probe(world: &mut World, unit: Entity) {
    if let Some(probe) = world.entity_mut(unit).take::<EscapeProbe>()
        && let Some(mut nav) = world.get_resource_mut::<Navigation>()
    {
        nav.cancel_query(unit, probe.goal);
    }
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

pub(super) fn approach_is_unsafe(world: &mut World, unit: Entity, target: Entity) -> Option<bool> {
    if target_is_hazardous(world, unit, target) {
        return Some(true);
    }
    path_crosses_hostile_field(world, unit, target)
}

fn path_crosses_hostile_field(world: &mut World, unit: Entity, target: Entity) -> Option<bool> {
    let fields = hostile_fields(world, unit);
    if fields.is_empty() {
        if let Some(mut nav) = world.get_resource_mut::<Navigation>() {
            nav.cancel_query(unit, Goal::Approach(target.to_bits()));
        }
        return Some(false);
    }
    let (Some(from), Some(to), Some(map)) = (
        world.get::<Position>(unit).copied(),
        world.get::<Position>(target).copied(),
        world.get::<InMap>(unit).copied(),
    ) else {
        return Some(false);
    };
    let Some(mut nav) = world.get_resource_mut::<Navigation>() else {
        return Some(false);
    };
    let result = nav.query_deferred(
        unit,
        map.map_id,
        Goal::Approach(target.to_bits()),
        NavQuery::Path {
            from: point(from),
            to: point(to),
        },
    )?;
    if result.is_err() {
        nav.cancel_query(unit, Goal::Approach(target.to_bits()));
    }
    Some(match result {
        Ok(Some(Output::Path(mut path))) => {
            path.insert(0, point(from));
            !route_leaves_fields(&path, &fields)
        }
        Ok(None) => false,
        Ok(Some(_)) => {
            error!("Follower {unit} field-aware approach returned a non-path result");
            false
        }
        Err(error) => {
            debug!("Follower {unit} field-aware approach query: {error}");
            false
        }
    })
}

pub(super) fn escape(world: &mut World, unit: Entity) -> bool {
    let Some(from) = world.get::<Position>(unit).copied() else {
        return false;
    };
    let fields = hostile_fields(world, unit);
    let Some(field) = fields.iter().find(|field| contains(field, point(from))) else {
        clear_escape_probe(world, unit);
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
    let to = match escape_destination(world, unit, map.map_id, from, goal, field, &fields) {
        EscapeDestination::Pending => {
            advance_escape(world, unit, map.map_id, from, None, goal);
            return true;
        }
        EscapeDestination::Ready(to) => to,
        EscapeDestination::Blocked => {
            warn!(
                "Follower {unit} cannot leave bound field {} through authored navigation",
                field.entity
            );
            return true;
        }
    };
    advance_escape(world, unit, map.map_id, from, Some(position(to)), goal);
    true
}

enum EscapeDestination {
    Pending,
    Ready(Vec3),
    Blocked,
}

fn escape_destination(
    world: &mut World,
    unit: Entity,
    map: u32,
    from: Position,
    goal: Goal,
    field: &Field,
    fields: &[Field],
) -> EscapeDestination {
    let Some(nav) = world.get_resource::<Navigation>() else {
        return EscapeDestination::Blocked;
    };
    if let Some(to) = nav
        .route_destination(unit, goal)
        .filter(|to| fields.iter().all(|field| !contains(field, *to)))
    {
        return EscapeDestination::Ready(to);
    }
    // A growing native field must not replace its batch every tick. Capture the
    // decision once, then validate its returned endpoints against current fields.
    let targets = if let Some(probe) = world.get::<EscapeProbe>(unit)
        && probe.map == map
        && probe.goal == goal
    {
        probe.targets.clone()
    } else {
        let targets = sample_escape_targets(world, map, from, field, fields);
        world.entity_mut(unit).insert(EscapeProbe {
            map,
            goal,
            targets: targets.clone(),
        });
        targets
    };
    let result = world.resource_mut::<Navigation>().query_deferred(
        unit,
        map,
        goal,
        NavQuery::Candidates {
            from: point(from),
            targets: targets.clone(),
        },
    );
    let Some(result) = result else {
        return EscapeDestination::Pending;
    };
    let routes = match result {
        Ok(Some(Output::Candidates(routes))) => routes,
        Ok(None) => return blocked_escape(world, unit),
        Ok(Some(_)) => {
            error!("Follower {unit} escape batch returned a non-candidate result");
            return blocked_escape(world, unit);
        }
        Err(error) => {
            debug!("Follower {unit} escape candidates: {error}");
            return blocked_escape(world, unit);
        }
    };
    let destination = choose_escape_route(unit, targets, routes, fields);
    if matches!(destination, EscapeDestination::Blocked) {
        clear_escape_probe(world, unit);
    }
    destination
}

fn blocked_escape(world: &mut World, unit: Entity) -> EscapeDestination {
    clear_escape_probe(world, unit);
    EscapeDestination::Blocked
}

fn sample_escape_targets(
    world: &mut World,
    map: u32,
    from: Position,
    field: &Field,
    fields: &[Field],
) -> Vec<Vec3> {
    let radius = field.radius + shared::los::body::BODY_RADIUS * 2.0;
    (0..8)
        .filter_map(|index| {
            let angle = index as f32 * std::f32::consts::FRAC_PI_4;
            let mut to = field.center + Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
            to.y = from.y;
            supported_candidate(world, map, to, fields)
        })
        .collect()
}

fn choose_escape_route(
    unit: Entity,
    targets: Vec<Vec3>,
    routes: Vec<Result<Vec<Vec3>, String>>,
    fields: &[Field],
) -> EscapeDestination {
    let mut best: Option<(f32, Vec3)> = None;
    for (to, route) in targets.into_iter().zip(routes) {
        let route = match route {
            Ok(route) => route,
            Err(error) => {
                debug!("Follower {unit} escape candidate {to:?}: {error}");
                continue;
            }
        };
        if fields.iter().any(|field| contains(field, to)) || !route_leaves_fields(&route, fields) {
            continue;
        }
        let length = route.windows(2).map(|pair| pair[0].distance(pair[1])).sum();
        if best.is_none_or(|(previous, _)| length < previous) {
            best = Some((length, to));
        }
    }
    best.map_or(EscapeDestination::Blocked, |(_, to)| {
        EscapeDestination::Ready(to)
    })
}

fn supported_candidate(
    world: &mut World,
    map: u32,
    mut to: Vec3,
    fields: &[Field],
) -> Option<Vec3> {
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
    Some(to)
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
    to: Option<Position>,
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
    let mut nav = world.resource_mut::<Navigation>();
    let step = if let Some(to) = to {
        nav.advance(
            unit,
            MoveRequest {
                map_id: map,
                goal,
                from,
                to,
                distance,
                reach: 0.0,
            },
        )
    } else {
        nav.continue_route(unit, map, from, distance)
    };
    drop(nav);
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
