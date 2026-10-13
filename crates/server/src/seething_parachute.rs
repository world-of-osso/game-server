//! Seething Shore disembark parachute (TC a352b1fa battleground_seething_shore.cpp:530-641).
//! - 250917 Rocket Parachute: its Effect0 PERIODIC_DUMMY tick casts 250921 on the unit.
//! - 250921 Rocket Parachute: its Effect2 PERIODIC_TRIGGER_SPELL tick casts 269779 once,
//!   when the unit is at most 30 yd above the floor; removing its Effect0 removes 250917.
//! - 269779 Parachute: Effect0 casts the spell its calculated value names plus the two
//!   achievement trackers; Effect1 restarts the player's fall at its current height
//!   (`SetFallInformation(0, GetPositionZ())`) in place of the knockback.

use bevy::prelude::*;
use shared::components::{Player, Position};

use crate::auras::scripts::{AuraScript, AuraScripts};
use crate::auras::{AuraInstance, RemoveReason};
use crate::ground::{GroundMap, GroundProbe};
use crate::maps::InMap;
use crate::spell_cast::effects::EffectCtx;
use crate::spell_info::SpellEffect;

const ROCKET_PARACHUTE: u32 = 250_917;
const ROCKET_PARACHUTE_2: u32 = 250_921;
pub(crate) const PARACHUTE: u32 = 269_779;
const ACHIEVEMENT_TRACKER_CREDIT_CLAIM_JUMPER: u32 = 261_955;
const ACHIEVEMENT_TRACKER_DEATH_FROM_ABOVE: u32 = 261_954;
/// `delta <= 30.0f` of `HandlePeriodicTrigger`.
const DEPLOY_HEIGHT: f32 = 30.0;
/// Casts from aura hooks resolve as periodic tick triggers do (no proc events).
const HOOK_TRIGGER_DEPTH: u8 = 1;

/// `_parachuteDeployed` of the 250921 aura instance whose ground check cast 269779.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
struct ParachuteDeployed(u32);

pub(crate) fn register_aura_scripts(mut scripts: ResMut<AuraScripts>) {
    scripts.0.insert(
        ROCKET_PARACHUTE,
        AuraScript {
            on_periodic: Some(trigger_ground_check),
            ..Default::default()
        },
    );
    scripts.0.insert(
        ROCKET_PARACHUTE_2,
        AuraScript {
            on_periodic: Some(check_ground),
            on_remove: Some(remove_rocket_parachute),
            ..Default::default()
        },
    );
    scripts.0.insert(
        PARACHUTE,
        AuraScript {
            on_apply: Some(apply_parachute_aura),
            ..Default::default()
        },
    );
}

/// `spell_bg_seething_shore_rocket_parachute_trigger::HandlePeriodic` (EFFECT_0).
fn trigger_ground_check(world: &mut World, unit: Entity, _aura: &AuraInstance, effect: u8) {
    if effect == 0 {
        crate::spell_cast::cast_triggered(
            world,
            unit,
            ROCKET_PARACHUTE_2,
            Some(unit),
            HOOK_TRIGGER_DEPTH,
        );
    }
}

/// `spell_bg_seething_shore_rocket_parachute_ground_check::HandlePeriodicTrigger` (EFFECT_2).
fn check_ground(world: &mut World, unit: Entity, aura: &AuraInstance, effect: u8) {
    if effect != 2
        || world.get::<ParachuteDeployed>(unit) == Some(&ParachuteDeployed(aura.instance_id))
    {
        return;
    }
    let Some(delta) = height_above_floor(world, unit) else {
        return;
    };
    if delta <= DEPLOY_HEIGHT
        && crate::spell_cast::try_cast_triggered(
            world,
            unit,
            PARACHUTE,
            Some(unit),
            HOOK_TRIGGER_DEPTH,
        )
    {
        world
            .entity_mut(unit)
            .insert(ParachuteDeployed(aura.instance_id));
    }
}

/// `GetPositionZ() - GetFloorZ()`: no floor under the unit is no delta.
fn height_above_floor(world: &mut World, unit: Entity) -> Option<f32> {
    let position = *world.get::<Position>(unit)?;
    let map = *world.get::<InMap>(unit)?;
    let feet = Vec3::new(position.x, position.y, position.z);
    match world.get_resource_mut::<GroundMap>()?.ground_in(map, feet) {
        GroundProbe::Supported(floor) => Some(position.y - floor),
        GroundProbe::Unsupported | GroundProbe::Unloaded => None,
    }
}

/// `HandleRemove` (EFFECT_0 CAN_TURN_WHILE_FALLING): stops 250917 re-applying 250921.
fn remove_rocket_parachute(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    _reason: RemoveReason,
) {
    if world.get::<ParachuteDeployed>(unit) == Some(&ParachuteDeployed(aura.instance_id)) {
        world.entity_mut(unit).remove::<ParachuteDeployed>();
    }
    if aura.effects.iter().any(|effect| effect.index == 0) {
        crate::auras::remove_auras(world, unit, RemoveReason::Cancelled, |aura| {
            aura.spell_id == ROCKET_PARACHUTE
        });
    }
}

/// `spell_bg_seething_shore_parachute::ApplyParachuteAura` (EFFECT_0 APPLY_AURA).
fn apply_parachute_aura(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if world.get::<Player>(unit).is_none() {
        return;
    }
    let Some(effect) = aura.effects.iter().find(|effect| effect.index == 0) else {
        return;
    };
    let spells = [
        effect.amount as u32,
        ACHIEVEMENT_TRACKER_CREDIT_CLAIM_JUMPER,
        ACHIEVEMENT_TRACKER_DEATH_FROM_ABOVE,
    ];
    for spell_id in spells {
        crate::spell_cast::cast_triggered(world, unit, spell_id, Some(unit), HOOK_TRIGGER_DEPTH);
    }
}

/// `spell_bg_seething_shore_parachute::ResetFallDamage` (EFFECT_1 KNOCK_BACK, TARGET_UNIT_CASTER).
pub(crate) fn reset_fall_information(world: &mut World, ctx: &EffectCtx, _effect: &SpellEffect) {
    let player = ctx.caster;
    if world.get::<Player>(player).is_none() {
        return;
    }
    let Some(height) = world.get::<Position>(player).map(|position| position.y) else {
        return;
    };
    if let Some(mut fall) = world.get_mut::<crate::networking::FallTracker>(player) {
        fall.restart_at(height);
    }
}

#[cfg(test)]
#[path = "seething_parachute_tests.rs"]
mod tests;
