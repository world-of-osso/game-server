//! TC a352b1fa FleeingMovementGenerator.cpp:26-27,66-89,118-198:
//! quiet distance28..43, surface-collision destination, LOS200ms/path100ms retry,
//! navmesh-only path≤30yd, running then800..1500ms pause. Owner control is lost.
use super::{AuraStatMods, Feared};
use crate::{
    maps::InMap,
    navigation::{Corners, Navigation},
};
use bevy::prelude::*;
use rand::Rng;
use shared::components::{
    CreatureMotion, Health, MovementControl, MovementSpeed, Player, PlayerMotion, Position,
    Rotation,
};

#[derive(Component)]
pub(super) struct Fleeing {
    path: Option<Corners>,
    destination: Vec3,
    wait: f32,
    epoch: Option<u32>,
}

pub(super) fn start(world: &mut World, unit: Entity) {
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
    });
    stop_motion(world, unit);
}

pub(super) fn stop(world: &mut World, unit: Entity) {
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
        return;
    }
    let speed = run_speed(world, unit);
    let blocked = world
        .get::<Health>(unit)
        .is_some_and(|hp| hp.current <= 0.0)
        || speed == 0.0
        || world.get::<shared::casting::CastState>(unit).is_some();
    if blocked {
        state.path = None;
        stop_motion(world, unit);
    } else {
        state.wait -= dt;
        if state.path.is_none() && state.wait <= 0.0 {
            choose_path(world, unit, &mut state);
        }
        if let Some(path) = &mut state.path {
            let before = point(*world.get::<Position>(unit).expect("fleeing position"));
            let (after, reached) = path.advance(before, speed * dt, 0.0, state.destination);
            move_unit(world, unit, before, after, speed);
            if reached {
                state.path = None;
                state.wait = rand::thread_rng().gen_range(0.8..=1.5);
                stop_motion(world, unit);
            }
        }
    }
    world.entity_mut(unit).insert(state);
}

fn run_speed(world: &World, unit: Entity) -> f32 {
    let base = world
        .get::<crate::creature_ai::SpawnPoint>(unit)
        .map_or(7.0, |spawn| 7.0 * spawn.speed_run);
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
    let (distance, angle) = if distance < 28.0 {
        (
            rng.gen_range(0.4..=1.3) * (28.0 - distance),
            angle + rng.gen_range(-PI / 8.0..=PI / 8.0),
        )
    } else if distance > 43.0 {
        (
            rng.gen_range(0.4..=1.0) * 15.0,
            -angle + rng.gen_range(-PI / 4.0..=PI / 4.0),
        )
    } else {
        (rng.gen_range(0.6..=1.2) * 15.0, rng.gen_range(0.0..TAU))
    };
    from + Vec3::new(angle.cos() * distance, 0.0, angle.sin() * distance)
}

fn choose_path(world: &World, unit: Entity, state: &mut Fleeing) {
    state.wait = 0.1;
    let Some(nav) = world.get_resource::<Navigation>() else {
        return;
    };
    let Some(map) = world.get::<InMap>(unit).copied() else {
        return;
    };
    let from = point(*world.get::<Position>(unit).expect("fleeing position"));
    let caster = world
        .get::<Feared>(unit)
        .and_then(|fear| world.get::<Position>(fear.flee_from))
        .copied()
        .map(point);
    let distance = if caster.is_some() {
        let caster = world.get::<Feared>(unit).expect("feared owner").flee_from;
        let Some(distance) = crate::scripted_ai::model_reach::distance(world, unit, caster) else {
            debug!("Fear {unit}: missing source combat reach; no guessed flee distance");
            return;
        };
        distance
    } else {
        0.0
    };
    let wanted = flee_point(from, caster, distance, &mut rand::thread_rng());
    let destination = match nav.first_collision(map.map_id, from, wanted) {
        Ok(Some(destination)) => destination,
        Ok(None) => return,
        Err(err) => {
            debug!("Fear {unit} collision route: {err}");
            return;
        }
    };
    let destination = clip_collision(world, map, from, destination);
    if let Some(los) = world.get_resource::<crate::line_of_sight::LineOfSight>()
        && !los.clear(map, from + Vec3::Y, destination + Vec3::Y)
    {
        state.wait = 0.2;
        return;
    }
    let points = match nav.query_path(map.map_id, from, destination) {
        Ok(Some(points)) => points,
        Ok(None) => return,
        Err(err) => {
            debug!("Fear {unit} path: {err}");
            return;
        }
    };
    let length: f32 = points.windows(2).map(|p| p[0].distance(p[1])).sum();
    if length > 30.0 {
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
