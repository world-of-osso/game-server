//! Earned Sudden Death Execute uses Executioner's native Onslaught value.
use bevy::prelude::*;

use super::combat;
use combat::auras::ActiveAuras;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterSpenderProc(after_execute)];

const ONSLAUGHT: u32 = 444780;
const EXECUTIONER: u32 = 445584;
const SUDDEN_DEATH: u32 = 52437;
const OVERWHELMED: u32 = 445836;
const BLADESTORM: u32 = 227847;
const EXECUTES: &[u32] = &[163201, 281000, 280735];

fn after_execute(world: &mut World, caster: Entity, spell: &SpellInfo, target: Option<Entity>) {
    // SimC76f8118 sc_warrior.cpp4361–4365/4561–4565: SD Execute reduces
    // the live Bladestorm cooldown and applies Overwhelmed per Executioner stack.
    // This runs after Reap but before the existing SD consumer; neither source
    // counter nor the independently maintained Executioner aura is replaced.
    let Some(stacks) = read_executioner_stacks(world, caster, spell.id) else {
        return;
    };
    let Some(target) = target else {
        warn!(?caster, "Native Onslaught Execute lacks its primary");
        return;
    };
    let seconds = script_effect_value(world, caster, ONSLAUGHT, 0) * f32::from(stacks);
    let action = combat::auras::overrides::resolve_spell(world, caster, BLADESTORM);
    combat::spell_cast::scripts::reduce_cooldown(world, caster, action, f64::from(seconds));
    apply_overwhelmed(world, caster, target, stacks);
}

fn read_executioner_stacks(world: &World, caster: Entity, action: u32) -> Option<u8> {
    if !EXECUTES.contains(&action)
        || !combat::auras::has_aura(world, caster, ONSLAUGHT)
        || !combat::auras::has_aura(world, caster, SUDDEN_DEATH)
    {
        return None;
    }
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == EXECUTIONER && aura.caster == caster)
        .map(|aura| aura.stacks)
}

fn apply_overwhelmed(world: &mut World, caster: Entity, target: Entity, executioner: u8) {
    let per_stack = script_effect_value(world, caster, ONSLAUGHT, 1);
    let stacks = (per_stack * f32::from(executioner)) as u8;
    let values = combat::spell_cast::TriggeredSpellOverrides {
        aura_stacks: Some(stacks),
        ..Default::default()
    };
    if !combat::spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        OVERWHELMED,
        Some(target),
        1,
        values,
    ) {
        warn!(
            ?caster,
            ?target,
            "Native Onslaught Overwhelmed payload refused"
        );
    }
}
