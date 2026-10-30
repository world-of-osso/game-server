//! Avenger's Shield deposits its resolved damage into Refining Fire.
use super::{combat, marksmanship};
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedDamage(after_shield)];

fn after_shield(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool, damage: f32) {
    const TALENT: u32 = 469883;
    const DOT: u32 = 469882;
    if ctx.spell.id != 31935
        || damage <= 0.0
        || !world.get::<ActiveAuras>(ctx.caster).is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == TALENT && aura.caster == ctx.caster)
        })
    {
        return;
    }
    // SimC76f8118 sc_paladin_protection.cpp150–160,239–244: actual Shield
    // damage deposits the talent percentage into a non-critical rolling residual.
    let percent = combat::spell_cast::scripts::script_effect_value(world, ctx.caster, TALENT, 0);
    marksmanship::bank_residual(world, ctx, target, DOT, percent, damage);
}
