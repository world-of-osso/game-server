//! Native equipped-set predicates and item-description role multipliers.
//! Data selection only; equipment_effects owns activation and removal.
use bevy::prelude::*;
use rusqlite::Connection;
use shared::item_data::ItemDefinitionSource;
use std::collections::{BTreeMap, HashMap};

const ITEM_SET_FLAG_LEGACY_INACTIVE: u32 = 0x01;

struct ItemSet {
    flags: u32,
    required_skill: u32,
    required_skill_rank: u32,
}

#[derive(Clone, Copy)]
struct SetBonus {
    set: u32,
    spec: u32,
    subtree: u32,
    threshold: usize,
}

#[derive(Resource)]
pub struct ItemProcInputs {
    sets: HashMap<u32, ItemSet>,
    bonuses: HashMap<u32, Vec<SetBonus>>,
    bonuses_by_set: HashMap<u32, Vec<(u32, SetBonus)>>,
    item_sets: HashMap<u32, u32>,
    variables: HashMap<u32, String>,
    spell_variables: HashMap<u32, u32>,
}

pub fn load(conn: &Connection) -> rusqlite::Result<ItemProcInputs> {
    let mut statement =
        conn.prepare("SELECT ID,SetFlags,RequiredSkill,RequiredSkillRank FROM item_set")?;
    let sets = statement
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                ItemSet {
                    flags: row.get(1)?,
                    required_skill: row.get(2)?,
                    required_skill_rank: row.get(3)?,
                },
            ))
        })?
        .collect::<rusqlite::Result<HashMap<_, _>>>()?;
    let mut statement = conn.prepare("SELECT ID,ItemSet FROM item_sparse WHERE ItemSet!=0")?;
    let item_sets = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<HashMap<_, _>>>()?;
    let mut statement = conn.prepare(
        "SELECT SpellID,ItemSetID,ChrSpecID,TraitSubTreeID,Threshold FROM item_set_spell ORDER BY ID")?;
    let mut bonuses: HashMap<u32, Vec<SetBonus>> = HashMap::new();
    let mut bonuses_by_set: HashMap<u32, Vec<(u32, SetBonus)>> = HashMap::new();
    for row in statement.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            SetBonus {
                set: row.get(1)?,
                spec: row.get(2)?,
                subtree: row.get(3)?,
                threshold: row.get(4)?,
            },
        ))
    })? {
        let (spell, bonus) = row?;
        bonuses_by_set
            .entry(bonus.set)
            .or_default()
            .push((spell, bonus));
        bonuses.entry(spell).or_default().push(bonus);
    }
    let mut statement = conn.prepare("SELECT ID,Variables FROM spell_description_variables")?;
    let variables = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<HashMap<_, _>>>()?;
    let mut statement = conn
        .prepare("SELECT SpellID,SpellDescriptionVariablesID FROM spell_x_description_variables")?;
    let spell_variables = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<HashMap<_, _>>>()?;
    Ok(ItemProcInputs {
        sets,
        bonuses,
        bonuses_by_set,
        item_sets,
        variables,
        spell_variables,
    })
}

impl ItemProcInputs {
    fn has_set_bonus(
        &self,
        equipped: Option<&crate::stats::EquippedItems>,
        spec: Option<u32>,
        spell: u32,
    ) -> Result<bool, String> {
        let bonuses = self
            .bonuses
            .get(&spell)
            .ok_or_else(|| format!("native set bonus{spell} has no ItemSetSpell row"))?;
        for bonus in bonuses {
            if bonus.spec != 0 && spec != Some(bonus.spec) {
                continue;
            }
            let equipped =
                equipped.ok_or("native equipped identities missing for matching set bonus")?;
            let count = equipped
                .items
                .iter()
                .filter(|(_, source, item)| {
                    *source == ItemDefinitionSource::Retail
                        && self.item_sets.get(item) == Some(&bonus.set)
                })
                .count();
            if self.bonus_applies(bonus, count, spec)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn bonus_applies(
        &self,
        bonus: &SetBonus,
        count: usize,
        spec: Option<u32>,
    ) -> Result<bool, String> {
        if count == 0 || count < bonus.threshold || (bonus.spec != 0 && spec != Some(bonus.spec)) {
            return Ok(false);
        }
        let set = self
            .sets
            .get(&bonus.set)
            .ok_or_else(|| format!("native equipped set{} has no ItemSet row", bonus.set))?;
        if set.flags & ITEM_SET_FLAG_LEGACY_INACTIVE != 0 {
            return Ok(false);
        }
        if set.required_skill != 0 {
            return Err(format!(
                "EQUIPPED_SET_SKILL_REQUIREMENT_UNSUPPORTED: set{} skill{} rank{}",
                bonus.set, set.required_skill, set.required_skill_rank
            ));
        }
        if bonus.subtree != 0 {
            return Err(format!(
                "EQUIPPED_SET_TRAIT_SUBTREE_UNSUPPORTED: set{} subtree{}",
                bonus.set, bonus.subtree
            ));
        }
        Ok(true)
    }

    fn select_set_spells(
        &self,
        bonuses: &[(u32, SetBonus)],
        count: usize,
        spec: Option<u32>,
    ) -> (Vec<(u32, u32)>, Vec<String>) {
        let mut desired = Vec::new();
        let mut errors = Vec::new();
        for (spell, bonus) in bonuses {
            match self.bonus_applies(bonus, count, spec) {
                Ok(true) => desired.push((bonus.set, *spell)),
                Ok(false) => {}
                Err(error) => errors.push(error),
            }
        }
        (desired, errors)
    }

    fn equipped_set_spells(
        &self,
        equipped: &crate::stats::EquippedItems,
        spec: Option<u32>,
    ) -> (Vec<(u32, u32)>, Vec<String>) {
        let mut counts = BTreeMap::<u32, usize>::new();
        for (_, source, item) in &equipped.items {
            if *source == ItemDefinitionSource::Retail
                && let Some(set) = self.item_sets.get(item)
            {
                *counts.entry(*set).or_default() += 1;
            }
        }
        let mut desired = Vec::new();
        let mut errors = Vec::new();
        for (set, count) in counts {
            if !self.sets.contains_key(&set) {
                errors.push(format!("native equipped set{set} has no ItemSet row"));
                continue;
            }
            if let Some(bonuses) = self.bonuses_by_set.get(&set) {
                let (spells, failures) = self.select_set_spells(bonuses, count, spec);
                desired.extend(spells);
                errors.extend(failures);
            }
        }
        (desired, errors)
    }

    fn role_multiplier(&self, driver: u32, has_aura: impl Fn(u32) -> bool) -> Result<f32, String> {
        let variable = self.spell_variables.get(&driver).ok_or_else(|| {
            format!("native item driver{driver} has no description-variable link")
        })?;
        let variables = self.variables.get(variable).ok_or_else(|| {
            format!("native item driver{driver} references missing variable{variable}")
        })?;
        parse_role_multiplier(variables, has_aura)
            .map_err(|error| format!("native item driver{driver}/variable{variable}: {error}"))
    }
}

/// Native884's aura-OR conditional, also parsed by SimC76f8118 unique_gear.cpp5021–5065.
/// Unsupported/missing expressions fail; the false result comes from the source, never a default.
fn parse_role_multiplier(variables: &str, has_aura: impl Fn(u32) -> bool) -> Result<f32, String> {
    let expression = variables
        .lines()
        .find(|line| line.starts_with("$rolemult="))
        .ok_or("missing native role multiplier expression")?;
    let expression = expression
        .strip_prefix("$rolemult=$?")
        .ok_or("unsupported native role multiplier condition")?;
    let (auras, branches) = expression
        .split_once('[')
        .ok_or("native role multiplier lacks conditional branches")?;
    let (yes, no) = branches
        .split_once("][")
        .ok_or("native role multiplier lacks false branch")?;
    let no = no
        .strip_suffix(']')
        .ok_or("native role multiplier has trailing expression")?;
    let yes = parse_role_amount(yes)?;
    let no = parse_role_amount(no)?;
    let mut active = false;
    for aura in auras.split('|') {
        let id = aura
            .strip_prefix('a')
            .ok_or("unsupported native role condition token")?
            .parse::<u32>()
            .map_err(|error| format!("invalid native role aura: {error}"))?;
        active |= has_aura(id);
    }
    Ok(if active { yes } else { no })
}

fn parse_role_amount(expression: &str) -> Result<f32, String> {
    let expression = expression
        .strip_prefix("${")
        .ok_or("unsupported native role amount")?;
    let (amount, format_suffix) = expression
        .split_once('}')
        .ok_or("unterminated native role amount")?;
    if !format_suffix
        .chars()
        .all(|character| character.is_ascii_digit() || character == '.')
    {
        return Err("unsupported native role number format".into());
    }
    let amount = amount
        .parse::<f32>()
        .map_err(|error| format!("invalid native role amount: {error}"))?;
    if !amount.is_finite() {
        return Err("non-finite native role multiplier".into());
    }
    Ok(amount)
}

pub fn has_equipped_bonus(world: &World, unit: Entity, spell: u32) -> Result<bool, String> {
    let inputs = world
        .get_resource::<ItemProcInputs>()
        .ok_or("native item proc inputs not initialized")?;
    let equipped = world.get::<crate::stats::EquippedItems>(unit);
    let spec = world
        .get::<crate::class_progression::ActiveSpec>(unit)
        .map(|spec| spec.0);
    inputs.has_set_bonus(equipped, spec, spell)
}

pub(crate) fn equipped_set_spells(world: &World, unit: Entity) -> Vec<(u32, u32)> {
    let inputs = world.resource::<ItemProcInputs>();
    let equipped = world
        .get::<crate::stats::EquippedItems>(unit)
        .expect("equipped set catalog identities");
    let spec = world
        .get::<crate::class_progression::ActiveSpec>(unit)
        .map(|spec| spec.0);
    let (spells, errors) = inputs.equipped_set_spells(equipped, spec);
    for error in errors {
        error!("Native equipped set activation: {error}");
    }
    spells
}

pub(crate) fn role_multiplier(world: &World, unit: Entity, driver: u32) -> Result<f32, String> {
    world
        .get_resource::<ItemProcInputs>()
        .ok_or("native item proc inputs not initialized")?
        .role_multiplier(driver, |spell| crate::auras::has_aura(world, unit, spell))
}
