//! Guided Prayer heals the Paladin after damage lowers their health.
use bevy::prelude::*;
use shared::components::Health;
use shared::protocol::{CombatLogEvent, CombatLogKind};

use super::combat;
use combat::auras::{ActiveAuras, procs};
use combat::spell_cast::{self, TriggeredSpellOverrides, script_registry::SpellScriptHook};

const TALENT: u32 = 404357;
const WORD_OF_GLORY: u32 = 85673;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::HealthChangeObserved(on_damage),
    SpellScriptHook::OwnsPolicyProc(|spell| spell == TALENT),
];

// SimC 76f8118 sc_paladin.cpp:1314–1322, 4875–4884: after health loss,
// use the talent's health threshold, effectiveness and native internal cooldown.
fn on_damage(world: &mut World, event: &CombatLogEvent) {
    if event.kind != CombatLogKind::Damage || event.amount <= 0 {
        return;
    }
    let Some(unit) = event.target.and_then(Entity::try_from_bits) else {
        return;
    };
    let Some(instance) = world.get::<ActiveAuras>(unit).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == TALENT && aura.caster == unit)
            .map(|aura| aura.instance_id)
    }) else {
        return;
    };
    let threshold = spell_cast::scripts::script_effect_value(world, unit, TALENT, 0);
    let below_threshold = world.get::<Health>(unit).is_some_and(|health| {
        health.current > 0.0 && health.current * 100.0 < health.max * threshold
    });
    if !below_threshold || procs::start_policy_proc(world, unit, instance).is_none() {
        return;
    }
    let effectiveness = spell_cast::scripts::script_effect_value(world, unit, TALENT, 1) / 100.0;
    spell_cast::cast_triggered_with_overrides(
        world,
        unit,
        WORD_OF_GLORY,
        Some(unit),
        1,
        TriggeredSpellOverrides {
            healing_multiplier: Some(effectiveness),
            ..Default::default()
        },
    );
}
