//! Native consumable payloads; named relationships follow pinned SimC76f8118.
use crate::auras::{ActiveAuras, AuraApplication, AuraInstance, AuraSource};
use crate::spell_info::{SpellInfo, SpellStore};
use bevy::prelude::*;

mod liquid_luster;
pub(crate) use liquid_luster::{
    ROOT as LIQUID_LUSTER, register as register_liquid_luster, tick as tick_liquid_luster,
};

/// Apply a prepared consumable through the ordinary aura and derived-stat paths.
/// The item level is the original native consumable quality, not the character's gear level.
pub fn apply(
    world: &mut World,
    actor: Entity,
    id: u32,
    item_level: Option<u16>,
) -> Result<(), String> {
    let mut spell = world
        .resource::<SpellStore>()
        .get(id)
        .cloned()
        .ok_or_else(|| format!("native consumable spell{id} absent"))?;
    spell.cast_item_level = item_level;
    match id {
        1294727 | 1232585 => set_primary_food_amount(world, actor, &mut spell)?,
        1232091 => set_highest_secondary_food(world, actor, &mut spell)?,
        _ => {}
    }
    apply_spell(world, actor, &spell);
    Ok(())
}

fn read_native_food_amount(world: &World, actor: Entity, index: u8) -> Result<f32, String> {
    const FOOD_COEFFICIENT: u32 = 1219179;
    let coefficient = world
        .resource::<SpellStore>()
        .get(FOOD_COEFFICIENT)
        .ok_or("native food coefficient1219179 absent")?;
    let effect = coefficient
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .ok_or_else(|| format!("native food coefficient1219179 index{index} absent"))?;
    Ok(crate::spell_cast::effects::modded_base_points(
        world,
        actor,
        coefficient,
        effect,
    ))
}

fn set_primary_food_amount(
    world: &World,
    actor: Entity,
    spell: &mut SpellInfo,
) -> Result<(), String> {
    // SimC76f8118 unique_gear_midnight.cpp115–139/6395–6398: effectN2 primary food.
    let amount = read_native_food_amount(world, actor, 1)?;
    for effect in &mut spell.effects {
        if effect.aura == 29 && effect.misc_values[0] == -2 {
            effect.base_points = amount;
        }
    }
    Ok(())
}

fn set_highest_secondary_food(
    world: &World,
    actor: Entity,
    spell: &mut SpellInfo,
) -> Result<(), String> {
    // SimC76f8118 unique_gear_midnight.cpp74–111/6388: snapshot the highest rating;
    // effectN4 is native zero-based index3, not a reference-report stat amount.
    let amount = read_native_food_amount(world, actor, 3)?;
    let ratings = read_secondary_ratings(world, actor);
    let highest = ratings.iter().max_by(|a, b| a.1.total_cmp(&b.1)).unwrap().0;
    for effect in &mut spell.effects {
        if effect.aura == 189 {
            effect.base_points = amount;
            effect.misc_values[0] = highest;
        }
    }
    Ok(())
}

fn apply_spell(world: &mut World, actor: Entity, spell: &SpellInfo) {
    crate::auras::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster: actor,
            target: actor,
            spell_id: spell.id,
            effect_mask: crate::auras::aura_effect_mask(spell),
            source: AuraSource::Cast,
        },
        spell,
    );
}

/// Native secondary rating masks and current values, in SimC stat order.
pub fn read_secondary_ratings(world: &World, actor: Entity) -> [(i32, f32); 4] {
    let ratings = world
        .get::<crate::stats::CharacterStats>(actor)
        .expect("consumable requires native derived ratings")
        .ratings;
    [
        (1792, ratings.crit),
        (917504, ratings.haste),
        (33554432, ratings.mastery),
        (1879048192, ratings.versatility),
    ]
}

/// unique_gear_midnight.cpp184–220: only highest positive and lowest negative secondary apply.
pub(crate) fn select_recklessness(world: &mut World, actor: Entity, aura: &AuraInstance) {
    let stats = read_secondary_ratings(world, actor);
    let highest = stats.iter().max_by(|a, b| a.1.total_cmp(&b.1)).unwrap().0;
    let lowest = stats.iter().min_by(|a, b| a.1.total_cmp(&b.1)).unwrap().0;
    let mut auras = world
        .get_mut::<ActiveAuras>(actor)
        .expect("applied potion aura");
    let live = auras
        .get_mut(aura.instance_id)
        .expect("applied potion instance");
    for effect in &mut live.effects {
        if effect.aura_type == 189
            && effect.misc_values[0]
                != if effect.amount >= 0.0 {
                    highest
                } else {
                    lowest
                }
        {
            effect.amount = 0.0;
        }
    }
}
