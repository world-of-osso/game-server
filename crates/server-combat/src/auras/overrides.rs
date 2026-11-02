//! TC a352b1fa Unit::GetCastSpellInfo: aura332 matches MiscValue when nonzero,
//! otherwise SpellClassSet/mask; the amount is the replacement ID. Never mutate
//! KnownSpells or persisted action slots: removal/expiry restores them naturally.
use super::handlers::OVERRIDE_ACTIONBAR_SPELLS;
use super::{ActiveAuras, AuraEffectState, AuraInstance};
use crate::spell_info::{SpellInfo, SpellStore};
use bevy::prelude::*;

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

/// The caster's action-bar override effects in aura order, the order `resolve`
/// tries them. Collect once to resolve many spells against the same auras.
pub(crate) struct ActionOverrides<'w> {
    world: &'w World,
    effects: Vec<(&'w AuraInstance, &'w AuraEffectState)>,
}

impl<'w> ActionOverrides<'w> {
    pub(crate) fn of(world: &'w World, caster: Entity) -> Self {
        let effects = world
            .get::<ActiveAuras>(caster)
            .map_or_else(Vec::new, |auras| {
                auras
                    .iter()
                    .flat_map(|aura| aura.effects.iter().map(move |effect| (aura, effect)))
                    .filter(|(_, effect)| effect.aura_type == OVERRIDE_ACTIONBAR_SPELLS)
                    .collect()
            });
        Self { world, effects }
    }

    pub(crate) fn resolve(&self, spell_id: u32) -> u32 {
        if self.effects.is_empty() {
            return spell_id;
        }
        let store = self.world.resource::<SpellStore>();
        let mut visited = vec![spell_id];
        let mut id = spell_id;
        while let Some(spell) = store.get(id) {
            let replacement = self.effects.iter().find_map(|&(aura, effect)| {
                let candidate = effect.amount as u32;
                (affects(aura, effect, spell)
                    && !visited.contains(&candidate)
                    && store.get(candidate).is_some())
                .then_some(candidate)
            });
            let Some(replacement) = replacement else {
                break;
            };
            visited.push(replacement);
            id = replacement;
        }
        id
    }
}

pub fn resolve_spell(world: &World, caster: Entity, spell_id: u32) -> u32 {
    ActionOverrides::of(world, caster).resolve(spell_id)
}

/// Some spell in `known` resolves to `spell_id` through the caster's overrides.
pub(crate) fn resolves_known(
    world: &World,
    caster: Entity,
    known: &crate::class_progression::KnownSpells,
    spell_id: u32,
) -> bool {
    let overrides = ActionOverrides::of(world, caster);
    if overrides.effects.is_empty() {
        return known.contains(spell_id);
    }
    known.iter().any(|base| overrides.resolve(base) == spell_id)
}
