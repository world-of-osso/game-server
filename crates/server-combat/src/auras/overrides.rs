//! TC a352b1fa Unit::GetCastSpellInfo: aura332 matches MiscValue when nonzero,
//! otherwise SpellClassSet/mask; the amount is the replacement ID. Never mutate
//! KnownSpells or persisted action slots: removal/expiry restores them naturally.
use super::handlers::OVERRIDE_ACTIONBAR_SPELLS;
use super::{ActiveAuras, AuraEffectState, AuraInstance};
use crate::spell_info::{SpellInfo, SpellStore};
use bevy::prelude::*;
use std::collections::HashSet;

pub(super) fn affects(aura: &AuraInstance, effect: &AuraEffectState, spell: &SpellInfo) -> bool {
    if effect.misc_values[0] != 0 {
        effect.misc_values[0] as u32 == spell.id
    } else {
        super::spell_mods::class_mask_affects(aura, effect, spell)
    }
}

pub(super) fn view_overrides(
    world: &World,
    unit: Entity,
    aura: &AuraInstance,
) -> Vec<shared::components::AuraOverride> {
    use shared::components::AuraOverride;
    let store = world.resource::<SpellStore>();
    let known: Vec<u32> = world
        .get::<crate::class_progression::KnownSpells>(unit)
        .map_or_else(Vec::new, |known| known.iter().collect());
    aura.effects
        .iter()
        .flat_map(|effect| match effect.aura_type {
            super::handlers::ANIM_REPLACEMENT_SET => {
                vec![AuraOverride::Animation(effect.misc_values[0] as u32)]
            }
            super::handlers::OVERRIDE_SPELLS => {
                let id = effect.misc_values[0] as u32;
                super::override_spell_data::spells(world, id).map_or_else(Vec::new, |spells| {
                    vec![AuraOverride::SpellSet {
                        id,
                        spells: spells.to_vec(),
                    }]
                })
            }
            OVERRIDE_ACTIONBAR_SPELLS => {
                let candidates = if effect.misc_values[0] != 0 {
                    vec![effect.misc_values[0] as u32]
                } else {
                    known.clone()
                };
                candidates
                    .into_iter()
                    .filter_map(|base| {
                        let spell = store.get(base)?;
                        let replacement = effect.amount as u32;
                        (affects(aura, effect, spell) && store.get(replacement).is_some())
                            .then_some(AuraOverride::ActionBar {
                                spell_id: base,
                                replacement,
                            })
                    })
                    .collect()
            }
            _ => Vec::new(),
        })
        .collect()
}

/// Newest override set grants temporary cast permission; never persist KnownSpells.
pub(crate) fn grants_spell(world: &World, caster: Entity, spell_id: u32) -> bool {
    world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .auras
                .iter()
                .rev()
                .flat_map(|aura| &aura.effects)
                .find(|effect| effect.aura_type == super::handlers::OVERRIDE_SPELLS)
        })
        .and_then(|effect| super::override_spell_data::spells(world, effect.misc_values[0] as u32))
        .is_some_and(|spells| spell_id != 0 && spells.contains(&spell_id))
}

pub fn resolve_spell(world: &World, caster: Entity, spell_id: u32) -> u32 {
    let Some(auras) = world.get::<ActiveAuras>(caster) else {
        return spell_id;
    };
    let store = world.resource::<SpellStore>();
    let mut visited = HashSet::from([spell_id]);
    let mut id = spell_id;
    while let Some(spell) = store.get(id) {
        let replacement = auras.iter().find_map(|aura| {
            aura.effects.iter().find_map(|effect| {
                let candidate = effect.amount as u32;
                (effect.aura_type == OVERRIDE_ACTIONBAR_SPELLS
                    && affects(aura, effect, spell)
                    && !visited.contains(&candidate)
                    && store.get(candidate).is_some())
                .then_some(candidate)
            })
        });
        let Some(replacement) = replacement else {
            break;
        };
        visited.insert(replacement);
        id = replacement;
    }
    id
}
