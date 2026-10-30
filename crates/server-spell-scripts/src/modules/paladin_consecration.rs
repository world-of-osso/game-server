//! Native Vision of Sanctity on actual Consecration pulses.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::ActualDamageMultiplier(vision_of_sanctity)];

fn vision_of_sanctity(world: &mut World, ctx: &EffectCtx<'_>, _target: Entity) -> f32 {
    const CONSECRATION_DAMAGE: u32 = 81297;
    const VISION_OF_SANCTITY: u32 = 1245354;
    if ctx.spell.id != CONSECRATION_DAMAGE
        || !combat::auras::has_aura(world, ctx.caster, VISION_OF_SANCTITY)
    {
        return 1.0;
    }
    // SimC76f8118 sc_paladin.cpp343–350: only a single-target pulse gains
    // the selected talent's native effect1 percentage.
    let targets =
        spell_cast::resolving_targets(world, ctx).expect("Consecration pulse target list");
    if targets.len() != 1 {
        return 1.0;
    }
    1.0 + spell_cast::scripts::script_effect_value(world, ctx.caster, VISION_OF_SANCTITY, 0) / 100.0
}
