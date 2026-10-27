//! Sudden Doom from actual Dread Plague ticks.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook};

#[derive(Component, Default)]
struct SuddenDoomAttempts(u32);

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::PeriodicDamageFinished(roll_sudden_doom)];

fn roll_sudden_doom(world: &mut World, ctx: &EffectCtx<'_>) {
    let caster = ctx.caster;
    if ctx.spell.id != 1240996 || !combat::auras::has_aura(world, caster, 49530) {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp7956–7957/14827–14830: Dread
    // Plague ticks use accumulated PRD with Harbinger and Ebon Fever.
    let value = |world: &World, id, index| {
        combat::spell_cast::scripts::script_effect_value(world, caster, id, index) as f64 / 100.0
    };
    let mut chance = value(world, 49530, 1);
    for (id, index) in [(276023, 1), (207269, 0)] {
        if combat::auras::has_aura(world, caster, id) {
            chance *= 1.0 + value(world, id, index);
        }
    }
    let constant = combat::random::accumulated_proc_constant(chance);
    if world.get::<SuddenDoomAttempts>(caster).is_none() {
        world
            .entity_mut(caster)
            .insert(SuddenDoomAttempts::default());
    }
    let mut attempts = world.get_mut::<SuddenDoomAttempts>(caster).unwrap();
    attempts.0 += 1;
    if combat::random::random::<f64>() >= constant * f64::from(attempts.0) {
        return;
    }
    attempts.0 = 0;
    combat::auras::apply_aura(
        world,
        &combat::auras::AuraApplication {
            caster,
            target: caster,
            spell_id: 81340,
            effect_mask: 15,
            source: combat::auras::AuraSource::Cast,
        },
    );
}
