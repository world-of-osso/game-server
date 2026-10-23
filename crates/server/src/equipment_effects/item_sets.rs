//! TC a352b1fa Item.cpp76–242: native set membership, thresholds and ownership.
use super::*;
use crate::class_progression::{ActiveSpec, TraitLoadout};
use crate::profession::PlayerProfessions;
use rusqlite::Connection;
use std::collections::{BTreeMap, HashMap};

const LEGACY_INACTIVE: u32 = 1;

#[derive(Clone, Debug)]
pub(crate) struct SetSpell {
    pub spell_id: u32,
    pub spec_id: u32,
    pub sub_tree_id: u32,
    pub threshold: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct ItemSet {
    flags: u32,
    skill: u32,
    skill_rank: u16,
    pub spells: Vec<SetSpell>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ItemSets {
    pub membership: HashMap<u32, u32>,
    pub sets: HashMap<u32, ItemSet>,
}

impl ItemSets {
    pub(crate) fn load(conn: &Connection) -> rusqlite::Result<Self> {
        let mut sets = Self::load_sets(conn)?;
        Self::load_set_spells(conn, &mut sets)?;
        let mut query = conn.prepare("SELECT ID,ItemSet FROM item_sparse WHERE ItemSet != 0")?;
        let membership = query
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(Self { membership, sets })
    }

    fn load_set_spells(
        conn: &Connection,
        sets: &mut HashMap<u32, ItemSet>,
    ) -> rusqlite::Result<()> {
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
        for row in rows {
            let (id, spell) = row?;
            let set = sets.get_mut(&id).ok_or_else(|| {
                rusqlite::Error::InvalidParameterName(format!(
                    "ItemSetSpell references absent native ItemSet{id}"
                ))
            })?;
            set.spells.push(spell);
        }
        Ok(())
    }

    fn load_sets(conn: &Connection) -> rusqlite::Result<HashMap<u32, ItemSet>> {
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

    pub(crate) fn equipped_counts(&self, equipped: &EquippedItems) -> BTreeMap<u32, u32> {
        let mut counts = BTreeMap::new();
        for &(_, source, item) in &equipped.items {
            if source == ItemDefinitionSource::Retail {
                if let Some(&set) = self.membership.get(&item) {
                    *counts.entry(set).or_default() += 1;
                }
            }
        }
        counts
    }
}

type ChangedSetOwners = QueryState<
    Entity,
    (
        With<EquippedItems>,
        With<EquippedItemModifiers>,
        Or<(
            Changed<EquippedItems>,
            Changed<ActiveSpec>,
            Changed<TraitLoadout>,
            Changed<PlayerProfessions>,
        )>,
    ),
>;

pub(crate) fn sync_item_sets(world: &mut World, changed: &mut ChangedSetOwners) {
    let owners: Vec<_> = changed.iter(world).collect();
    for unit in owners {
        let desired = read_set_spells(world, unit);
        crate::auras::remove_auras(world, unit, RemoveReason::Unlearned, |aura| {
            let AuraSource::ItemSet(set) = aura.source else {
                return false;
            };
            !desired
                .iter()
                .any(|(id, spell)| *id == set && spell.id == aura.spell_id)
        });
        for (set, spell) in desired {
            super::apply_missing_equipment_aura(world, unit, AuraSource::ItemSet(set), &spell);
        }
    }
}

fn read_set_spells(world: &World, unit: Entity) -> Vec<(u32, SpellInfo)> {
    let equipped = world
        .get::<EquippedItems>(unit)
        .expect("set owner equipment");
    if equipped.items.is_empty() {
        return Vec::new();
    }
    let catalog = &world
        .resource::<crate::item_templates::ItemTemplates>()
        .item_sets;
    let counts = catalog.equipped_counts(equipped);
    counts
        .into_iter()
        .flat_map(|(id, count)| {
            let set = catalog
                .sets
                .get(&id)
                .unwrap_or_else(|| panic!("equipped ItemSet{id} absent"));
            read_eligible_spells(world, unit, set, count)
                .into_iter()
                .map(move |spell| (id, spell))
        })
        .collect()
}

fn read_eligible_spells(world: &World, unit: Entity, set: &ItemSet, count: u32) -> Vec<SpellInfo> {
    if set.flags & LEGACY_INACTIVE != 0 || !skill_allows(world, unit, set) {
        return Vec::new();
    }
    let spec = world.get::<ActiveSpec>(unit).map_or(0, |spec| spec.0);
    let sub_tree = selected_sub_tree(world, unit);
    set.spells
        .iter()
        .filter(|spell| {
            let spec_matches = spell.spec_id == 0 || spell.spec_id == spec;
            let subtree_matches = spell.sub_tree_id == 0 || Some(spell.sub_tree_id) == sub_tree;
            spell.threshold <= count && spec_matches && subtree_matches
        })
        .map(|spell| read_set_spell(world, spell.spell_id))
        .collect()
}

fn skill_allows(world: &World, unit: Entity, set: &ItemSet) -> bool {
    set.skill == 0
        || world
            .get::<PlayerProfessions>(unit)
            .is_some_and(|skills| skills.rank(set.skill) >= set.skill_rank)
}

fn selected_sub_tree(world: &World, unit: Entity) -> Option<u32> {
    let traits = world.get::<TraitLoadout>(unit)?;
    let classes = world.get_resource::<crate::spell_info::ClassSpells>()?;
    traits.selected_sub_tree(classes)
}

fn read_set_spell(world: &World, id: u32) -> SpellInfo {
    world
        .resource::<SpellStore>()
        .get(id)
        .cloned()
        .unwrap_or_else(|| panic!("native ItemSetSpell{id} absent"))
}
