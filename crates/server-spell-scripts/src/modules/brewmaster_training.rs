//! SimC76f8118 sc_monk.cpp6162–6190: Stagger severity sets Training mastery.
use bevy::prelude::*;
use shared::components::Health;

use super::combat;
use combat::auras::{self, ActiveAuras, AuraInstance};
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::script_effect_value};

const TRAINING: u32 = 383714;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterAuraApplication(after_aura),
    SpellScriptHook::AfterNativeCastUpdate(update),
];

fn after_aura(world: &mut World, target: Entity, aura: &AuraInstance) {
    if aura.spell_id == TRAINING && aura.caster == target {
        refresh(world, target);
    }
}

fn update(world: &mut World) {
    let actors: Vec<_> = world
        .query::<(Entity, &ActiveAuras)>()
        .iter(world)
        .filter(|(actor, auras)| {
            auras
                .iter()
                .any(|aura| aura.spell_id == TRAINING && aura.caster == *actor)
        })
        .map(|(actor, _)| actor)
        .collect();
    for actor in actors {
        refresh(world, actor);
    }
}

fn refresh(world: &mut World, actor: Entity) {
    let pool = auras::stagger::pool_size(world, actor);
    let max_health = world
        .get::<Health>(actor)
        .expect("Training actor health")
        .max;
    // sc_monk.cpp6196–6203 and sc_stagger.hpp668–675: strict pool/max-health thresholds.
    let fraction = pool / max_health;
    let level = if fraction > 0.6 {
        3.0
    } else if fraction > 0.2 {
        2.0
    } else if fraction > 0.0 {
        1.0
    } else {
        0.0
    };
    let amount = script_effect_value(world, actor, TRAINING, 0) * level;
    let existing = world
        .get::<ActiveAuras>(actor)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == TRAINING && aura.caster == actor)
        })
        .map(|aura| (aura.instance_id, aura.effects[0].amount));
    if let Some((instance, old)) = existing
        && old != amount
    {
        assert!(
            auras::set_effect_amount(world, actor, instance, 0, amount),
            "Training effect must exist"
        );
    }
}
