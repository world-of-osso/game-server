//! Native equipped-set predicates and item-description role multipliers.
//! Data only: this does not activate unrelated set bonuses or evaluate arbitrary tooltips.
pub mod sets;
#[cfg(test)]
mod tests;
use bevy::prelude::*;
use rusqlite::Connection;
pub use sets::equipped_counts;
use std::collections::{BTreeSet, HashMap};

#[derive(Resource)]
pub struct ItemProcInputs {
    pub sets: HashMap<u32, sets::ItemSet>,
    bonus_sets: HashMap<u32, BTreeSet<u32>>,
    variables: HashMap<u32, String>,
    spell_variables: HashMap<u32, u32>,
}

pub fn load(conn: &Connection) -> rusqlite::Result<ItemProcInputs> {
    let mut sets = sets::load_sets(conn)?;
    let bonus_sets = sets::load_spells(conn, &mut sets)?;
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
        bonus_sets,
        variables,
        spell_variables,
    })
}

impl ItemProcInputs {
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
    inputs.has_set_bonus(world, unit, spell)
}

/// SimC76f8118 unique_gear.cpp5023–5025 uses the same role expression as native variable884.
pub(crate) fn default_role_multiplier(world: &World, unit: Entity) -> Result<f32, String> {
    let inputs = world
        .get_resource::<ItemProcInputs>()
        .ok_or("native item proc inputs not initialized")?;
    let expression = inputs
        .variables
        .get(&884)
        .ok_or("native role variable884 absent")?;
    parse_role_multiplier(expression, |spell| {
        crate::auras::has_aura(world, unit, spell)
    })
}

pub fn role_multiplier(world: &World, unit: Entity, driver: u32) -> Result<f32, String> {
    world
        .get_resource::<ItemProcInputs>()
        .ok_or("native item proc inputs not initialized")?
        .role_multiplier(driver, |spell| crate::auras::has_aura(world, unit, spell))
}
