//! 70448 Resurgence refunds mana for direct critical heals.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook};

const PASSIVE: u32 = 16_196;
const ENERGIZE: u32 = 101_033;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedHeal(refund)];

fn refund(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    _target: Entity,
    raw: f32,
    _effective: f32,
    critical: bool,
    periodic: bool,
) {
    if !critical || periodic || raw <= 0.0 || !combat::auras::has_aura(world, ctx.caster, PASSIVE) {
        return;
    }
    // SpellDescriptionVariables 841 uses 16196m1/100 times the source factor.
    let factor = match ctx.spell.id {
        77_472 => 0.8,
        61_295 => 0.48,
        1_064 => 0.2,
        _ => return,
    };
    let percent = combat::spell_cast::scripts::script_effect_value(world, ctx.caster, PASSIVE, 0)
        / 100.0
        * factor;
    combat::spell_cast::cast_triggered_with_effect0_amount(
        world,
        ctx.caster,
        ENERGIZE,
        Some(ctx.caster),
        ctx.depth + 1,
        percent,
    );
}
