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
    if movement_paused(world, unit) {
        publish_step(world, unit, from, from, to, 0.0);
        return Ok(());
    }
    let (next, speed) = plan_caster_step(world, unit, target, from, to)?;
    let next = match probe_step(world, unit, from, next) {
        Ok(next) => next,
        Err(error) => {
            publish_step(world, unit, from, from, to, 0.0);
            return Err(error);
        }
    };
    publish_step(world, unit, from, next, to, speed);
    Ok(())
}

fn movement_paused(world: &World, unit: Entity) -> bool {
    let casting = world
        .get::<shared::casting::CastState>(unit)
        .is_some_and(|cast| !cast.cast_while_moving)
        || world
            .get::<super::smart_ai::CombatMovementDisabled>(unit)
            .is_some();
    let multiplier = world
        .get::<crate::auras::AuraStatMods>(unit)
        .map_or(1.0, |mods| mods.movement_speed);
    casting || multiplier <= 0.0
}

fn plan_caster_step(
    world: &mut World,
    unit: Entity,
    target: Entity,
    from: Vec3,
    to: Vec3,
) -> Result<(Vec3, f32), String> {
    let multiplier = world
        .get::<crate::auras::AuraStatMods>(unit)
        .map_or(1.0, |mods| mods.movement_speed);
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
    Ok((next, speed))
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
        chase.travel = select_travel(distance, reach, tolerance);
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
    let offset = match travel {
        Travel::Approach => to - from,
        Travel::Retreat => from - to,
    };
    // TC GetAbsoluteAngle uses atan2(0,0)=0, including coincident retreat.
    let angle = offset.z.atan2(offset.x);
    let direction = Vec3::new(angle.cos(), 0.0, angle.sin());
    from + direction * step.min(remaining)
}

fn select_travel(distance: f32, reach: f32, tolerance: f32) -> Option<Travel> {
    if distance < reach {
        Some(Travel::Retreat)
    } else if distance >= tolerance + CONTACT_DISTANCE {
        Some(Travel::Approach)
    } else {
        None
    }
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
    let bearing_changed = write_bearing(world, unit, to, target, if moving { speed } else { 0.0 });
    if moving {
        *world.get_mut::<Position>(unit).expect("moving caster") = Position {
            x: to.x,
            y: to.y,
            z: to.z,
        };
        crate::interest::mark_interest_position_changed(
            world.get_resource::<crate::interest::InterestDirtyQueue>(),
            unit,
        );
    }
    if moving || bearing_changed {
        crate::spell_cast::mark_dirty(world, unit);
    }
}

fn write_bearing(world: &mut World, unit: Entity, to: Vec3, target: Vec3, speed: f32) -> bool {
    let mut changed = false;
    if let Some(mut motion) = world.get_mut::<CreatureMotion>(unit) {
        let desired = if speed > 0.0 {
            CreatureMotion::Run
        } else {
            CreatureMotion::Still
        };
        changed |= motion.set_if_neq(desired);
    }
    if let Some(mut movement) = world.get_mut::<MovementSpeed>(unit) {
        changed |= movement.set_if_neq(MovementSpeed(speed));
    }
    if let Some(mut rotation) = world.get_mut::<shared::components::Rotation>(unit)
        && let Some(facing) = crate::creature_ai::facing_along(target.x - to.x, target.z - to.z)
    {
        let desired = shared::components::Rotation {
            y: facing,
            ..*rotation
        };
        changed |= rotation.set_if_neq(desired);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encounter_coverage_caster_range_hysteresis_approaches_source_tolerance() {
        let victim = Entity::from_bits(1);
        let mut chase = CasterChase::new(100.0);
        let from = Vec3::ZERO;
        let inside = Vec3::X * 104.99;
        assert_eq!(plan_step(&mut chase, victim, from, inside, 4.5, 0.35), from);
        let threshold = Vec3::X * 105.0;
        let first = plan_step(&mut chase, victim, from, threshold, 4.5, 0.35);
        assert!((first.x - 0.35).abs() < 0.001);
        let second = plan_step(&mut chase, victim, first, threshold, 4.5, 0.35);
        assert!(
            (second.x - 0.5).abs() < 0.001,
            "finish at104.5yd, not first point inside105"
        );
        assert_eq!(
            plan_step(&mut chase, victim, second, threshold, 4.5, 0.35),
            second
        );
    }

    #[test]
    fn encounter_coverage_caster_contact_retreat_persists_until_source_tolerance() {
        let victim = Entity::from_bits(1);
        let mut chase = CasterChase::new(100.0);
        let target = Vec3::X * 3.0;
        let mut position = Vec3::ZERO;
        for _ in 0..400 {
            position = plan_step(&mut chase, victim, position, target, 4.5, 0.35);
        }
        assert!(
            (position.x + 101.5).abs() < 0.001,
            "retreat continues to100+reaches, not just outside contact"
        );
        assert_eq!(
            plan_step(&mut chase, victim, position, target, 4.5, 0.35),
            position
        );
    }
}
