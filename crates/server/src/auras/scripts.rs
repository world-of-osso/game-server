//! Script hooks for script-only aura types (DUMMY 4, PERIODIC_DUMMY 226),
//! keyed by spell ID. Class registrations below run on
//! apply and remove of that spell's auras, on its PERIODIC_DUMMY ticks and on
//! its procs. Every hook receives the unit carrying the aura.

use std::collections::HashMap;

use bevy::prelude::*;

use super::AuraInstance;

pub type AuraHook = fn(&mut World, Entity, &AuraInstance);
pub type RemoveHook = fn(&mut World, Entity, &AuraInstance, super::RemoveReason);
/// Also receives the ticking effect index.
pub type PeriodicHook = fn(&mut World, Entity, &AuraInstance, u8);
/// Also receives the other party of the proc event.
pub type ProcHook = fn(&mut World, Entity, &AuraInstance, Option<Entity>);

#[derive(Debug, Clone, Copy, Default)]
pub struct AuraScript {
    pub on_apply: Option<AuraHook>,
    pub on_remove: Option<RemoveHook>,
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

pub(super) fn run_remove(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    reason: super::RemoveReason,
) {
    if let Some(run) = script(world, aura.spell_id).and_then(|script| script.on_remove) {
        run(world, unit, aura, reason);
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

const ASPECT_OF_THE_TURTLE: u32 = 186_265;
const TURTLE_PACIFY: u32 = 205_769;
const BARKSKIN: u32 = 22_812;
const BRAMBLES: u32 = 203_953;
const BRAMBLES_DAMAGE: u32 = 213_709;
const DRAIN_SOUL: u32 = 198_590;
const DRAIN_SOUL_ENERGIZE: u32 = 205_292;

pub(super) fn register_class_scripts(mut scripts: ResMut<AuraScripts>) {
    scripts.0.insert(
        DRAIN_SOUL,
        AuraScript {
            on_remove: Some(drain_soul_shard_on_death),
            ..Default::default()
        },
    );
    scripts.0.insert(
        BARKSKIN,
        AuraScript {
            on_periodic: Some(barkskin_brambles),
            ..Default::default()
        },
    );
    scripts.0.insert(
        ASPECT_OF_THE_TURTLE,
        AuraScript {
            on_apply: Some(apply_turtle_pacify),
            on_remove: Some(remove_turtle_pacify),
            ..Default::default()
        },
    );
}

/// TC a352b1fa spell_druid.cpp:297-316, spell_dru_barkskin. Effect 2's
/// PERIODIC_DUMMY ticks only deal damage when Brambles is present. Catalog
/// 203953's tooltip references 213709s1 and 22812t3; the child supplies the
/// AoE targeting and 0.075 AP coefficient, the parent the 1000ms cadence.
fn barkskin_brambles(world: &mut World, unit: Entity, _aura: &AuraInstance, effect_index: u8) {
    if effect_index == 2 && super::has_aura(world, unit, BRAMBLES) {
        crate::spell_cast::cast_triggered(world, unit, BRAMBLES_DAMAGE, Some(unit), 1);
    }
}

/// TrinityCore a352b1fa spell_hun_aspect_of_the_turtle applies catalog 205769
/// while the parent is present, then removes it (spell_hunter.cpp:198-220).
fn apply_turtle_pacify(world: &mut World, unit: Entity, _aura: &AuraInstance) {
    super::apply_aura(
        world,
        &super::AuraApplication {
            caster: unit,
            target: unit,
            spell_id: TURTLE_PACIFY,
            effect_mask: u32::MAX,
            source: super::AuraSource::Cast,
        },
    );
}

/// TC a352b1fa spell_warlock.cpp:831-848: only death removal awards
/// 205292 (10 raw Soul Shards = one whole shard). Expiry/cancellation do not.
fn drain_soul_shard_on_death(
    world: &mut World,
    _unit: Entity,
    aura: &AuraInstance,
    reason: super::RemoveReason,
) {
    if reason == super::RemoveReason::Death {
        crate::spell_cast::cast_triggered(
            world,
            aura.caster,
            DRAIN_SOUL_ENERGIZE,
            Some(aura.caster),
            1,
        );
    }
}

fn remove_turtle_pacify(
    world: &mut World,
    unit: Entity,
    _aura: &AuraInstance,
    _reason: super::RemoveReason,
) {
    super::remove_auras(world, unit, super::RemoveReason::Cancelled, |aura| {
        aura.spell_id == TURTLE_PACIFY && aura.caster == unit
    });
}
