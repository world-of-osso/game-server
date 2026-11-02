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
const ENVELOPING_MIST: u32 = 124682;
const SOOTHING_MIST: u32 = 115175;
const ENVELOPING_DIRECT_HEAL: u32 = 274062;
const JADEFIRE_STOMP: u32 = 1248815;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterResourceValues(after_cast)];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    match spell {
        RISING_SUN_KICK => Some(schedule_jadefire),
        ENVELOPING_MIST => Some(empower_enveloping),
        _ => None,
    }
}

fn empower_enveloping(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !combat::auras::has_aura(world, caster, TEA) {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        warn!(?caster, "empowered Enveloping Mist has no target");
        return;
    };
    if !combat::spell_cast::try_cast_triggered(
        world,
        caster,
        ENVELOPING_DIRECT_HEAL,
        Some(target),
        1,
    ) {
        warn!(?caster, ?target, "Thunder Focus Tea direct heal failed");
    }
    start_soothing(world, caster, target);
}

fn start_soothing(world: &mut World, caster: Entity, target: Entity) {
    if !combat::auras::has_aura(world, caster, ELIXIR)
        || !combat::auras::has_aura(world, caster, TEA)
        || world.get::<shared::casting::CastState>(caster).is_some()
    {
        return;
    }
    if !combat::spell_cast::start_triggered_channel(world, caster, SOOTHING_MIST, target) {
        warn!(
            ?caster,
            ?target,
            "empowered Enveloping Mist could not start Soothing Mist"
        );
    }
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
