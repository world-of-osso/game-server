//! Invoke Chi-Ji's 70448 kick-triggered Gusts.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    self,
    script_registry::SpellScriptHook,
    scripts::{AfterCastHook, script_effect_value},
    targets::{self, Relation},
};
use combat::spell_info::{SpellInfo, SpellStore};

const INVOKE: u32 = 325197;
const PRESENCE: u32 = 343818;
const GUST: u32 = 343819;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::NativeAfterCast(after_cast)];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 100784 | 107428 | 101546).then_some(trigger_gusts)
}

fn trigger_gusts(world: &mut World, caster: Entity, _: &SpellInfo) {
    if !combat::auras::has_aura(world, caster, INVOKE) {
        return;
    }
    let Some(center) = targets::position(world, caster) else {
        return;
    };
    let presence = world.resource::<SpellStore>().get(PRESENCE).expect("Chi-Ji presence missing");
    let radius = targets::radius(&presence.effects[0], 0);
    let mut allies = targets::area_units(world, caster, center, radius, Relation::Ally, 0);
    allies.sort_by(|a, b| {
        let distance = |unit| targets::position(world, unit).unwrap().distance_squared(center);
        distance(*a).total_cmp(&distance(*b)).then_with(|| a.to_bits().cmp(&b.to_bits()))
    });
    let limit = script_effect_value(world, caster, PRESENCE, 2).max(0.0) as usize;
    allies.truncate(limit);
    let count = script_effect_value(world, caster, INVOKE, 1).max(0.0) as usize;
    let multiplier = script_effect_value(world, caster, INVOKE, 5) / 100.0;
    for _ in 0..count {
        for &target in &allies {
            if !spell_cast::cast_triggered_with_overrides(
                world, caster, GUST, Some(target), 1,
                spell_cast::TriggeredSpellOverrides { healing_multiplier: Some(multiplier), ..Default::default() },
            ) {
                warn!(?caster, ?target, "Chi-Ji Gust failed");
            }
        }
    }
}
