//! Script hooks for script-only aura types (DUMMY 4, PERIODIC_DUMMY 226),
//! keyed by spell ID. The registry starts empty; a registered script runs on
//! apply and remove of that spell's auras, on its PERIODIC_DUMMY ticks and on
//! its procs. Every hook receives the unit carrying the aura.

use std::collections::HashMap;

use bevy::prelude::*;

use super::AuraInstance;

pub type AuraHook = fn(&mut World, Entity, &AuraInstance);
/// Also receives the ticking effect index.
pub type PeriodicHook = fn(&mut World, Entity, &AuraInstance, u8);
/// Also receives the other party of the proc event.
pub type ProcHook = fn(&mut World, Entity, &AuraInstance, Option<Entity>);

#[derive(Debug, Clone, Copy, Default)]
pub struct AuraScript {
    pub on_apply: Option<AuraHook>,
    pub on_remove: Option<AuraHook>,
    pub on_periodic: Option<PeriodicHook>,
    pub on_proc: Option<ProcHook>,
}

#[derive(Resource, Debug, Default)]
pub struct AuraScripts(pub HashMap<u32, AuraScript>);

fn script(world: &World, spell_id: u32) -> Option<AuraScript> {
    world
        .get_resource::<AuraScripts>()
        .and_then(|scripts| scripts.0.get(&spell_id).copied())
}

pub(super) fn run_apply(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if let Some(run) = script(world, aura.spell_id).and_then(|script| script.on_apply) {
        run(world, unit, aura);
    }
}

pub(super) fn run_remove(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if let Some(run) = script(world, aura.spell_id).and_then(|script| script.on_remove) {
        run(world, unit, aura);
    }
}

pub(super) fn run_proc(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    other: Option<Entity>,
) {
    if let Some(run) = script(world, aura.spell_id).and_then(|script| script.on_proc) {
        run(world, unit, aura, other);
    }
}

pub(super) fn run_periodic(world: &mut World, unit: Entity, aura: &AuraInstance, effect_index: u8) {
    if let Some(run) = script(world, aura.spell_id).and_then(|script| script.on_periodic) {
        run(world, unit, aura, effect_index);
    }
}
