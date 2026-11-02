//! Emperor's Elixir empowers Thunder Focus Tea's Rising Sun Kick.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    effects::EffectCtx, queued_triggers, script_registry::SpellScriptHook, scripts::AfterCastHook,
};
use combat::spell_info::SpellInfo;

const ELIXIR: u32 = 1268807;
const TEA: u32 = 116680;
const RISING_SUN_KICK: u32 = 107428;
const JADEFIRE_STOMP: u32 = 1248815;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterResourceValues(after_cast)];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == RISING_SUN_KICK).then_some(schedule_jadefire)
}

fn schedule_jadefire(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !combat::auras::has_aura(world, caster, ELIXIR)
        || !combat::auras::has_aura(world, caster, TEA)
    {
        return;
    }
    let ctx = EffectCtx {
        caster,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    // INFERRED_WCL: 2,153 +17 kicks have a 306 ms median cast-to-impact delay
    // (5th–95th percentile 292–319 ms). Retire when native timing is known.
    queued_triggers::schedule_full_trigger(world, &ctx, caster, JADEFIRE_STOMP, 300);
}
