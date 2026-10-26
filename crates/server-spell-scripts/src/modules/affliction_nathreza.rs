//! Selected Shadow of Nathreza uses the actual native Haunt pulse clock.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    self,
    effects::EffectCtx,
    script_registry::{ContextHitHook, SpellScriptHook},
};

const HAUNT: u32 = 48181;
const HAUNT_PULSE_EFFECT: u8 = 4;
const APEX: u32 = 1261984;
const SHADOW_OF_NATHREZA: u32 = 1262710;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ChannelPeriodic(periodic_trigger)];

fn periodic_trigger(spell: u32, index: u8) -> Option<ContextHitHook> {
    (spell == HAUNT && index == HAUNT_PULSE_EFFECT).then_some(cast_shadow)
}

// SimC76f8118 sc_warlock.cpp41–49: selected rank1, native Haunt effect5's
// period and no tick_zero. The scheduler retains its real owner and recipient.
// Wrath's separate RPPM/Desperate Soul producer is not this damage payload.
fn cast_shadow(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    if combat::auras::has_aura(world, ctx.caster, APEX) {
        spell_cast::cast_triggered(
            world,
            ctx.caster,
            SHADOW_OF_NATHREZA,
            Some(target),
            ctx.depth + 1,
        );
        super::affliction_wrath::try_summon(world, ctx.caster);
    }
}
