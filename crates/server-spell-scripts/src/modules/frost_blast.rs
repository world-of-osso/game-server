//! Icebreaker amplifies an earned Rime Howling Blast primary hit.
use bevy::prelude::*;

use super::combat;
use combat::auras;
use combat::spell_cast::effects::EffectCtx;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::script_effect_value;

const HOWLING_BLAST: u32 = 49184;
const RIME: u32 = 59052;
const ICEBREAKER: u32 = 392950;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetContextDamageMultiplier(
    icebreaker_primary,
)];

fn icebreaker_primary(world: &World, ctx: &EffectCtx, target: Entity, _: bool) -> f32 {
    let earned_rime = ctx
        .spell
        .cast_charge_mods
        .iter()
        .flatten()
        .any(|modifier| modifier.aura_spell_id == RIME);
    if ctx.spell.id != HOWLING_BLAST
        || ctx.primary_target() != Some(target)
        || !earned_rime
        || !auras::has_aura(world, ctx.caster, ICEBREAKER)
    {
        return 1.0;
    }
    // SimC76f8118 sc_death_knight.cpp11545–11551: Icebreaker amplifies
    // the earned Rime primary with the selected native talent amount.
    1.0 + script_effect_value(world, ctx.caster, ICEBREAKER, 0) / 100.0
}
