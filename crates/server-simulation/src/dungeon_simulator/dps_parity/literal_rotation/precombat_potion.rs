//! Literal pre_pot_time uses the same native item cast, aura and cooldown as combat.
use bevy::prelude::*;
use super::execution::Execution;

pub(super) fn execute(world: &mut World, actor: Entity, execution: Execution, age: &str) -> Result<(), String> {
    let seconds = age.parse::<f64>().map_err(|_| format!("invalid pre_pot_time={age}"))?;
    if !seconds.is_finite() { return Err(format!("invalid pre_pot_time={age}")); }
    let Execution::Item(item, target) = execution else {
        return Err("pre_pot_time requires an actual native use item".into());
    };
    crate::spell_cast::cast_item_spell(world, actor, item.on_use.spell_id, Some(target),
        item.on_use.cooldown, item.item_level, Some(item.item_id))
        .map_err(|reason| format!("native precombat item{} rejected: {reason:?}", item.item_id))?;
    // SimC76f8118 consumable.cpp500–565: preserve the later combat use while
    // shortening real deadlines and paying the native elapsed periodic ordinals.
    crate::spell_cast::age_precombat_item_use(world, actor, item.on_use.spell_id, seconds.max(0.0))
}
