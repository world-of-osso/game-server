//! TC a352b1fa AuraEffect::HandleLinkedSummon: aura target casts its trigger
//! on application; removal despawns one owned creature per SUMMON effect.
use bevy::prelude::*;
use shared::components::Npc;

use super::{AuraInstance, handlers};
use crate::spell_cast::summon::{self, SummonedCreature};
use crate::spell_info::{SpellEffect, SpellStore};

pub(crate) fn unsupported_reason(world: &World, effect: &SpellEffect) -> Option<String> {
    let Some(spell) = world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(effect.trigger_spell))
    else {
        return Some(format!(
            "linked summon missing trigger {}",
            effect.trigger_spell
        ));
    };
    if spell
        .misc
        .as_ref()
        .is_some_and(|misc| misc.speed > 0.0 || misc.launch_delay > 0.0)
    {
        return Some("delayed linked summon requires lifecycle script".into());
    }
    let effects: Vec<_> = spell
        .effects
        .iter()
        .filter(|effect| effect.effect == summon::EFFECT_SUMMON)
        .collect();
    if effects.is_empty() {
        return Some("linked summon trigger has no summon effect".into());
    }
    effects
        .into_iter()
        .find_map(|effect| summon::unsupported_reason(world, effect))
}

pub(super) fn on_applied(world: &mut World, unit: Entity, aura: &AuraInstance) {
    let effects = linked_effects(world, aura);
    for effect in effects {
        if let Some(reason) = unsupported_reason(world, &effect) {
            warn!("Linked summon aura {}: {reason}", aura.spell_id);
            continue;
        }
        crate::spell_cast::cast_triggered(world, unit, effect.trigger_spell, Some(unit), 1);
    }
}

fn linked_effects(world: &World, aura: &AuraInstance) -> Vec<SpellEffect> {
    if !aura
        .effects
        .iter()
        .any(|effect| effect.aura_type == handlers::LINKED_SUMMON)
    {
        return Vec::new();
    }
    world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(aura.spell_id))
        .map(|spell| {
            spell
                .effects
                .iter()
                .filter(|effect| {
                    effect.aura == handlers::LINKED_SUMMON
                        && aura
                            .effects
                            .iter()
                            .any(|applied| applied.index == effect.index)
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn on_removed(world: &mut World, unit: Entity, aura: &AuraInstance) {
    for effect in linked_effects(world, aura) {
        let entries: Vec<_> = world
            .get_resource::<SpellStore>()
            .and_then(|store| store.get(effect.trigger_spell))
            .map(|spell| {
                spell
                    .effects
                    .iter()
                    .filter(|effect| effect.effect == summon::EFFECT_SUMMON)
                    .map(|effect| effect.misc_values[0] as u32)
                    .collect()
            })
            .unwrap_or_default();
        for entry in entries {
            let owned = world
                .query::<(Entity, &Npc, &SummonedCreature)>()
                .iter(world)
                .find(|(_, npc, summon)| {
                    npc.template_id == entry
                        && summon.owner == unit
                        && summon.spell_id == effect.trigger_spell
                })
                .map(|(entity, _, _)| entity);
            if let Some(entity) = owned {
                summon::despawn_summon(world, entity);
            }
        }
    }
}
