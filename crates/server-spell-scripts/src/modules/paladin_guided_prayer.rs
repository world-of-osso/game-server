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
    // INFERRED_WCL: 2gBLy8vcfWAHQ7t3/25/1 at 5.696s and
    // vhKYp3J2BcVdQ7fa/2/1 at 11.180s proc above 25% post-hit HP; subtracting
    // that hit again crosses the native threshold. Retire when Blizzard's
    // authoritative proc health calculation becomes available.
    let below_threshold = world.get::<Health>(unit).is_some_and(|health| {
        health.current > 0.0
            && (health.current - event.amount as f32) * 100.0 < health.max * threshold
    });
    if !below_threshold || !procs::policy_proc_ready(world, unit, instance) {
        return;
    }
    let spell = match spell_cast::knowledge::resolve_actor_action(world, unit, WORD_OF_GLORY) {
        Ok(spell) => spell,
        Err(error) => {
            warn!(
                ?unit,
                ?error,
                "Guided Prayer cannot resolve its healing action"
            );
            return;
        }
    };
    if procs::start_policy_proc(world, unit, instance).is_none() {
        return;
    }
    let effectiveness = spell_cast::scripts::script_effect_value(world, unit, TALENT, 1) / 100.0;
    spell_cast::cast_triggered_with_overrides(
        world,
        unit,
        spell,
        Some(unit),
        1,
        TriggeredSpellOverrides {
            healing_multiplier: Some(effectiveness),
            ..Default::default()
        },
    );
}
