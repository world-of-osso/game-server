//! Sheilun's Gift cloud production and consumption from 70448 spell data.
//! WCL BaN8nxT64QztvMkH f1 p1 records both cloud auras every eight seconds
//! and removes them after the 399491 cast.
use super::{cast_relationships::has_talent, combat};
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;
use shared::components::CombatStatus;

const GIFT: u32 = 399491;
const TIMER: u32 = 399494;
const CLOUD_OVERRIDE: u32 = 399497;
const CLOUD_COUNT: u32 = 399510;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterQueuedTriggers(update_timers),
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::NamedHealingMultiplier(cloud_healing_multiplier),
    SpellScriptHook::AfterResourceValues(after_cast),
    SpellScriptHook::ConsumedEffectValues(&[(GIFT, 4)]),
];

fn update_timers(world: &mut World) {
    let actors: Vec<_> = world
        .query_filtered::<(Entity, &CombatStatus), With<TraitLoadout>>()
        .iter(world)
        .map(|(unit, status)| (unit, status.0))
        .collect();
    for (unit, in_combat) in actors {
        if !has_talent(world, unit, GIFT) {
            continue;
        }
        if in_combat && !auras::has_aura(world, unit, TIMER) {
            if !spell_cast::try_cast_triggered(world, unit, TIMER, Some(unit), 1) {
                warn!(?unit, "Sheilun cloud timer failed");
            }
        } else if !in_combat && auras::has_aura(world, unit, TIMER) {
            remove_cloud_auras(world, unit, &[TIMER]);
        }
    }
}

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(TIMER).or_default().on_periodic = Some(generate_cloud);
}

fn generate_cloud(world: &mut World, unit: Entity, aura: &AuraInstance, effect: u8) {
    if aura.caster != unit || effect != 0 {
        return;
    }
    for spell in [CLOUD_OVERRIDE, CLOUD_COUNT] {
        if !spell_cast::try_cast_triggered(world, unit, spell, Some(unit), 1) {
            warn!(?unit, spell, "Sheilun cloud application failed");
        }
    }
}

fn cloud_healing_multiplier(world: &World, caster: Entity, _: Entity, spell: &SpellInfo) -> f32 {
    if spell.id != GIFT {
        return 1.0;
    }
    let stacks = world.get::<ActiveAuras>(caster).map_or(0, |auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == CLOUD_COUNT && aura.caster == caster)
            .map_or(0, |aura| aura.stacks)
    });
    let percent = combat::spell_cast::scripts::script_effect_value(world, caster, GIFT, 4);
    1.0 + f32::from(stacks) * percent / 100.0
}

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == GIFT).then_some(consume_clouds)
}

fn consume_clouds(world: &mut World, caster: Entity, _: &SpellInfo) {
    remove_cloud_auras(world, caster, &[CLOUD_OVERRIDE, CLOUD_COUNT]);
}

fn remove_cloud_auras(world: &mut World, caster: Entity, spells: &[u32]) {
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.caster == caster && spells.contains(&aura.spell_id)
    });
}
