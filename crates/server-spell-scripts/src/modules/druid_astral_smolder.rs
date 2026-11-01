//! SimC76f8118 sc_druid.cpp12503–12533: critical damage deposits while in Eclipse.
use super::{combat, marksmanship::bank_residual};
use bevy::prelude::*;
use combat::auras;
use combat::class_progression::KnownSpells;
use combat::spell_cast::{
    effects::EffectCtx, script_registry::SpellScriptHook, scripts::script_effect_value,
};

const TALENT: u32 = 1261565;
const SMOLDER: u32 = 1263250;
const SOLAR: u32 = 48517;
const LUNAR: u32 = 48518;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedDamage(deposit)];

fn deposit(world: &mut World, ctx: &EffectCtx, target: Entity, critical: bool, damage: f32) {
    if !critical
        || damage <= 0.0
        || ctx.spell.id == SMOLDER
        || !world
            .get::<KnownSpells>(ctx.caster)
            .is_some_and(|known| known.contains(TALENT))
        || !(auras::has_aura(world, ctx.caster, SOLAR) || auras::has_aura(world, ctx.caster, LUNAR))
    {
        return;
    }
    let percent = script_effect_value(world, ctx.caster, TALENT, 0);
    bank_residual(world, ctx, target, SMOLDER, percent, damage);
}
