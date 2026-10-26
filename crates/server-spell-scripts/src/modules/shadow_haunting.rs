//! SimC76f8118 sc_priest_shadow.cpp427–435: apparition impacts extend owned DoTs.
use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::spell_cast::script_registry::{ContextHitHook, SpellScriptHook};
use combat::spell_cast::{self, effects::EffectCtx};

const HAUNTING: u32 = 1279355;
const NORMAL_DAMAGE: u32 = 413231;
const VOID_DAMAGE: u32 = 1264176;
const DOTS: &[u32] = &[589, 34914];

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ContextHit(impact_hook),
    SpellScriptHook::ConsumedEffectValues(&[(HAUNTING, 1)]),
];

fn impact_hook(spell: u32) -> Option<ContextHitHook> {
    matches!(spell, NORMAL_DAMAGE | VOID_DAMAGE).then_some(extend_dots)
}

fn extend_dots(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    let payload = ctx.effect.index == 0;
    if !payload || !auras::has_aura(world, ctx.caster, HAUNTING) {
        return;
    }
    let milliseconds = spell_cast::scripts::script_effect_value(world, ctx.caster, HAUNTING, 1);
    auras::extend_owned_duration(world, ctx.caster, target, DOTS, milliseconds as u32);
}
