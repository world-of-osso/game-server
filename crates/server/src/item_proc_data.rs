//! Native equipped-set predicates and item-description role multipliers.
//! Data only: this does not activate unrelated set bonuses or evaluate arbitrary tooltips.
use bevy::prelude::*;
use rusqlite::Connection;
use shared::item_data::ItemDefinitionSource;
use std::collections::HashMap;

struct ItemSet {
    items: Vec<u32>,
    flags: u32,
    required_skill: u32,
    required_skill_rank: u32,
}

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
    variables: HashMap<u32, String>,
    spell_variables: HashMap<u32, u32>,
}

pub fn load(conn: &Connection) -> rusqlite::Result<ItemProcInputs> {
    let item_columns = (0..17)
        .map(|index| format!("ItemID_{index}"))
        .collect::<Vec<_>>()
        .join(",");
    let mut statement = conn.prepare(&format!(
        "SELECT ID,SetFlags,RequiredSkill,RequiredSkillRank,{item_columns} FROM item_set"
    ))?;
    let sets = statement
        .query_map([], |row| {
            let items = (4..21)
                .map(|index| row.get::<_, u32>(index))
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok((
                row.get(0)?,
                ItemSet {
                    items: items.into_iter().filter(|&id| id != 0).collect(),
                    flags: row.get(1)?,
                    required_skill: row.get(2)?,
                    required_skill_rank: row.get(3)?,
                },
            ))
        })?
        .collect::<rusqlite::Result<HashMap<_, _>>>()?;
    let mut statement = conn.prepare(
        "SELECT SpellID,ItemSetID,ChrSpecID,TraitSubTreeID,Threshold FROM item_set_spell ORDER BY ID")?;
    let mut bonuses: HashMap<u32, Vec<SetBonus>> = HashMap::new();
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
            let set = self.sets.get(&bonus.set).ok_or_else(|| {
                format!("native bonus{spell} references missing set{}", bonus.set)
            })?;
            let count = equipped
                .items
                .iter()
                .filter(|(_, source, item)| {
                    *source == ItemDefinitionSource::Retail && set.items.contains(item)
                })
                .count();
            if count == 0 || count < bonus.threshold {
                continue;
            }
            if set.flags != 0
                || set.required_skill != 0
                || set.required_skill_rank != 0
                || bonus.subtree != 0
            {
                return Err(format!(
                    "native set{} bonus{spell} has unimplemented flag/skill/subtree predicates",
                    bonus.set
                ));
            }
            return Ok(true);
        }
        Ok(false)
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

pub(crate) fn has_equipped_bonus(world: &World, unit: Entity, spell: u32) -> Result<bool, String> {
    let inputs = world
        .get_resource::<ItemProcInputs>()
        .ok_or("native item proc inputs not initialized")?;
    let equipped = world.get::<crate::stats::EquippedItems>(unit);
    let spec = world
        .get::<crate::class_progression::ActiveSpec>(unit)
        .map(|spec| spec.0);
    inputs.has_set_bonus(equipped, spec, spell)
}

pub(crate) fn role_multiplier(world: &World, unit: Entity, driver: u32) -> Result<f32, String> {
    world
        .get_resource::<ItemProcInputs>()
        .ok_or("native item proc inputs not initialized")?
        .role_multiplier(driver, |spell| crate::auras::has_aura(world, unit, spell))
}
