//! Culling Cyclone's Bladestorm damage bonus.
use bevy::prelude::*;
use super::combat;
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook, targets};

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::ActualDamageMultiplier(culling_cyclone)];

fn culling_cyclone(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) -> f32 {
    const CULLING_CYCLONE: u32 = 444778;
    if !matches!(ctx.spell.id, 50622 | 95738)
        || !combat::auras::has_aura(world, ctx.caster, CULLING_CYCLONE)
    {
        return 1.0;
    }
    // SimC76f8118 sc_warrior.cpp:3202–3207: divide effect1 by the
    // number of active enemies, including the current impact target.
    let other_enemies = combat::combat_refs::active_opponents(world, ctx.caster)
        .filter(|&enemy| {
            enemy != target
                && targets::is_alive(world, enemy)
                && targets::relation_matches(world, ctx.caster, enemy, targets::Relation::Enemy)
        })
        .count();
    let count = other_enemies + 1;
    1.0 + combat::spell_cast::scripts::script_effect_value(world, ctx.caster, CULLING_CYCLONE, 0)
        / (100.0 * count as f32)
}
