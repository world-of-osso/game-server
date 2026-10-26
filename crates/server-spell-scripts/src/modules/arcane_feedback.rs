//! Arcane eligibility for the native merged critical-feedback clock.
use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::spell_cast::{effects::EffectCtx, queued_triggers, script_registry::SpellScriptHook};
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::AfterFireDirectHit(record_barrage_outcome)];

const BARRAGE: u32 = 44425;
const OVERFLOWING_TALENT: u32 = 390218;
const OVERFLOWING: u32 = 394195;

fn record_barrage_outcome(world: &mut World, ctx: &EffectCtx, _target: Entity, critical: bool) {
    if ctx.spell.id != BARRAGE {
        return;
    }
    if world
        .get::<ActiveSpec>(ctx.caster)
        .is_none_or(|spec| spec.0 != 62)
        || !auras::has_aura(world, ctx.caster, OVERFLOWING_TALENT)
    {
        return;
    }
    // SimC76f8118 sc_mage.cpp2693/1897–1898: Barrage noncritical hits
    // earn a native stack; critical hits expire it after the impact batch.
    // Native394195 effect2 supplies Arcane's per-stack critical chance.
    queued_triggers::schedule_buff_outcome(world, ctx.caster, OVERFLOWING, !critical);
}
