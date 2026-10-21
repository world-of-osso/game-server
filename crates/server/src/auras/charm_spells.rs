//! TC a352b1fa CharmInfo.cpp74-165: native creature slots, not pet family spells.
use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use rusqlite::Connection;
use shared::components::{Npc, Player};
use shared::protocol::{
    ACT_COMMAND, ACT_DISABLED, ACT_ENABLED, ACT_PASSIVE, COMMAND_ATTACK, PET_ACTION_BAR_SLOTS,
    pet_action_button,
};

use crate::spell_info::{SpellInfo, SpellStore};

const CREATURE_SPELL_SLOTS: usize = 8;
const CHARM_SPELL_SLOTS: usize = 4;
/// SPELL_ATTR5_NOT_AVAILABLE_WHILE_CHARMED.
const NOT_AVAILABLE_WHILE_CHARMED: u32 = 0x0000_4000;
const NO_AUTOCAST_AI: u32 = 0x0002_0000;
const AUTOCAST_OFF_BY_DEFAULT: u32 = 0x0002_0000;

#[derive(Resource, Default)]
pub(crate) struct CreatureCharmSpells(HashMap<u32, [u32; CREATURE_SPELL_SLOTS]>);

pub(crate) fn load(conn: &Connection) -> rusqlite::Result<CreatureCharmSpells> {
    let mut statement = conn.prepare(
        "SELECT CreatureID, \"Index\", Spell FROM tdb_creature_template_spell ORDER BY CreatureID, \"Index\"",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            row.get::<_, usize>(1)?,
            row.get::<_, u32>(2)?,
        ))
    })?;
    let mut slots = HashMap::new();
    for row in rows {
        let (entry, index, spell) = row?;
        let unit: &mut [u32; CREATURE_SPELL_SLOTS] = slots.entry(entry).or_default();
        let Some(slot) = unit.get_mut(index) else {
            return Err(rusqlite::Error::IntegralValueOutOfRange(1, index as i64));
        };
        *slot = spell;
    }
    Ok(CreatureCharmSpells(slots))
}

pub(super) fn build_bar(
    world: &World,
    target: Entity,
    possessed: bool,
) -> ([u32; PET_ACTION_BAR_SLOTS], Vec<u32>) {
    let empty = possessed || world.get::<Player>(target).is_some();
    let mut bar = if empty {
        empty_bar()
    } else {
        crate::pet_control::pet_action_bar(&[], 0)
    };
    let Some(npc) = world.get::<Npc>(target) else {
        return (bar, Vec::new());
    };
    let source = world.resource::<CreatureCharmSpells>();
    let Some(slots) = source.0.get(&npc.template_id) else {
        warn!(
            "CHARM_CREATURE_SPELL_BAR_SOURCE_ABSENT entry={}",
            npc.template_id
        );
        return (bar, Vec::new());
    };
    let count = if possessed {
        CREATURE_SPELL_SLOTS
    } else {
        CHARM_SPELL_SLOTS
    };
    let passives =
        insert_source_spells(world, npc.template_id, &slots[..count], &mut bar, possessed);
    (bar, passives)
}

fn insert_source_spells(
    world: &World,
    entry: u32,
    slots: &[u32],
    bar: &mut [u32; PET_ACTION_BAR_SLOTS],
    possessed: bool,
) -> Vec<u32> {
    let mut seen = HashSet::new();
    let mut passives = Vec::new();
    for &id in slots {
        if id == 0 || !seen.insert(id) {
            continue;
        }
        let Some(spell) = world.resource::<SpellStore>().get(id) else {
            warn!("CHARM_SOURCE_SPELL_ABSENT_NATIVE70448 entry={entry} spell={id}");
            continue;
        };
        if super::attributes(spell, 5) & NOT_AVAILABLE_WHILE_CHARMED != 0 {
            continue;
        }
        if super::is_passive(spell) {
            passives.push(id);
            continue;
        }
        insert_button(bar, id, button_kind(spell, possessed), possessed);
    }
    passives
}

fn empty_bar() -> [u32; PET_ACTION_BAR_SLOTS] {
    let mut bar = [0; PET_ACTION_BAR_SLOTS];
    bar[0] = pet_action_button(COMMAND_ATTACK, ACT_COMMAND);
    bar
}

fn button_kind(spell: &SpellInfo, possessed: bool) -> u8 {
    if possessed || super::attributes(spell, 1) & NO_AUTOCAST_AI != 0 {
        return ACT_PASSIVE;
    }
    let explicit = crate::spell_cast::targets::explicit_requirement(spell.effects.iter()).is_some();
    if explicit && super::attributes(spell, 9) & AUTOCAST_OFF_BY_DEFAULT == 0 {
        ACT_ENABLED
    } else {
        ACT_DISABLED
    }
}

fn insert_button(bar: &mut [u32; PET_ACTION_BAR_SLOTS], spell: u32, kind: u8, possessed: bool) {
    let slots = if possessed {
        1..PET_ACTION_BAR_SLOTS
    } else {
        3..7
    };
    if let Some(slot) = slots.into_iter().find(|&slot| bar[slot] == 0) {
        bar[slot] = pet_action_button(spell, kind);
    }
}

pub(super) fn cast_passives(world: &mut World, target: Entity, spells: &[u32]) {
    for &spell in spells {
        if let Err(failure) =
            crate::spell_cast::creature_cast(world, target, spell, Some(target), true)
        {
            warn!("CHARM_NATIVE_PASSIVE_CAST_FAILED unit={target} spell={spell}: {failure:?}");
        }
    }
}
