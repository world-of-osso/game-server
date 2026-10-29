//! 70448 Earthliving weapon imbue and its native Riptide/direct-heal proc.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    self, effects::EffectCtx, script_registry::SpellScriptHook, scripts::AfterCastHook,
};
use combat::spell_info::SpellInfo;

const IMBUE: u32 = 382_021;
const BUFF: u32 = 382_022;
const HOT: u32 = 382_024;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::NativeAfterCast(after_cast),
    SpellScriptHook::ResolvedHeal(earthliving),
];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == IMBUE).then_some(imbue)
}

fn imbue(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    spell_cast::cast_triggered(world, caster, BUFF, Some(caster), 1);
}

fn earthliving(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    raw: f32,
    _effective: f32,
    _critical: bool,
    periodic: bool,
) {
    if periodic
        || raw <= 0.0
        || !matches!(ctx.spell.id, 61_295 | 77_472 | 8_004 | 1_064)
        || !combat::auras::has_aura(world, ctx.caster, BUFF)
    {
        return;
    }
    let chance = spell_cast::scripts::script_effect_value(world, ctx.caster, BUFF, 1);
    if combat::random::random::<f32>() * 100.0 < chance {
        spell_cast::cast_triggered(world, ctx.caster, HOT, Some(target), ctx.depth + 1);
    }
}
