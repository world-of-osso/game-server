//! Imminent Demise accelerates Bladestorm's earned ticks.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

const BLADES: &[u32] = &[227847, 446035];
const IMMINENT: u32 = 444769;
const SUDDEN_DEATH: u32 = 29725;
const STACKS: u32 = 445606;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::PeriodicTickRate(tick_rate),
    SpellScriptHook::LateAuraRegistration(register),
];

fn selected(world: &World, caster: Entity) -> bool {
    world.get::<ActiveSpec>(caster).is_some_and(|spec| matches!(spec.0, 71 | 72))
        && auras::has_aura(world, caster, IMMINENT)
        && auras::has_aura(world, caster, SUDDEN_DEATH)
}

fn tick_rate(world: &World, caster: Entity, _target: Entity, spell: &SpellInfo) -> f32 {
    if !BLADES.contains(&spell.id) || !selected(world, caster) {
        return 1.0;
    }
    let stacks = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras.iter().find(|aura| aura.spell_id == STACKS && aura.caster == caster)
    }).map_or(0, |aura| aura.stacks);
    let duration = spell.misc.as_ref().and_then(|misc| misc.duration)
        .expect("native Bladestorm has a duration").duration_ms;
    assert!(duration > 0, "native Bladestorm duration must be positive");
    // SimC76f8118 sc_warrior.cpp3279–3299: tick time is divided by
    // (base duration seconds + earned stacks) / base duration seconds.
    1.0 + f32::from(stacks) * 1000.0 / duration as f32
}

fn register(scripts: &mut AuraScripts) {
    for &spell in BLADES {
        scripts.0.entry(spell).or_default().on_natural_end = Some(finish_bladestorm);
    }
}

fn finish_bladestorm(world: &mut World, _unit: Entity, aura: &AuraInstance) {
    if selected(world, aura.caster) {
        // SimC76f8118 sc_warrior.cpp3344–3349: expire all earned stacks at the last tick.
        auras::remove_auras(world, aura.caster, RemoveReason::Consumed, |buff| {
            buff.spell_id == STACKS && buff.caster == aura.caster
        });
    }
}
