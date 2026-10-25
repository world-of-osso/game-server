//! TC a352b1fa spell_druid.cpp271–295/319/1336/1934/2263/2821.
//! Native BeforeCast: switch to the script's form only when not already in it.
use bevy::prelude::*;

use super::CastFailure;
use crate::auras::ActiveAuras;
use crate::auras::shapeshift::current_form;
use crate::spell_info::{SpellInfo, SpellStore};

const BERSERK: u32 = 50334;
const RESET_COOLDOWNS: [u32; 3] = [33917, 77758, 6795];

const CAT: u32 = 768;
const BEAR: u32 = 5487;

fn required_form(spell_id: u32) -> Option<(i32, u32)> {
    match spell_id {
        5215 | 252216 => Some((1, CAT)),
        99 | 50334 | 106898 => Some((5, BEAR)),
        _ => None,
    }
}

pub(super) fn before_cast(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    depth: u8,
) -> Result<(), CastFailure> {
    let Some((form, child)) = required_form(spell.id) else {
        return Ok(());
    };
    if spell.id == BERSERK {
        let store = world.resource::<SpellStore>();
        for id in RESET_COOLDOWNS {
            if store.get(id).is_none() {
                return Err(CastFailure::Unsupported(format!(
                    "native Berserk reset child {id} is missing"
                )));
            }
        }
    }
    let current = world
        .get::<ActiveAuras>(caster)
        .and_then(current_form)
        .map(|(_, form)| form);
    if current == Some(form) {
        return Ok(());
    }
    if !super::try_cast_triggered(world, caster, child, Some(caster), depth + 1) {
        return Err(CastFailure::Unsupported(format!(
            "native Druid form child {child} could not be cast for {}",
            spell.id
        )));
    }
    Ok(())
}

/// TC spell_druid.cpp:330–345, SpellHistory.cpp:647–671/1091–1094.
/// Reset existing spell/category cooldowns, not charges or global cooldowns.
/// The native default update=false does not send an invented reset packet.
pub(super) fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != BERSERK {
        return;
    }
    let now = super::history_now(world);
    let categories: Vec<_> = {
        let store = world.resource::<SpellStore>();
        RESET_COOLDOWNS
            .into_iter()
            .map(|id| {
                let spell = store.get(id).expect("Berserk validated reset child");
                (
                    id,
                    spell
                        .categories
                        .as_ref()
                        .map_or(0, |category| category.category),
                )
            })
            .collect()
    };
    let Some(mut history) = world.get_mut::<super::SpellHistory>(caster) else {
        return;
    };
    for (id, category) in categories {
        if !history
            .spell_ready_at
            .get(&id)
            .is_some_and(|ready| *ready > now)
        {
            continue;
        }
        history.spell_ready_at.remove(&id);
        history.category_ready_at.remove(&category);
        history.category_spell_ids.remove(&category);
        history.item_cooldown_sources.remove(&id);
    }
}
