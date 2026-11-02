//! Culling Cyclone's single-target Bladestorm bonus.
use bevy::prelude::*;
use super::combat;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::ActualDamageMultiplier(culling_cyclone)];

fn culling_cyclone(world: &mut World, ctx: &EffectCtx<'_>, _target: Entity) -> f32 {
    const CULLING_CYCLONE: u32 = 444778;
    if !matches!(ctx.spell.id, 50622 | 95738)
        || !combat::auras::has_aura(world, ctx.caster, CULLING_CYCLONE)
    {
        return 1.0;
    }
    // SimC76f8118 sc_warrior.cpp:3202–3207: divide effect1 by the
    // number of active enemies. Single-target parity has one recipient.
    let targets = spell_cast::resolving_targets(world, ctx).expect("Bladestorm tick target list");
    if targets.len() != 1 {
        return 1.0;
    }
    1.0 + spell_cast::scripts::script_effect_value(world, ctx.caster, CULLING_CYCLONE, 0) / 100.0
}
