//! TC a352b1fa AuraEffect::HandleLinkedSummon: aura target casts its trigger
//! on application; removal despawns one owned creature per SUMMON effect.
use bevy::prelude::*;
use shared::components::Npc;

use super::{AuraEffectState, AuraInstance, handlers};
use crate::spell_cast::summon::{self, SummonedCreature};
use crate::spell_info::SpellStore;

pub(crate) fn unsupported_reason(world: &World, trigger_spell: u32) -> Option<String> {
    let Some(spell) = world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(trigger_spell))
    else {
        return Some(format!("linked summon missing trigger {trigger_spell}"));
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

fn linked_effects(aura: &AuraInstance) -> impl Iterator<Item = &AuraEffectState> {
    aura.effects
        .iter()
        .filter(|effect| effect.aura_type == handlers::LINKED_SUMMON)
}

pub(super) fn on_applied(world: &mut World, unit: Entity, aura: &AuraInstance) {
    for effect in linked_effects(aura) {
        if let Some(reason) = unsupported_reason(world, effect.trigger_spell) {
            warn!("Linked summon aura {}: {reason}", aura.spell_id);
            continue;
        }
        crate::spell_cast::cast_triggered(world, unit, effect.trigger_spell, Some(unit), 1);
    }
}

fn summoned_entries(world: &World, trigger_spell: u32) -> Vec<u32> {
    let Some(spell) = world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(trigger_spell))
    else {
        warn!("Removing linked summon: missing trigger {trigger_spell}");
        return Vec::new();
    };
    spell
        .effects
        .iter()
        .filter(|effect| effect.effect == summon::EFFECT_SUMMON)
        .map(|effect| effect.misc_values[0] as u32)
        .collect()
}

pub(super) fn on_removed(world: &mut World, unit: Entity, aura: &AuraInstance) {
    for effect in linked_effects(aura) {
        for entry in summoned_entries(world, effect.trigger_spell) {
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
