//! SimC76f8118 consumable.cpp500–565: prepot ages its real buff and native cooldown.
use super::SpellHistory;
use bevy::prelude::*;

pub fn age_precombat_item_use(
    world: &mut World,
    caster: Entity,
    spell: u32,
    seconds: f64,
) -> Result<(), String> {
    let elapsed = crate::auras::precombat::age(world, caster, spell, seconds)?;
    let mut history = world
        .get_mut::<SpellHistory>(caster)
        .ok_or("successful precombat item use lacks native history")?;
    let category = history
        .item_cooldown_sources
        .get(&spell)
        .ok_or("precombat item use lacks native cooldown source")?
        .category;
    if let Some(deadline) = history.spell_ready_at.get_mut(&spell) {
        *deadline -= elapsed;
    }
    if let Some(deadline) = history.category_ready_at.get_mut(&category) {
        *deadline -= elapsed;
    }
    if let Some(cast) = history.successful_casts.get_mut(&spell) {
        cast.at -= elapsed;
    }
    Ok(())
}
