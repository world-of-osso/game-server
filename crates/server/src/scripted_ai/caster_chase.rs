//! TC a352b1fa UnitAI.cpp:47, MovementDefines.cpp and ChaseMovementGenerator.
//! Direct ground-supported movement subset; obstacle-generated paths are not implemented.
use bevy::prelude::*;
use shared::components::{CreatureMotion, MovementSpeed, Position};

use crate::creature_ai::CreatureState;

/// TC ObjectDefines.h CONTACT_DISTANCE, used by ChaseRange(float).
const CONTACT_DISTANCE: f32 = 0.5;

#[derive(Clone, Copy)]
enum Travel {
    Approach,
    Retreat,
}

/// AttackStartCaster's non-melee attack and persistent chase generator.
#[derive(Component)]
pub(crate) struct CasterChase {
    range: f32,
    travel: Option<Travel>,
    victim: Option<Entity>,
}

impl CasterChase {
    pub(crate) fn new(range: f32) -> Self {
        Self {
            range,
            travel: None,
            victim: None,
        }
    }
}

pub(crate) fn advance(world: &mut World) {
    let units: Vec<_> = world
        .query_filtered::<Entity, (With<CasterChase>, crate::creature_ai::ActiveUncontrolled)>()
        .iter(world)
        .collect();
    for unit in units {
        if let Err(error) = advance_caster(world, unit) {
            error!("Caster chase {unit:?}: {error}");
        }
    }
}

fn advance_caster(world: &mut World, unit: Entity) -> Result<(), String> {
    let Some(CreatureState::Pursuing { target }) = world.get::<CreatureState>(unit).copied() else {
        return Ok(());
    };
    let target = Entity::from_bits(target);
    let from = crate::spell_cast::targets::position(world, unit).ok_or("caster has no position")?;
    let to = crate::spell_cast::targets::position(world, target).ok_or("victim has no position")?;
    let paused = world
        .get::<shared::casting::CastState>(unit)
        .is_some_and(|cast| !cast.cast_while_moving)
        || world
            .get::<super::smart_ai::CombatMovementDisabled>(unit)
            .is_some();
    let multiplier = world
        .get::<crate::auras::AuraStatMods>(unit)
        .map_or(1.0, |mods| mods.movement_speed);
    if paused || multiplier <= 0.0 {
        publish_step(world, unit, from, from, to, 0.0);
        return Ok(());
    }
    let reach = super::model_reach::combat_reach(world, unit)
        .ok_or("caster has no sourced combat reach")?
        + super::model_reach::combat_reach(world, target)
            .ok_or("victim has no sourced combat reach")?;
    let speed = world
        .get::<crate::creature_ai::SpawnPoint>(unit)
        .ok_or("caster has no sourced run speed")?
        .speed_run
        * crate::creature_ai::BASE_RUN_SPEED
        * multiplier;
    let step = speed * world.resource::<Time>().delta_secs();
    let next = plan_step(
        &mut world.get_mut::<CasterChase>(unit).expect("caster policy"),
        target,
        from,
        to,
        reach,
        step,
    );
    let next = probe_step(world, unit, from, next)?;
    publish_step(world, unit, from, next, to, speed);
    Ok(())
}

fn plan_step(
    chase: &mut CasterChase,
    victim: Entity,
    from: Vec3,
    to: Vec3,
    reach: f32,
    step: f32,
) -> Vec3 {
    if chase.victim != Some(victim) {
        chase.victim = Some(victim);
        chase.travel = None;
    }
    let distance = from.distance(to);
    let tolerance = chase.range + reach;
    if chase.travel.is_none() {
        chase.travel = if distance < reach {
            Some(Travel::Retreat)
        } else if distance >= tolerance + CONTACT_DISTANCE {
            Some(Travel::Approach)
        } else {
            None
        };
    }
    let Some(travel) = chase.travel else {
        return from;
    };
    let remaining = match travel {
        Travel::Approach => distance - tolerance,
        Travel::Retreat => tolerance - distance,
    };
    if remaining <= 0.0 {
        chase.travel = None;
        return from;
    }
    let offset = to - from;
    let direction = Vec3::new(offset.x, 0.0, offset.z).normalize_or_zero();
    let direction = match travel {
        Travel::Approach => direction,
        Travel::Retreat => -direction,
    };
    from + direction * step.min(remaining)
}

fn probe_step(world: &mut World, unit: Entity, from: Vec3, to: Vec3) -> Result<Vec3, String> {
    if from == to {
        return Ok(to);
    }
    let map = *world
        .get::<crate::maps::InMap>(unit)
        .ok_or("caster has no map")?;
    let los = world
        .get_resource::<crate::line_of_sight::LineOfSight>()
        .ok_or("collision service unavailable")?;
    let mut next = crate::spell_cast::teleport::clamp_to_wall(los, map, from, to);
    let mut ground = world
        .get_resource_mut::<crate::ground::GroundMap>()
        .ok_or("ground service unavailable")?;
    let crate::ground::GroundProbe::Supported(height) = ground.creature_ground_in(map, next) else {
        return Err("next caster step has no supported ground".into());
    };
    next.y = height;
    Ok(next)
}

fn publish_step(world: &mut World, unit: Entity, from: Vec3, to: Vec3, target: Vec3, speed: f32) {
    let moving = from != to;
    *world.get_mut::<Position>(unit).expect("moving caster") = Position {
        x: to.x,
        y: to.y,
        z: to.z,
    };
    if let Some(mut motion) = world.get_mut::<CreatureMotion>(unit) {
        *motion = if moving {
            CreatureMotion::Run
        } else {
            CreatureMotion::Still
        };
    }
    if let Some(mut movement) = world.get_mut::<MovementSpeed>(unit) {
        movement.0 = if moving { speed } else { 0.0 };
    }
    if let Some(mut rotation) = world.get_mut::<shared::components::Rotation>(unit)
        && let Some(facing) = crate::creature_ai::facing_along(target.x - to.x, target.z - to.z)
    {
        rotation.y = facing;
    }
    crate::interest::mark_interest_position_changed(
        world.get_resource::<crate::interest::InterestDirtyQueue>(),
        unit,
    );
    crate::spell_cast::mark_dirty(world, unit);
}
