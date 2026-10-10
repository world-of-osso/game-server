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

pub(crate) fn resolve_spell(world: &World, caster: Entity, spell_id: u32) -> u32 {
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
