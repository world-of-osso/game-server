//! SimC76f8118 sc_demon_hunter.cpp2559–2573: paid base Fury shortens Demon Spikes.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::KnownSpells;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;

const TALENT: u32 = 218612;
const DEMON_SPIKES: u32 = 203720;
// Native70448 Fury-cost Vengeance actions: Soul Cleave, Spirit Bomb, Fel Devastation, Chaos Nova.
const SPENDERS: &[u32] = &[228477, 247454, 212084, 179057];
pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResourceOnlyContext(SPENDERS),
    SpellScriptHook::AfterForegroundSpecial(reduce_from_paid_fury),
];

#[derive(Component, Default)]
struct FuryRemainder(f32);

fn reduce_from_paid_fury(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let base = super::havoc_destiny::paid_base_fury(spell);
    let talented = world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(TALENT));
    if !talented || base == 0.0 {
        return;
    }
    let threshold = combat::spell_cast::scripts::script_effect_value(world, caster, TALENT, 0);
    let milliseconds = combat::spell_cast::scripts::script_effect_value(world, caster, TALENT, 1);
    assert!(
        threshold > 0.0 && milliseconds > 0.0,
        "native Feed the Demon values must be positive"
    );
    let count = accumulate_base_fury(world, caster, base, threshold);
    if count > 0.0 {
        let seconds = f64::from(count * milliseconds) / 1000.0;
        combat::spell_cast::scripts::reduce_cooldown(world, caster, DEMON_SPIKES, seconds);
    }
}

fn accumulate_base_fury(world: &mut World, caster: Entity, base: f32, threshold: f32) -> f32 {
    let mut entity = world.entity_mut(caster);
    let mut remainder = entity.entry::<FuryRemainder>().or_default().into_mut();
    let total = remainder.0 + base;
    let count = (total / threshold).floor();
    remainder.0 = total - count * threshold;
    count
}
