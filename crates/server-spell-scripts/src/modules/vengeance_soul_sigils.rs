//! Soul Sigils grants fragments once per successful sigil detonation.
use super::combat;
use bevy::prelude::*;
use combat::soul_fragments::{self, FragmentKind};
use combat::spell_cast::{
    self,
    effects::EffectCtx,
    script_registry::SpellScriptHook,
    targets::{self, Relation},
};

const SOUL_SIGILS: u32 = 395446;
pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterChainResolved(resolve)];

fn resolve(world: &mut World, ctx: &EffectCtx<'_>) {
    if !matches!(ctx.spell.id, 204598 | 389860)
        || !world
            .get::<combat::class_progression::TraitLoadout>(ctx.caster)
            .is_some_and(|traits| {
                traits
                    .active_talent_spells()
                    .any(|spell| spell.spell_id == SOUL_SIGILS)
            })
        || !spell_cast::resolving_damage(world, ctx).is_some_and(|hits| {
            hits.iter().any(|(target, _)| {
                targets::relation_matches(world, ctx.caster, *target, Relation::Enemy)
            })
        })
    {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp:3540-3548 grants once per sigil with a hit.
    let count = spell_cast::scripts::script_effect_value(world, ctx.caster, SOUL_SIGILS, 0);
    if !count.is_finite() || count < 0.0 || count.fract() != 0.0 {
        error!(count, "invalid Soul Sigils fragment count");
        return;
    }
    for _ in 0..count as u32 {
        if let Err(reason) = soul_fragments::spawn(world, ctx.caster, FragmentKind::Lesser, false) {
            error!(%reason, "Soul Sigils fragment spawn failed");
        }
    }
}
