//! TC a352b1fa spell_shaman.cpp861–920: native passive proc target + periodic child.
use bevy::prelude::*;

use super::procs::AuraEvent;
use super::scripts::{AuraScript, AuraScripts};
use super::{AuraInstance, RemoveReason};

pub(super) const PASSIVE: u32 = 170_374;
const PERIODIC: u32 = 170_377;
const DAMAGE: u32 = 170_379;

/// TC passive AuraScript::_procTargetGuid: only this caster's latest action target.
#[derive(Component)]
struct EarthenRageTarget(Entity);

pub(super) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        PASSIVE,
        AuraScript {
            check_proc: Some(check_spell),
            on_damage_proc: Some(record_target_and_cast_proc),
            on_remove: Some(clear_target),
            ..Default::default()
        },
    );
    scripts.0.insert(
        PERIODIC,
        AuraScript {
            on_periodic: Some(cast_damage_on_tick),
            ..Default::default()
        },
    );
}

fn check_spell(_world: &World, _aura: &AuraInstance, event: &AuraEvent) -> bool {
    event.spell_id.is_some_and(|spell| spell != DAMAGE) && event.target.is_some()
}

fn record_target_and_cast_proc(
    world: &mut World,
    unit: Entity,
    _aura: &AuraInstance,
    event: &AuraEvent,
) {
    let target = event
        .target
        .expect("Earthen Rage proc requires its actual action target");
    world.entity_mut(unit).insert(EarthenRageTarget(target));
    crate::spell_cast::cast_triggered(world, unit, PERIODIC, Some(unit), 1);
}

fn cast_damage_on_tick(world: &mut World, unit: Entity, aura: &AuraInstance, index: u8) {
    if index != 0 || !super::has_aura(world, aura.caster, PASSIVE) {
        return;
    }
    let Some(EarthenRageTarget(target)) = world.get::<EarthenRageTarget>(aura.caster) else {
        return;
    };
    let target = *target;
    if world.get_entity(target).is_err() {
        return;
    }
    crate::spell_cast::cast_triggered(world, unit, DAMAGE, Some(target), 1);
}

fn clear_target(world: &mut World, unit: Entity, _aura: &AuraInstance, _reason: RemoveReason) {
    world.entity_mut(unit).remove::<EarthenRageTarget>();
}
