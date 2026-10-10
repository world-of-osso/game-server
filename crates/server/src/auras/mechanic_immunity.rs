//! Mechanic immunity is keyed by the catalog's spell and effect mechanics.
use bevy::prelude::*;

use super::{ActiveAuras, AuraInstance, RemoveReason, handlers};
use crate::spell_info::{SpellInfo, SpellStore};

const ATTR0_NO_IMMUNITIES: u32 = 0x2000_0000;

fn matches_mechanic(spell: &SpellInfo, effect_mechanic: u32, mechanic: i32) -> bool {
    mechanic > 0
        && (effect_mechanic == mechanic as u32
            || spell
                .categories
                .as_ref()
                .is_some_and(|categories| categories.mechanic == mechanic as u32))
}

/// Pass zero for category-only hit rejection; effect hits also supply their mechanic.
pub(crate) fn blocks_mechanic(
    world: &World,
    target: Entity,
    spell: &SpellInfo,
    effect_mechanic: u32,
) -> bool {
    if super::attributes(spell, 0) & ATTR0_NO_IMMUNITIES != 0 {
        return false;
    }
    world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras.iter().flat_map(|aura| &aura.effects).any(|immunity| {
            immunity.aura_type == handlers::MECHANIC_IMMUNITY
                && matches_mechanic(spell, effect_mechanic, immunity.misc_values[0])
        })
    })
}

/// Remove the whole existing aura when any applied effect matches, preserving
/// normal removal hooks, combat logs and stat reconciliation.
pub(super) fn remove_matching(world: &mut World, target: Entity, immunity: &AuraInstance) {
    let mechanics: Vec<i32> = immunity
        .effects
        .iter()
        .filter(|effect| effect.aura_type == handlers::MECHANIC_IMMUNITY)
        .map(|effect| effect.misc_values[0])
        .collect();
    if mechanics.is_empty() {
        return;
    }
    let removed: Vec<u32> = world
        .get::<ActiveAuras>(target)
        .into_iter()
        .flat_map(ActiveAuras::iter)
        .filter(|aura| aura.spell_id != immunity.spell_id)
        .filter(|aura| aura_matches(world, aura, &mechanics))
        .map(|aura| aura.instance_id)
        .collect();
    super::remove_auras(world, target, RemoveReason::Dispelled, |aura| {
        removed.contains(&aura.instance_id)
    });
}

fn aura_matches(world: &World, aura: &AuraInstance, mechanics: &[i32]) -> bool {
    let Some(spell) = world.resource::<SpellStore>().get(aura.spell_id) else {
        return false;
    };
    spell.effects.iter().any(|effect| {
        let applied = aura
            .effects
            .iter()
            .any(|stored| stored.index == effect.index);
        applied
            && mechanics
                .iter()
                .any(|&mechanic| matches_mechanic(spell, effect.mechanic, mechanic))
    })
}
