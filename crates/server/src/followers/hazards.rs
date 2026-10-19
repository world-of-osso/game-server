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

// FOLLOWER_HAZARD_REACTION_POLICY: project policy, not sourced Retail timing.
const PREDICTION_SECONDS: f64 = 0.050;
const REACTION_SECONDS: f64 = 0.250;

#[derive(Component)]
struct EscapeProbe {
    map: u32,
    goal: Goal,
    detected_at: f64,
    targets: Option<Vec<Vec3>>,
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
    hostile_fields_at(world, unit, 0.0)
}

fn hostile_fields_at(world: &mut World, unit: Entity, prediction: f64) -> Vec<Field> {
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
    let at = now + prediction;
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
                    && field.expires_at.is_none_or(|expiry| expiry > at)
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
                    crate::spell_cast::area_trigger::avoidance_radius_at(world, entity, unit, at)
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
    let Some(to) = world.get::<Position>(target).copied() else {
        return Some(false);
    };
    read_route_hazard(world, unit, to, Goal::Approach(target.to_bits()))
}

pub(super) fn point_route_crosses_hostile_field(
    world: &mut World,
    unit: Entity,
    to: Position,
) -> Option<bool> {
    read_route_hazard(world, unit, to, Goal::Point)
}

fn read_route_hazard(world: &mut World, unit: Entity, to: Position, goal: Goal) -> Option<bool> {
    let fields = hostile_fields(world, unit);
    if fields.is_empty() {
        if let Some(mut nav) = world.get_resource_mut::<Navigation>() {
            nav.cancel_query(unit, goal);
        }
        return Some(false);
    }
    let (Some(from), Some(map)) = (
        world.get::<Position>(unit).copied(),
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
        goal,
        NavQuery::Path {
            from: point(from),
            to: point(to),
        },
    )?;
    if result.is_err() {
        nav.cancel_query(unit, goal);
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
    let fields = hostile_fields_at(world, unit, PREDICTION_SECONDS);
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
    let now = crate::auras::aura_now(world);
    let same_detection = world
        .get::<EscapeProbe>(unit)
        .is_some_and(|probe| probe.map == map.map_id && probe.goal == goal);
    if !same_detection {
        clear_escape_probe(world, unit);
        world.entity_mut(unit).insert(EscapeProbe {
            map: map.map_id,
            goal,
            detected_at: now,
            targets: None,
        });
    }
    // Submit immediately; preparation/publication may finish during human reaction.
    let destination = escape_destination(world, unit, map.map_id, from, goal, field, &fields);
    let detected_at = world
        .get::<EscapeProbe>(unit)
        .expect("active detection")
        .detected_at;
    if now < detected_at + REACTION_SECONDS {
        crate::creature_ai::move_follower(world, unit, from, 0.0);
        return true;
    }
    match destination {
        EscapeDestination::Ready(to) => {
            advance_escape(world, unit, map.map_id, from, position(to), goal)
        }
        EscapeDestination::Pending => crate::creature_ai::move_follower(world, unit, from, 0.0),
        EscapeDestination::Blocked => {
            warn!(
                "Follower {unit} cannot leave bound field {} through authored navigation",
                field.entity
            );
            crate::creature_ai::move_follower(world, unit, from, 0.0);
        }
    }
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
    let targets = if let Some(targets) = world
        .get::<EscapeProbe>(unit)
        .and_then(|probe| probe.targets.clone())
    {
        targets
    } else {
        let targets = sample_escape_targets(world, map, from, field, fields);
        world
            .get_mut::<EscapeProbe>(unit)
            .expect("active detection")
            .targets = Some(targets.clone());
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
    let Some((to, path)) = choose_escape_route(unit, targets, routes, fields) else {
        return blocked_escape(world, unit);
    };
    world.resource_mut::<Navigation>().adopt_candidate_route(
        unit,
        MoveRequest {
            map_id: map,
            goal,
            from,
            to: position(to),
            distance: 0.0,
            reach: 0.0,
        },
        path,
    );
    EscapeDestination::Ready(to)
}

fn blocked_escape(world: &mut World, unit: Entity) -> EscapeDestination {
    if let Some(mut probe) = world.get_mut::<EscapeProbe>(unit) {
        probe.targets = None;
        let goal = probe.goal;
        if let Some(mut nav) = world.get_resource_mut::<Navigation>() {
            nav.cancel_query(unit, goal);
        }
    }
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
) -> Option<(Vec3, Vec<Vec3>)> {
    let mut best: Option<(f32, Vec3, Vec<Vec3>)> = None;
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
        if best
            .as_ref()
            .is_none_or(|(previous, _, _)| length < *previous)
        {
            best = Some((length, to, route));
        }
    }
    best.map(|(_, to, path)| (to, path))
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

pub(super) fn route_crosses_hostile_field(world: &mut World, unit: Entity, route: &[Vec3]) -> bool {
    !route_leaves_fields(route, &hostile_fields(world, unit))
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

pub(super) fn advance_grounded_route(
    world: &mut World,
    unit: Entity,
    map: InMap,
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
            map_id: map.map_id,
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
            .creature_ground_in(map, point(position))
        else {
            error!("Follower {unit} escape route lost ground support");
            return;
        };
        position.y = height;
        crate::creature_ai::move_follower(world, unit, position, speed);
    }
}
