//! Sole native ItemSet catalog predicates; original ItemTemplate membership owns counts.
use super::ItemProcInputs;
use crate::class_progression::{ActiveSpec, TraitLoadout};
use crate::item_templates::ItemTemplates;
use crate::profession::PlayerProfessions;
use crate::stats::EquippedItems;
use bevy::prelude::*;
use rusqlite::Connection;
use shared::item_data::ItemDefinitionSource;
use std::collections::{BTreeMap, BTreeSet, HashMap};

const LEGACY_INACTIVE: u32 = 1;

#[derive(Clone, Debug)]
pub struct SetSpell {
    pub spell_id: u32,
    pub spec_id: u32,
    pub sub_tree_id: u32,
    pub threshold: u32,
}

#[derive(Clone, Debug)]
pub struct ItemSet {
    flags: u32,
    skill: u32,
    skill_rank: u16,
    pub spells: Vec<SetSpell>,
}

pub(super) fn load_sets(conn: &Connection) -> rusqlite::Result<HashMap<u32, ItemSet>> {
    let mut query =
        conn.prepare("SELECT ID,SetFlags,RequiredSkill,RequiredSkillRank FROM item_set")?;
    query
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                ItemSet {
                    flags: row.get(1)?,
                    skill: row.get(2)?,
                    skill_rank: row.get(3)?,
                    spells: Vec::new(),
                },
            ))
        })?
        .collect()
}

pub(super) fn load_spells(
    conn: &Connection,
    sets: &mut HashMap<u32, ItemSet>,
) -> rusqlite::Result<HashMap<u32, BTreeSet<u32>>> {
    let mut query = conn.prepare("SELECT ItemSetID,SpellID,ChrSpecID,TraitSubTreeID,Threshold FROM item_set_spell ORDER BY ID")?;
    let rows = query.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            SetSpell {
                spell_id: row.get(1)?,
                spec_id: row.get(2)?,
                sub_tree_id: row.get(3)?,
                threshold: row.get(4)?,
            },
        ))
    })?;
    let mut index: HashMap<u32, BTreeSet<u32>> = HashMap::new();
    for row in rows {
        let (id, spell) = row?;
        let set = sets.get_mut(&id).ok_or_else(|| {
            rusqlite::Error::InvalidParameterName(format!(
                "ItemSetSpell references absent native ItemSet{id}"
            ))
        })?;
        index.entry(spell.spell_id).or_default().insert(id);
        set.spells.push(spell);
    }
    Ok(index)
}

pub fn equipped_counts(world: &World, unit: Entity) -> Result<BTreeMap<u32, u32>, String> {
    let equipped = world
        .get::<EquippedItems>(unit)
        .ok_or("native equipped identities missing for set count")?;
    let templates = world
        .get_resource::<ItemTemplates>()
        .ok_or("native item templates missing for set count")?;
    let mut counts = BTreeMap::new();
    for &(_, source, item) in &equipped.items {
        if source != ItemDefinitionSource::Retail {
            continue;
        }
        let template = templates
            .get(source, item)
            .ok_or_else(|| format!("native equipped item{item} absent for set count"))?;
        if template.item_set != 0 {
            *counts.entry(template.item_set).or_default() += 1;
        }
    }
    Ok(counts)
}

impl ItemProcInputs {
    pub(super) fn has_set_bonus(
        &self,
        world: &World,
        unit: Entity,
        spell: u32,
    ) -> Result<bool, String> {
        let set_ids = self
            .bonus_sets
            .get(&spell)
            .ok_or_else(|| format!("native set bonus{spell} has no ItemSetSpell row"))?;
        let counts = equipped_counts(world, unit)?;
        for id in set_ids {
            let set = self.sets.get(id).expect("loaded set-spell index");
            let count = counts.get(id).copied().unwrap_or(0);
            if set
                .spells
                .iter()
                .any(|bonus| bonus.spell_id == spell && set.eligible(world, unit, bonus, count))
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(crate) fn active_set_spells(
        &self,
        world: &World,
        unit: Entity,
        counts: &BTreeMap<u32, u32>,
    ) -> Result<Vec<(u32, u32)>, String> {
        let mut active = Vec::new();
        for (&id, &count) in counts {
            let set = self
                .sets
                .get(&id)
                .ok_or_else(|| format!("equipped native ItemSet{id} absent"))?;
            active.extend(
                set.spells
                    .iter()
                    .filter(|bonus| set.eligible(world, unit, bonus, count))
                    .map(|bonus| (id, bonus.spell_id)),
            );
        }
        Ok(active)
    }
}

impl ItemSet {
    fn eligible(&self, world: &World, unit: Entity, bonus: &SetSpell, count: u32) -> bool {
        let threshold_met = count > 0 && count >= bonus.threshold;
        if !threshold_met {
            return false;
        }
        let active = self.flags & LEGACY_INACTIVE == 0 && self.skill_allows(world, unit);
        if !active {
            return false;
        }
        let spec = world.get::<ActiveSpec>(unit).map(|spec| spec.0);
        let spec_matches = bonus.spec_id == 0 || spec == Some(bonus.spec_id);
        let subtree_matches =
            bonus.sub_tree_id == 0 || selected_sub_tree(world, unit) == Some(bonus.sub_tree_id);
        spec_matches && subtree_matches
    }

    fn skill_allows(&self, world: &World, unit: Entity) -> bool {
        self.skill == 0
            || world
                .get::<PlayerProfessions>(unit)
                .is_some_and(|skills| skills.rank(self.skill) >= self.skill_rank)
    }
}

fn selected_sub_tree(world: &World, unit: Entity) -> Option<u32> {
    let traits = world.get::<TraitLoadout>(unit)?;
    let classes = world.get_resource::<crate::spell_info::ClassSpells>()?;
    traits.selected_sub_tree(classes)
}
