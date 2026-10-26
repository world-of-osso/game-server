//! Arms Execute resolves the caster-owned Fatal Mark through native Fatality.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::auras::{ActiveAuras, RemoveReason};
use combat::spell_cast::effects::EffectCtx;
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::AfterPolicyDirectDamage(resolve_fatality)];

const FATALITY: u32 = 383703;
const FATAL_MARK: u32 = 383704;
const FATAL_DAMAGE: u32 = 383706;
const CANNOT_CRIT: u32 = 0x2000_0000;

fn resolve_fatality(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool) {
    if world
        .get::<ActiveSpec>(ctx.caster)
        .is_none_or(|spec| spec.0 != 71)
        || !combat::auras::has_aura(world, ctx.caster, FATALITY)
    {
        return;
    }
    match ctx.spell.id {
        260798 => cast_mark_damage(world, ctx, target),
        // SimC76f8118 sc_warrior.cpp4420–4423: expire on Fatality impact,
        // not before its snapshotted stack-scaled native damage resolves.
        FATAL_DAMAGE => {
            combat::auras::remove_auras(world, target, RemoveReason::Consumed, |aura| {
                aura.spell_id == FATAL_MARK && aura.caster == ctx.caster
            });
        }
        _ => {}
    }
}

fn cast_mark_damage(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    let stacks = world.get::<ActiveAuras>(target).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == FATAL_MARK && aura.caster == ctx.caster)
            .map(|aura| aura.stacks)
    });
    let Some(stacks) = stacks.filter(|stacks| *stacks > 0) else {
        return;
    };
    let Some(mut payload) =
        combat::spell_cast::load_triggered_spell(world, FATAL_DAMAGE, ctx.depth + 1)
    else {
        warn!(caster = ?ctx.caster, "Native Fatality payload unavailable");
        return;
    };
    // SimC4251–4252/4404–4423: native payload, actual mark-stack
    // multiplier, and may_crit=false. No coefficient or flat damage override.
    payload.cast_damage_multiplier = Some(f32::from(stacks));
    payload
        .misc
        .as_mut()
        .expect("native Fatality spell metadata")
        .attributes[2] |= CANNOT_CRIT;
    if !combat::spell_cast::launch_triggered_spell(
        world,
        ctx.caster,
        &payload,
        Some(target),
        ctx.depth + 1,
    ) {
        warn!(caster = ?ctx.caster, ?target, "Native Fatality cast refused");
    }
}
