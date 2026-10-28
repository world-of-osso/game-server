//! Imminent Demise accelerates Bladestorm's earned ticks.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    self, ActiveAuras, AuraInstance, RemoveReason, SpellModOp, scripts::AuraScripts, spell_mods,
};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

const BLADES: &[u32] = &[446035];
const IMMINENT: u32 = 444769;
const SUDDEN_DEATH: u32 = 29725;
const STACKS: u32 = 445606;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::PeriodicTickRate(tick_rate),
    SpellScriptHook::LateAuraRegistration(register),
];

fn selected(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 72)
        && auras::has_aura(world, caster, IMMINENT)
        && auras::has_aura(world, caster, SUDDEN_DEATH)
}

fn tick_rate(world: &World, caster: Entity, _target: Entity, spell: &SpellInfo) -> f32 {
    if !BLADES.contains(&spell.id) || !selected(world, caster) {
        return 1.0;
    }
    let stacks = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == STACKS && aura.caster == caster)
        })
        .map_or(0, |aura| aura.stacks);
    let base = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.duration)
        .expect("native Bladestorm has a duration")
        .duration_ms;
    let duration = spell_mods::modify(world, caster, spell, SpellModOp::Duration, base as f32);
    assert!(
        duration > 0.0,
        "modified Bladestorm duration must be positive"
    );
    // SimC76f8118 sc_warrior.cpp3279–3299: tick time is divided by
    // (base duration seconds + earned stacks) / base duration seconds.
    1.0 + f32::from(stacks) * 1000.0 / duration
}

fn register(scripts: &mut AuraScripts) {
    for &spell in BLADES {
        let script = scripts.0.entry(spell).or_default();
        script.on_apply = Some(start_bladestorm);
        script.on_natural_end = Some(finish_bladestorm);
    }
}

fn start_bladestorm(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if !selected(world, aura.caster) {
        return;
    }
    let haste = world
        .get::<combat::stats::CharacterStats>(aura.caster)
        .map_or(1.0, |stats| 1.0 + stats.haste_pct / 100.0);
    assert!(haste > 0.0, "Bladestorm haste must be positive");
    // SimC76f8118 sc_warrior.cpp3279–3285: the modified duration is hasted,
    // while the extra Imminent Demise ticks fit inside that same duration.
    let duration = (aura.duration_ms as f32 / haste).round() as u32;
    assert!(auras::set_owned_duration(
        world,
        aura.caster,
        unit,
        aura.spell_id,
        duration
    ));
}

fn finish_bladestorm(world: &mut World, _unit: Entity, aura: &AuraInstance) {
    if selected(world, aura.caster) {
        // SimC76f8118 sc_warrior.cpp3344–3349: expire all earned stacks at the last tick.
        auras::remove_auras(world, aura.caster, RemoveReason::Consumed, |buff| {
            buff.spell_id == STACKS && buff.caster == aura.caster
        });
    }
}
