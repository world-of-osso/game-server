//! TC a352b1fa FleeingMovementGenerator.cpp:26-27,66-89,118-198:
//! quiet distance28..43, surface-collision destination, LOS200ms/path100ms retry,
//! navmesh-only path≤30yd, running then800..1500ms pause. Owner control is lost.
use super::{AuraStatMods, Feared};
use crate::random as rand;
use crate::{
    maps::InMap,
    navigation::{Corners, Goal, Navigation},
};
use ::navigation::worker::{Output, Query as NavQuery};
use bevy::prelude::*;
use rand::Rng;
use shared::components::{
    CreatureMotion, Health, MovementControl, MovementSpeed, Player, PlayerMotion, Position,
    Rotation,
};

const MIN_QUIET_DISTANCE: f32 = 28.0;
const MAX_QUIET_DISTANCE: f32 = 43.0;
const MAX_PATH_LENGTH: f32 = 30.0;
const PATH_RETRY: f32 = 0.1;
const LOS_RETRY: f32 = 0.2;
const RUN_SPEED: f32 = 7.0;
const MIN_PAUSE: f32 = 0.8;
const MAX_PAUSE: f32 = 1.5;

#[derive(Component)]
pub(super) struct Fleeing {
    path: Option<Corners>,
    destination: Vec3,
    wait: f32,
    epoch: Option<u32>,
    query: Option<FleeQuery>,
}

#[derive(Clone, Copy)]
enum FleeQuery {
    Collision {
        map: InMap,
        from: Vec3,
        wanted: Vec3,
    },
    Path {
        map: InMap,
        from: Vec3,
        destination: Vec3,
    },
}

pub(super) fn start(world: &mut World, unit: Entity) {
    cancel_queries(world, unit);
    crate::knockback::cancel(world, unit);
    let epoch = if world.get::<Player>(unit).is_some() {
        let mut control = world
            .get::<MovementControl>(unit)
            .copied()
            .unwrap_or_default();
        control.epoch = control.epoch.wrapping_add(1);
        control.controlled = true;
        world.entity_mut(unit).insert(control);
        Some(control.epoch)
    } else {
        None
    };
    world.entity_mut(unit).insert(Fleeing {
        path: None,
        destination: Vec3::ZERO,
        wait: 0.0,
        epoch,
        query: None,
    });
    stop_motion(world, unit);
}

pub(super) fn stop(world: &mut World, unit: Entity) {
    cancel_queries(world, unit);
    let epoch = world.get::<Fleeing>(unit).and_then(|state| state.epoch);
    world.entity_mut(unit).remove::<Fleeing>();
    if let Some(epoch) = epoch {
        let mut control = world
            .get_mut::<MovementControl>(unit)
            .expect("fleeing player control");
        if control.epoch != epoch {
            return;
        }
        control.controlled = false;
        control.epoch = control.epoch.wrapping_add(1);
    }
    stop_motion(world, unit);
}

fn stop_motion(world: &mut World, unit: Entity) {
    if let Some(mut speed) = world.get_mut::<MovementSpeed>(unit) {
        speed.0 = 0.0;
    }
    if let Some(mut motion) = world.get_mut::<CreatureMotion>(unit) {
        *motion = CreatureMotion::Still;
    }
    if let Some(mut motion) = world.get_mut::<PlayerMotion>(unit) {
        motion.0 = 0;
    }
    crate::spell_cast::mark_dirty(world, unit);
}

pub(super) fn update(world: &mut World) {
    let dt = world.resource::<Time>().delta_secs();
    let units: Vec<_> = world
        .query_filtered::<Entity, With<Feared>>()
        .iter(world)
        .collect();
    for unit in units {
        advance(world, unit, dt);
    }
}

fn advance(world: &mut World, unit: Entity, dt: f32) {
    let Some(mut state) = world.entity_mut(unit).take::<Fleeing>() else {
        return;
    };
    if state.epoch.is_some_and(|epoch| {
        world
            .get::<MovementControl>(unit)
            .is_none_or(|control| control.epoch != epoch || !control.controlled)
    }) {
        cancel_queries(world, unit);
        state.query = None;
        world.entity_mut(unit).insert(state);
        return;
    }
    let speed = run_speed(world, unit);
    let blocked = world
        .get::<Health>(unit)
        .is_some_and(|hp| hp.current <= 0.0)
        || speed == 0.0
        || world
            .get::<shared::casting::CastState>(unit)
            .is_some_and(|cast| !cast.cast_while_moving);
    if blocked {
        cancel_queries(world, unit);
        state.query = None;
        state.path = None;
        stop_motion(world, unit);
    } else {
        state.wait -= dt;
        if state.path.is_none() && state.wait <= 0.0 {
            choose_path(world, unit, &mut state);
        }
        advance_path(world, unit, &mut state, speed, dt);
    }
    world.entity_mut(unit).insert(state);
}

fn advance_path(world: &mut World, unit: Entity, state: &mut Fleeing, speed: f32, dt: f32) {
    let Some(path) = &mut state.path else {
        return;
    };
    let before = point(*world.get::<Position>(unit).expect("fleeing position"));
    let (after, reached) = path.advance(before, speed * dt, 0.0, state.destination);
    move_unit(world, unit, before, after, speed);
    if reached {
        state.path = None;
        state.wait = rand::thread_rng().gen_range(MIN_PAUSE..=MAX_PAUSE);
        stop_motion(world, unit);
    }
}

fn run_speed(world: &World, unit: Entity) -> f32 {
    let base = world
        .get::<crate::creature_ai::SpawnPoint>(unit)
        .map_or(RUN_SPEED, |spawn| RUN_SPEED * spawn.speed_run);
    base * world
        .get::<AuraStatMods>(unit)
        .map_or(1.0, |mods| mods.movement_speed)
}

/// Absolute caster-relative direction, as required by the lead's away-from-caster
/// contract; Bevy x/z replaces native x/y, with no extra owner-facing rotation.
fn flee_point(from: Vec3, caster: Option<Vec3>, distance: f32, rng: &mut impl Rng) -> Vec3 {
    use std::f32::consts::{PI, TAU};
    let offset = caster.map(|caster| from - caster);
    let angle = match offset {
        Some(delta) if distance > 0.2 => delta.z.atan2(delta.x),
        _ => rng.gen_range(0.0..TAU),
    };
    let (distance, angle) = if distance < MIN_QUIET_DISTANCE {
        (
            rng.gen_range(0.4..=1.3) * (MIN_QUIET_DISTANCE - distance),
            angle + rng.gen_range(-PI / 8.0..=PI / 8.0),
        )
    } else if distance > MAX_QUIET_DISTANCE {
        (
            rng.gen_range(0.4..=1.0) * (MAX_QUIET_DISTANCE - MIN_QUIET_DISTANCE),
            -angle + rng.gen_range(-PI / 4.0..=PI / 4.0),
        )
    } else {
        (
            rng.gen_range(0.6..=1.2) * (MAX_QUIET_DISTANCE - MIN_QUIET_DISTANCE),
            rng.gen_range(0.0..TAU),
        )
    };
    from + Vec3::new(angle.cos() * distance, 0.0, angle.sin() * distance)
}

fn choose_path(world: &mut World, unit: Entity, state: &mut Fleeing) {
    if state.query.is_none() {
        state.wait = PATH_RETRY;
        state.query = capture_query(world, unit);
    }
    let Some(query) = state.query else {
        return;
    };
    let (map, from, goal, request) = match query {
        FleeQuery::Collision { map, from, wanted } => (
            map,
            from,
            Goal::FearCollision,
            NavQuery::Collision { from, to: wanted },
        ),
        FleeQuery::Path {
            map,
            from,
            destination,
        } => (
            map,
            from,
            Goal::FearPath,
            NavQuery::Path {
                from,
                to: destination,
            },
        ),
    };
    let at = point(*world.get::<Position>(unit).expect("fleeing position"));
    if world.get::<InMap>(unit) != Some(&map) || at != from {
        retry_path(world, unit, state, PATH_RETRY);
        return;
    }
    let Some(mut nav) = world.get_resource_mut::<Navigation>() else {
        retry_path(world, unit, state, PATH_RETRY);
        return;
    };
    state.wait = 0.0;
    let Some(result) = nav.query_deferred(unit, map.map_id, goal, request) else {
        return;
    };
    drop(nav);
    match (query, result) {
        (FleeQuery::Collision { .. }, Ok(Some(Output::Collision(destination)))) => {
            continue_collision(world, unit, state, map, from, destination)
        }
        (FleeQuery::Path { destination, .. }, Ok(Some(Output::Path(points)))) => {
            retry_path(world, unit, state, PATH_RETRY);
            start_path(destination, points, state);
        }
        (_, result) => {
            if let Err(error) = result {
                debug!("Fear {unit} {goal:?}: {error}");
            } else if result.is_ok_and(|output| output.is_some()) {
                error!("Fear {unit} {goal:?} returned an unexpected query result");
            }
            retry_path(world, unit, state, PATH_RETRY);
        }
    }
}

fn capture_query(world: &World, unit: Entity) -> Option<FleeQuery> {
    let map = *world.get::<InMap>(unit)?;
    let from = point(*world.get::<Position>(unit)?);
    let (caster, distance) = caster_geometry(world, unit, map)?;
    let wanted = flee_point(from, caster, distance, &mut rand::thread_rng());
    Some(FleeQuery::Collision { map, from, wanted })
}

fn cancel_queries(world: &mut World, unit: Entity) {
    if let Some(mut nav) = world.get_resource_mut::<Navigation>() {
        nav.cancel_query(unit, Goal::FearCollision);
        nav.cancel_query(unit, Goal::FearPath);
    }
}

fn retry_path(world: &mut World, unit: Entity, state: &mut Fleeing, wait: f32) {
    cancel_queries(world, unit);
    state.query = None;
    state.wait = wait;
}

fn continue_collision(
    world: &mut World,
    unit: Entity,
    state: &mut Fleeing,
    map: InMap,
    from: Vec3,
    destination: Vec3,
) {
    let destination = clip_collision(world, map, from, destination);
    if let Some(los) = world.get_resource::<crate::line_of_sight::LineOfSight>()
        && !los.clear(map, from + Vec3::Y, destination + Vec3::Y)
    {
        retry_path(world, unit, state, LOS_RETRY);
        return;
    }
    cancel_queries(world, unit);
    state.query = Some(FleeQuery::Path {
        map,
        from,
        destination,
    });
    // Submit this stage now; only the next fixed publication may make it ready.
    let _ = world.resource_mut::<Navigation>().query_deferred(
        unit,
        map.map_id,
        Goal::FearPath,
        NavQuery::Path {
            from,
            to: destination,
        },
    );
}

fn caster_geometry(world: &World, unit: Entity, map: InMap) -> Option<(Option<Vec3>, f32)> {
    let caster = world.get::<Feared>(unit)?.flee_from;
    let position = world
        .get::<Position>(caster)
        .filter(|_| world.get::<InMap>(caster) == Some(&map));
    let Some(position) = position else {
        return Some((None, 0.0));
    };
    let distance = crate::scripted_ai::model_reach::distance(world, unit, caster);
    if distance.is_none() {
        error!("Fear {unit}: missing source combat reach for {caster}; no guessed flee distance");
    }
    distance.map(|distance| (Some(point(*position)), distance))
}

fn start_path(destination: Vec3, points: Vec<Vec3>, state: &mut Fleeing) {
    let length: f32 = points.windows(2).map(|p| p[0].distance(p[1])).sum();
    if length > MAX_PATH_LENGTH {
        return;
    }
    state.destination = destination;
    state.path = Some(Corners::new(points));
}

fn clip_collision(world: &World, map: InMap, from: Vec3, destination: Vec3) -> Vec3 {
    let Some(los) = world.get_resource::<crate::line_of_sight::LineOfSight>() else {
        return destination;
    };
    let Some(hit) = los.first_hit(map, from + Vec3::Y, destination + Vec3::Y) else {
        return destination;
    };
    // Object.cpp:2881-2890: dynamic hit offset0.5 plus CONTACT_DISTANCE0.5.
    let length = from.distance(destination);
    from.lerp(destination, (hit - 1.0 / length.max(1.0)).max(0.0))
}

fn point(position: Position) -> Vec3 {
    Vec3::new(position.x, position.y, position.z)
}

fn move_unit(world: &mut World, unit: Entity, before: Vec3, after: Vec3, speed: f32) {
    world.entity_mut(unit).insert(Position {
        x: after.x,
        y: after.y,
        z: after.z,
    });
    world.entity_mut(unit).insert(MovementSpeed(speed));
    if let Some(mut rotation) = world.get_mut::<Rotation>(unit)
        && let Some(facing) =
            crate::creature_ai::facing_along(after.x - before.x, after.z - before.z)
    {
        rotation.y = facing;
    }
    if let Some(mut motion) = world.get_mut::<CreatureMotion>(unit) {
        *motion = CreatureMotion::Run;
    }
    crate::interest::mark_interest_position_changed(
        world.get_resource::<crate::interest::InterestDirtyQueue>(),
        unit,
    );
    crate::spell_cast::mark_dirty(world, unit);
}
