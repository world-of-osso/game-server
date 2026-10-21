//! TC a352b1fa EffectCharge/EffectChargeDest: launch movement, sourced hit delay.
//! Two-point movement subset, matching the existing direct point generator.
//! Obstacle-generated paths are not implemented.
use super::{effects::EffectCtx, *};
use shared::components::{CreatureMotion, MovementSpeed, Position, Rotation};

const ATTACK_ON_CHARGE: u32 = 0x0004_0000;
const SPEED_IS_DELAY: u32 = 0x10;
/// TC MovementDefines.h: SPEED_CHARGE.
const DEFAULT_CHARGE_SPEED: f32 = 42.0;

#[derive(Component)]
pub(crate) struct ChargeMovement {
    from: Vec3,
    to: Vec3,
    started: f64,
    duration_ms: u64,
    finished: bool,
}

pub(super) enum ChargeDelay {
    Unit { target: Entity, milliseconds: u64 },
    Destination { milliseconds: u64 },
}

pub(super) fn launch(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    plan: &TargetPlan,
) -> Result<Option<ChargeDelay>, String> {
    let Some(planned) = plan.iter().find(|planned| {
        matches!(
            spell.effects[planned.index].effect,
            96 | effects::EFFECT_CHARGE_DEST
        )
    }) else {
        return Ok(None);
    };
    let (movement, speed, delay) = if spell.effects[planned.index].effect == 96 {
        prepare_unit_charge(world, caster, spell, planned)?
    } else {
        prepare_destination_charge(world, caster, spell, planned)?
    };
    world
        .entity_mut(caster)
        .insert((movement, CreatureMotion::Run, MovementSpeed(speed)));
    Ok(Some(delay))
}

fn prepare_unit_charge(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    planned: &PlannedEffect,
) -> Result<(ChargeMovement, f32, ChargeDelay), String> {
    let target = *planned.units.first().ok_or("charge has no unit target")?;
    let from = targets::position(world, caster).ok_or("charge caster has no position")?;
    let position = targets::position(world, target).ok_or("charge target has no position")?;
    let reach = crate::scripted_ai::model_reach::combat_reach(world, target)
        .ok_or("charge target has no sourced combat reach")?;
    let (movement, speed) = calculate_movement(from, position, reach, spell, now(world));
    let delay = ChargeDelay::Unit {
        target,
        milliseconds: movement.duration_ms,
    };
    Ok((movement, speed, delay))
}

fn prepare_destination_charge(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    planned: &PlannedEffect,
) -> Result<(ChargeMovement, f32, ChargeDelay), String> {
    let from =
        targets::position(world, caster).ok_or("destination charge caster has no position")?;
    let destination = planned
        .dest
        .ok_or("destination charge has no destination")?;
    let (movement, speed) =
        calculate_destination_movement(from, destination.position, spell, now(world));
    let delay = ChargeDelay::Destination {
        milliseconds: movement.duration_ms,
    };
    Ok((movement, speed, delay))
}

fn calculate_movement(
    from: Vec3,
    target: Vec3,
    reach: f32,
    spell: &SpellInfo,
    started: f64,
) -> (ChargeMovement, f32) {
    let toward_caster = Vec3::new(from.x - target.x, 0.0, from.z - target.z).normalize_or_zero();
    let to = target + toward_caster * reach;
    calculate_destination_movement(from, to, spell, started)
}

fn calculate_destination_movement(
    from: Vec3,
    to: Vec3,
    spell: &SpellInfo,
    started: f64,
) -> (ChargeMovement, f32) {
    let distance = from.distance(to);
    let authored_speed = spell.misc.as_ref().map_or(0.0, |misc| misc.speed);
    let speed_is_delay = attributes(spell, 9) & SPEED_IS_DELAY != 0;
    let (seconds, speed) = calculate_timing(distance, authored_speed, speed_is_delay);
    // MoveSpline CommonInitializer: integer segment time starts at1ms.
    let duration_ms = 1 + (seconds * 1000.0) as u64;
    (
        ChargeMovement {
            from,
            to,
            started,
            duration_ms,
            finished: false,
        },
        speed,
    )
}

fn calculate_timing(distance: f32, authored_speed: f32, speed_is_delay: bool) -> (f32, f32) {
    if authored_speed > 0.0 && speed_is_delay {
        return (authored_speed, distance / authored_speed);
    }
    let speed = if authored_speed > 0.0 {
        authored_speed
    } else {
        DEFAULT_CHARGE_SPEED
    };
    (distance / speed, speed)
}

/// Keep motion ownership through the landing tick, as with scripted jumps.
pub(super) fn release_finished(
    mut commands: Commands,
    movers: Query<(Entity, &ChargeMovement, Has<crate::combat::DeathState>)>,
) {
    for (unit, movement, dead) in &movers {
        if dead || movement.finished {
            commands.entity(unit).remove::<ChargeMovement>();
        }
    }
}

pub(super) fn advance(world: &mut World) {
    let clock = now(world);
    let moved: Vec<_> = world
        .query_filtered::<(
            Entity,
            &mut ChargeMovement,
            &mut Position,
            Option<&mut Rotation>,
        ), Without<crate::combat::DeathState>>()
        .iter_mut(world)
        .filter_map(|(unit, mut movement, mut position, rotation)| {
            step_charge(&mut movement, &mut position, rotation, clock).then_some(unit)
        })
        .collect();
    for unit in moved {
        publish_position(world, unit);
    }
}

fn step_charge(
    movement: &mut ChargeMovement,
    position: &mut Position,
    rotation: Option<Mut<Rotation>>,
    clock: f64,
) -> bool {
    if movement.finished {
        return false;
    }
    let elapsed_ms = (clock - movement.started) * 1000.0;
    let progress = (elapsed_ms / movement.duration_ms as f64).clamp(0.0, 1.0) as f32;
    let next = movement.from.lerp(movement.to, progress);
    *position = Position {
        x: next.x,
        y: next.y,
        z: next.z,
    };
    if let Some(mut rotation) = rotation
        && let Some(facing) =
            crate::creature_ai::facing_along(movement.to.x - next.x, movement.to.z - next.z)
    {
        rotation.y = facing;
    }
    movement.finished = progress >= 1.0;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encounter_coverage_charge_timing_preserves_velocity_default_and_delay_modes() {
        assert_eq!(calculate_timing(84.0, 42.0, false), (2.0, 42.0));
        assert_eq!(calculate_timing(63.0, 0.0, false), (1.5, 42.0));
        assert_eq!(calculate_timing(63.0, 2.0, true), (2.0, 31.5));
    }
}

fn publish_position(world: &World, unit: Entity) {
    let queue = world.get_resource::<crate::interest::InterestDirtyQueue>();
    crate::interest::mark_interest_position_changed(queue, unit);
    if let Some(queue) = world.get_resource::<lightyear::prelude::ReplicationDirtyQueue>() {
        queue.mark(unit);
    }
}

pub(super) fn hit(
    world: &mut World,
    ctx: &EffectCtx,
    effect: &crate::spell_info::SpellEffect,
    target: Entity,
) {
    if attributes(ctx.spell, 7) & ATTACK_ON_CHARGE != 0
        && targets::is_hostile(world, ctx.caster, target)
    {
        let current = world.get::<crate::combat::AutoAttack>(ctx.caster).cloned();
        crate::combat::start_attack(&mut world.commands(), ctx.caster, target, current.as_ref());
    }
    if effect.trigger_spell != 0 {
        super::cast_triggered(
            world,
            ctx.caster,
            effect.trigger_spell,
            Some(target),
            ctx.depth + 1,
        );
    }
}

/// TC EffectChargeDest HIT: optional child at the authored destination after flight.
pub(super) fn hit_destination(
    world: &mut World,
    ctx: &EffectCtx,
    effect: &crate::spell_info::SpellEffect,
) {
    if effect.trigger_spell != 0
        && let Some(destination) = ctx.dest
    {
        super::cast_triggered_at_destination(
            world,
            ctx.caster,
            effect.trigger_spell,
            destination.position,
            ctx.depth + 1,
        );
    }
}
