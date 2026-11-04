//! Restoration MID2 direct heals can create independent Healing Rain fields.
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, procs};
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook, targets};

const TWO_PIECE: u32 = 1296629;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResolvedHeal(roll_tier_rain),
    SpellScriptHook::OwnsPolicyProc(owns_proc),
];

fn owns_proc(spell: u32) -> bool {
    spell == TWO_PIECE
}

fn roll_tier_rain(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    raw: f32,
    _: f32,
    _: bool,
    periodic: bool,
) {
    if periodic || raw <= 0.0 || !matches!(ctx.spell.id, 77472 | 1064) {
        return;
    }
    let Some(instance) = world.get::<ActiveAuras>(ctx.caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == TWO_PIECE && aura.caster == ctx.caster)
            .map(|aura| aura.instance_id)
    }) else {
        return;
    };
    let Some(center) = targets::position(world, target) else {
        return;
    };
    if procs::try_native_policy_proc(world, ctx.caster, instance) {
        combat::spell_cast::scripts::spawn_restoration_tier_rain(world, ctx.caster, center);
    }
}
