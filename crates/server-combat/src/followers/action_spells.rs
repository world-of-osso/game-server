//! Resolve available class actions and selected equipped on-use spells.
use bevy::prelude::*;
use std::collections::BTreeSet;

use super::FollowerActionSpells;
use crate::spell_cast::CastFailure;

pub(super) fn available(world: &World, unit: Entity) -> Result<Vec<u32>, String> {
    let actions = &world
        .get::<FollowerActionSpells>(unit)
        .ok_or_else(|| format!("follower {unit} action metadata unavailable"))?
        .0;
    let items = equipped_on_use_spells(world, unit)?;
    let mut spells = BTreeSet::new();
    for &action in actions {
        if items.contains(&action) {
            spells.insert(action);
        }
        match crate::spell_cast::knowledge::resolve_actor_action(world, unit, action) {
            Ok(spell) => {
                spells.insert(spell);
            }
            Err(CastFailure::UnknownSpell | CastFailure::NotKnown) => {}
            Err(error) => return Err(format!("follower {unit} action {action}: {error:?}")),
        }
    }
    Ok(spells.into_iter().collect())
}

fn equipped_on_use_spells(world: &World, unit: Entity) -> Result<BTreeSet<u32>, String> {
    let Some(items) = world.get::<crate::stats::EquippedItems>(unit) else {
        return Ok(BTreeSet::new());
    };
    if items.items.is_empty() {
        return Ok(BTreeSet::new());
    }
    let templates = world
        .get_resource::<crate::item_templates::ItemTemplates>()
        .ok_or("follower item templates unavailable")?;
    let mut spells = BTreeSet::new();
    for &(_, source, item_id) in &items.items {
        let template = templates
            .get(source, item_id)
            .ok_or_else(|| format!("follower equipped item {item_id} unavailable"))?;
        if let Some(on_use) = template.on_use {
            spells.insert(on_use.spell_id);
        }
    }
    Ok(spells)
}
