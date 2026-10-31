//! Mechanic immunity is keyed by the catalog's spell and effect mechanics.
use bevy::prelude::*;

use super::{ActiveAuras, AuraEffectState, AuraInstance, RemoveReason, handlers};
use crate::spell_info::{SpellInfo, SpellStore};

const ATTR0_NO_IMMUNITIES: u32 = 0x2000_0000;

/// Mechanics that never land on encounter bosses in retail logs, while they land on trash
/// and on adds inside the same boss pulls: disoriented 2, fear 5, root 7, sleep 10,
/// stun 12, freeze 13, knockout 14, polymorph 17, sapped 30. Snares, bleeds and taunts
/// do land. Evidence: docs/wiki/systems/encounter-boss-immunity.md.
const ENCOUNTER_BOSS_MECHANICS: u64 =
    1 << 2 | 1 << 5 | 1 << 7 | 1 << 10 | 1 << 12 | 1 << 13 | 1 << 14 | 1 << 17 | 1 << 30;

/// A unit bound to a DungeonEncounter through the encounter-state catalog carries the
/// retail boss immunity mask; every other unit has none from this source.
fn encounter_boss_mask(world: &World, target: Entity) -> u64 {
    let (Some(map), Some(npc)) = (
        world.get::<crate::maps::InMap>(target),
        world.get::<shared::components::Npc>(target),
    ) else {
        return 0;
    };
    let bound = world
        .get_resource::<crate::maps::MapStore>()
        .is_some_and(|store| {
            store
                .raid_creature_encounters
                .contains_key(&(map.map_id, npc.template_id))
        });
    if bound { ENCOUNTER_BOSS_MECHANICS } else { 0 }
}

fn matches_mechanic(spell: &SpellInfo, effect_mechanic: u32, mechanic: i32) -> bool {
    mechanic > 0
        && (effect_mechanic == mechanic as u32
            || spell
                .categories
                .as_ref()
                .is_some_and(|categories| categories.mechanic == mechanic as u32))
}

fn mask_matches(spell: &SpellInfo, effect_mechanic: u32, mask: u64) -> bool {
    let category = spell
        .categories
        .as_ref()
        .map_or(0, |categories| categories.mechanic);
    [category, effect_mechanic]
        .into_iter()
        .any(|mechanic| mechanic > 0 && mechanic < u64::BITS && mask & (1u64 << mechanic) != 0)
}

fn effect_matches(spell: &SpellInfo, mechanic: u32, immunity: &AuraEffectState) -> bool {
    match immunity.aura_type {
        handlers::MECHANIC_IMMUNITY => matches_mechanic(spell, mechanic, immunity.misc_values[0]),
        handlers::MECHANIC_IMMUNITY_MASK => immunity
            .immunity
            .as_ref()
            .is_some_and(|definition| mask_matches(spell, mechanic, definition.mechanics_mask)),
        _ => false,
    }
}

/// The unit's actual mechanic-immunity registrations, independent of an incoming spell.
pub(crate) fn has_mechanic_immunity(world: &World, target: Entity, mechanic: u32) -> bool {
    if mechanic == 0 || mechanic >= u64::BITS {
        return false;
    }
    if encounter_boss_mask(world, target) & (1u64 << mechanic) != 0 {
        return true;
    }
    world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras
            .iter()
            .flat_map(|aura| &aura.effects)
            .any(|effect| match effect.aura_type {
                handlers::MECHANIC_IMMUNITY => u32::try_from(effect.misc_values[0]) == Ok(mechanic),
                handlers::MECHANIC_IMMUNITY_MASK => effect
                    .immunity
                    .as_ref()
                    .is_some_and(|definition| definition.mechanics_mask & (1u64 << mechanic) != 0),
                _ => false,
            })
    })
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
    if mask_matches(spell, effect_mechanic, encounter_boss_mask(world, target)) {
        return true;
    }
    world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras
            .iter()
            .flat_map(|aura| &aura.effects)
            .any(|immunity| effect_matches(spell, effect_mechanic, immunity))
    })
}

/// Remove the whole existing aura when any applied effect matches, preserving
/// normal removal hooks, combat logs and stat reconciliation.
pub(super) fn remove_matching(world: &mut World, target: Entity, immunity: &AuraInstance) {
    const IMMUNITY_PURGES_EFFECT: u32 = 0x0000_8000;
    let purges = world
        .resource::<SpellStore>()
        .get(immunity.spell_id)
        .is_some_and(|spell| super::attributes(spell, 1) & IMMUNITY_PURGES_EFFECT != 0);
    let mechanics: Vec<&AuraEffectState> = immunity
        .effects
        .iter()
        .filter(|effect| {
            effect.aura_type == handlers::MECHANIC_IMMUNITY
                || effect.aura_type == handlers::MECHANIC_IMMUNITY_MASK && purges
        })
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

fn aura_matches(world: &World, aura: &AuraInstance, mechanics: &[&AuraEffectState]) -> bool {
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
                .any(|immunity| effect_matches(spell, effect.mechanic, immunity))
    })
}
