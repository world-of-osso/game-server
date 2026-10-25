//! Native consumable payloads; named relationships follow pinned SimC76f8118.
use crate::auras::{ActiveAuras, AuraApplication, AuraInstance, AuraSource};
use crate::spell_info::{SpellInfo, SpellStore};
use bevy::prelude::*;

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
    if id == 1294727 {
        // unique_gear_midnight.cpp112–139,6397: Royal Roast uses food coefficient1219179 effect2.
        let coeff = world
            .resource::<SpellStore>()
            .get(1219179)
            .ok_or("native food coefficient absent")?;
        let effect = coeff
            .effects
            .iter()
            .find(|e| e.index == 1)
            .ok_or("native food primary coefficient absent")?;
        let amount = crate::spell_cast::effects::modded_base_points(world, actor, coeff, effect);
        for effect in &mut spell.effects {
            if effect.aura == 29 && effect.misc_values[0] == -2 {
                effect.base_points = amount;
            }
        }
    }
    apply_spell(world, actor, &spell);
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

/// unique_gear_midnight.cpp184–220: only highest positive and lowest negative secondary apply.
pub(crate) fn select_recklessness(world: &mut World, actor: Entity, aura: &AuraInstance) {
    let ratings = world
        .get::<crate::stats::CharacterStats>(actor)
        .expect("potion requires native derived ratings")
        .ratings;
    let stats = [
        (1792, ratings.crit),
        (917504, ratings.haste),
        (33554432, ratings.mastery),
        (1879048192, ratings.versatility),
    ];
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
