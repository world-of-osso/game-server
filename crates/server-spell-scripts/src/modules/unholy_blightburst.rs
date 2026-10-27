//! Putrefy impacts apply or erupt and extend the owner's diseases.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{self, script_registry::{HitHook, SpellScriptHook}, scripts::script_effect_value};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EffectHit(effect_hit)];

fn effect_hit(spell: u32, index: u8) -> Option<HitHook> {
    match (spell, index) {
        (1277016, 0) => Some(dread_burst),
        (390220, 0) => Some(virulent_burst),
        _ => None,
    }
}

fn dread_burst(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    blightburst(world, caster, target, depth, 1240996, 1241171);
}

fn virulent_burst(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    blightburst(world, caster, target, depth, 191587, 1241167);
}

fn blightburst(world: &mut World, caster: Entity, target: Entity, depth: u8, plague: u32, eruption: u32) {
    if !combat::auras::has_aura(world, caster, 1254552) { return; }
    // SimC76f8118 sc_death_knight.cpp12103–12131/12153–12187: apply a
    // missing disease, or erupt a fraction of its extension and extend it.
    let period = world.get::<ActiveAuras>(target).and_then(|auras| auras.iter()
        .find(|aura| aura.spell_id == plague && aura.caster == caster))
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0 && effect.period_ms > 0))
        .map(|effect| effect.period_ms);
    let Some(period) = period else {
        if !spell_cast::try_cast_triggered(world, caster, plague, Some(target), depth + 1) {
            warn!(?caster, ?target, plague, "native Blightburst disease application refused");
        }
        return;
    };
    let milliseconds = script_effect_value(world, caster, 1254552, 0) as u32;
    let multiplier = script_effect_value(world, caster, 1254552, 1) / 100.0;
    if let Some(tick) = super::unholy_eruption::owned_tick_amount(world, caster, target, plague) {
        spell_cast::cast_triggered_with_effect0_amount(world, caster, eruption, Some(target), depth + 1,
            tick * milliseconds as f32 / period as f32 * multiplier);
    }
    combat::auras::extend_owned_duration(world, caster, target, &[plague], milliseconds);
}
