//! SimC76f8118 sc_rogue.cpp4921–4945: default bugs=1 Kingsbane
//! double-dips direct Mastery and direct/periodic Implacable modifiers.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetContextDamageMultiplier(
    extra_multiplier,
)];
const KINGSBANE: u32 = 385627;
const MASTERY: u32 = 76803;
const IMPLACABLE: u32 = 1265386;

fn read_native_percent(world: &World, caster: Entity, spell: u32, index: u8) -> f32 {
    world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == spell && aura.caster == caster)
        })
        .map_or(0.0, |aura| {
            aura.effects
                .iter()
                .find(|effect| effect.index == index)
                .expect("native Kingsbane modifier retains its sourced effect")
                .amount
        })
}

fn extra_multiplier(world: &World, ctx: &EffectCtx, _target: Entity, periodic: bool) -> f32 {
    if ctx.spell.id != KINGSBANE {
        return 1.0;
    }
    // Native family-mask modifiers already contribute once. The pinned
    // scripted duplicate reads the live native amounts, not a replacement coefficient.
    let index = if periodic { 2 } else { 1 };
    let implacable = 1.0 + read_native_percent(world, ctx.caster, IMPLACABLE, index) / 100.0;
    let mastery = if periodic {
        1.0
    } else {
        1.0 + read_native_percent(world, ctx.caster, MASTERY, 0) / 100.0
    };
    implacable * mastery
}
