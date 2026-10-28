//! Vengeance MID2 four-piece bonus on targets affected by Sigil of Flame.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ActualDamageMultiplier(damage)];

fn damage(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) -> f32 {
    if !matches!(ctx.spell.id, 258921 | 258922 | 389860)
        || !combat::auras::has_aura(world, ctx.caster, 1296614)
    {
        return 1.0;
    }
    let affected = world
        .get::<combat::auras::ActiveAuras>(target)
        .is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == 204598 && aura.caster == ctx.caster)
        });
    if !affected {
        return 1.0;
    }
    // SimC76f8118 sc_demon_hunter.cpp4679–4686,5429–5436.
    1.0 + spell_cast::scripts::script_effect_value(world, ctx.caster, 1296614, 0) / 100.0
}
