//! Critical Bloodthirst-family impacts grant the selected native Gushing Wound.
use bevy::prelude::*;

use super::combat;
use combat::spell_cast::effects::EffectCtx;
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterPolicyDirectDamage(after_hit)];

const COLD_STEEL: u32 = 383959;
const GUSHING_WOUND: u32 = 385042;

fn after_hit(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, critical: bool) {
    // SimC76f8118 sc_warrior.cpp2679–2682/2915–2919: only critical
    // Bloodthirst or Bloodbath impacts with the selected talent apply the bleed.
    if !critical
        || !matches!(ctx.spell.id, 23881 | 335096)
        || !combat::auras::has_aura(world, ctx.caster, COLD_STEEL)
    {
        return;
    }
    combat::spell_cast::cast_triggered(
        world,
        ctx.caster,
        GUSHING_WOUND,
        Some(target),
        ctx.depth + 1,
    );
}
