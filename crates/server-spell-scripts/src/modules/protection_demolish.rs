//! Protection Demolish's native channel ticks and Decimator bleed.
use super::combat;
use bevy::prelude::*;
use combat::auras::AuraInstance;
use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::ResolvedDamage(decimator),
];

fn protection(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 73)
}

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        436358,
        AuraScript {
            on_periodic: Some(demolish_tick),
            ..Default::default()
        },
    );
}

fn demolish_tick(world: &mut World, target: Entity, aura: &AuraInstance, index: u8) {
    if index != 0 || !protection(world, aura.caster) {
        return;
    }
    // SimC76f8118 sc_warrior.cpp 3873–3892: native 100ms channel
    // ticks 1, 3 and 8 execute the three authored damage spells.
    let tick = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Demolish requires its native periodic effect")
        .periodic_ticks();
    let spell = match tick {
        1 => 440884,
        3 => 440886,
        8 => 440888,
        _ => return,
    };
    spell_cast::cast_triggered(world, aura.caster, spell, Some(target), 1);
    if tick == 8 && super::cast_relationships::has_talent(world, aura.caster, 1270710) {
        for buff in [1270843, 1270846] {
            if !spell_cast::try_cast_triggered(world, aura.caster, buff, Some(aura.caster), 1) {
                warn!(?buff, "Celeritous Conclusion buff refused");
            }
        }
    }
}

fn decimator(world: &mut World, ctx: &EffectCtx, target: Entity, _: bool, amount: f32) {
    // SimC76f8118 sc_warrior.cpp 3842–3848: final damage applies
    // native Deep Wounds on each target when Decimator is selected.
    if ctx.spell.id == 440888
        && amount > 0.0
        && protection(world, ctx.caster)
        && combat::auras::has_aura(world, ctx.caster, 1270704)
    {
        spell_cast::cast_triggered(world, ctx.caster, 262115, Some(target), ctx.depth + 1);
    }
}
