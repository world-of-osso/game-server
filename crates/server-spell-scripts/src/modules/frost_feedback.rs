//! Frost eligibility for the shared native merged critical-feedback clock.
use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::spell_cast::{effects::EffectCtx, queued_triggers, script_registry::SpellScriptHook};
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::AfterFireDirectHit(record_feedback)];

const OVERFLOWING_TALENT: u32 = 390218;
const OVERFLOWING: u32 = 394195;
const FROSTBOLT: u32 = 228597;
const GLACIAL_SPIKE: u32 = 228600;

fn record_feedback(world: &mut World, ctx: &EffectCtx, _target: Entity, critical: bool) {
    if !matches!(ctx.spell.id, FROSTBOLT | GLACIAL_SPIKE) {
        return;
    }
    if world
        .get::<ActiveSpec>(ctx.caster)
        .is_none_or(|spec| spec.0 != 64)
        || !auras::has_aura(world, ctx.caster, OVERFLOWING_TALENT)
    {
        return;
    }
    // SimC76f8118 sc_mage.cpp4050/4244 marks these actual Frostbolt/Spike
    // payloads eligible;1897–1898 merges noncrit gains and critical expiration
    // after the hit batch. Native394195 retains Frost's10% per-stack modifiers.
    queued_triggers::schedule_buff_outcome(world, ctx.caster, OVERFLOWING, !critical);
}
