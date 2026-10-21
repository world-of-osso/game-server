//! Retail item stats at the item's level: primary stats, ratings, armor and
//! weapon damage, from world.db DB2 tables and the item-level game tables.
//!
//! Formulas follow SimC `sc_item_data.cpp` (`item_database::scaled_stat`,
//! `random_suffix_type`, `item_combat_rating_type`, `armor_value`,
//! `weapon_dmg_min/max`) and `sc_const_data.cpp` (`dbc_t::weapon_dps`), SimC
//! commit 3036108; TrinityCore `Item::GetItemStatValue`,
//! `ItemTemplate::GetArmor` and `ItemTemplate::GetDamage` use the same tables.
//! Old items first take the Midnight item-level squish (SimC
//! `item_t::download_item_data`, curve 92181 from ItemSquishEra).
use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};
#[cfg(test)]
use shared::components::{ItemStatBlock, UnitStats, WeaponType};

use crate::character_stats_data::{
    COMBAT_RATING_MULT_BY_ILVL, SOCKET_COST_BY_ILVL, STAMINA_MULT_BY_ILVL,
};
use crate::item_templates::ItemTemplate;

pub use shared::item_scaling::{
    ItemScaling, ItemScalingTables, ItemStatAllocation, ItemStats, WeaponDamage,
};

#[cfg(test)]
const ITEM_CLASS_WEAPON: u8 = 2;
#[cfg(test)]
const ITEM_CLASS_ARMOR: u8 = 4;
type QualityRow = [f64; 7];

/// Source-local Forever tables; never share the Retail table resource.
#[derive(Resource, Debug, Default, Clone)]
pub struct ForeverItemScalingTables(pub ItemScalingTables);

/// Stats of the source-owned template, using the shared authoritative formula.
pub fn item_stats(template: &ItemTemplate, tables: &ItemScalingTables) -> ItemStats {
    shared::item_scaling::item_stats(&template.scaling, template.inventory_type, tables)
}

fn load_map<K, V>(
    conn: &Connection,
    sql: &str,
    row: impl Fn(&rusqlite::Row) -> rusqlite::Result<(K, V)>,
) -> rusqlite::Result<HashMap<K, V>>
where
    K: std::hash::Hash + Eq,
{
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map([], row)?;
    rows.collect()
}

fn floats<const N: usize>(row: &rusqlite::Row, first: usize) -> rusqlite::Result<[f64; N]> {
    let mut values = [0.0; N];
    for (index, value) in values.iter_mut().enumerate() {
        *value = row.get(first + index)?;
    }
    Ok(values)
}

fn quality_table(
    conn: &Connection,
    table: &str,
    level: &str,
    prefix: &str,
) -> rusqlite::Result<HashMap<u16, QualityRow>> {
    let columns: Vec<String> = (0..7).map(|n| format!("{prefix}_{n}")).collect();
    let sql = format!("SELECT {level}, {} FROM {table}", columns.join(", "));
    load_map(conn, &sql, |row| Ok((row.get(0)?, floats(row, 1)?)))
}

const RAND_PROP_COLUMNS: &str = "ID, EpicF_0, EpicF_1, EpicF_2, EpicF_3, EpicF_4, \
     SuperiorF_0, SuperiorF_1, SuperiorF_2, SuperiorF_3, SuperiorF_4, \
     GoodF_0, GoodF_1, GoodF_2, GoodF_3, GoodF_4";

fn load_squish(conn: &Connection) -> rusqlite::Result<Option<(u8, Vec<(f64, f64)>)>> {
    let era: Option<(u8, i64)> = conn
        .query_row(
            "SELECT ID, CurveID FROM item_squish_era WHERE CurveID > 0 ORDER BY ID DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map(Some)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            error => Err(error),
        })?;
    let Some((era, curve)) = era else {
        return Ok(None);
    };
    let mut statement =
        conn.prepare("SELECT Pos_0, Pos_1 FROM curve_point WHERE CurveID = ? ORDER BY OrderIndex")?;
    let points = statement
        .query_map([curve], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(Some((era, points)))
}

pub fn load_item_scaling_tables(path: &Path) -> rusqlite::Result<ItemScalingTables> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    load_scaling_namespace(&conn, "", load_squish(&conn)?)
}

pub fn load_forever_item_scaling_tables(path: &Path) -> rusqlite::Result<ItemScalingTables> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    // The pinned Forever schema has no modern ItemSquishEra curve.
    load_scaling_namespace(&conn, "skyborne_", None)
}

fn load_scaling_namespace(
    conn: &Connection,
    table_prefix: &str,
    squish: Option<(u8, Vec<(f64, f64)>)>,
) -> rusqlite::Result<ItemScalingTables> {
    let table = |name: &str| format!("{table_prefix}{name}");
    let rand_prop_sql = format!(
        "SELECT {RAND_PROP_COLUMNS} FROM {}",
        table("rand_prop_points")
    );
    let rand_prop_points = load_map(conn, &rand_prop_sql, |row| {
        Ok((
            row.get(0)?,
            [floats(row, 1)?, floats(row, 6)?, floats(row, 11)?],
        ))
    })?;
    let spell_budgets = if table_prefix.is_empty() {
        load_map(
            conn,
            "SELECT ID, DamageReplaceStatF, DamageSecondaryF, SuperiorF_0 FROM rand_prop_points",
            |row| Ok((row.get(0)?, [row.get(1)?, row.get(2)?, row.get(3)?])),
        )?
    } else {
        // Pinned Forever RandPropPoints has no Retail spell-budget fields.
        HashMap::new()
    };
    let armor_total = load_map(
        conn,
        &format!(
            "SELECT ItemLevel, Cloth, Leather, Mail, Plate FROM {}",
            table("item_armor_total")
        ),
        |row| Ok((row.get(0)?, floats(row, 1)?)),
    )?;
    let armor_location = load_map(
        conn,
        &format!(
            "SELECT ID, Clothmodifier, Leathermodifier, Chainmodifier, Platemodifier FROM {}",
            table("armor_location")
        ),
        |row| Ok((row.get(0)?, floats(row, 1)?)),
    )?;
    let damage = |name| quality_table(conn, &table(name), "ItemLevel", "Quality");
    Ok(ItemScalingTables {
        rand_prop_points,
        spell_budgets,
        armor_quality: quality_table(conn, &table("item_armor_quality"), "ID", "Qualitymod")?,
        armor_shield: quality_table(conn, &table("item_armor_shield"), "ItemLevel", "Quality")?,
        armor_total,
        armor_location,
        damage: [
            damage("item_damage_one_hand")?,
            damage("item_damage_one_hand_caster")?,
            damage("item_damage_two_hand")?,
            damage("item_damage_two_hand_caster")?,
        ],
        squish,
        socket_cost: SOCKET_COST_BY_ILVL.to_vec(),
        stamina_multiplier: STAMINA_MULT_BY_ILVL.to_vec(),
        rating_multiplier: COMBAT_RATING_MULT_BY_ILVL.to_vec(),
    })
}

pub struct ItemScalingPlugin;

impl Plugin for ItemScalingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup_load_item_scaling_tables);
    }
}

fn startup_load_item_scaling_tables(mut commands: Commands) {
    let path = Path::new(crate::spell_info::WORLD_DB_PATH);
    let tables = load_item_scaling_tables(path).unwrap_or_else(|error| {
        panic!("load item scaling tables from {}: {error}", path.display())
    });
    info!(
        "Loaded item scaling for {} item levels",
        tables.rand_prop_points.len()
    );
    commands.insert_resource(tables);
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("open native item-instance tables");
    commands.insert_resource(crate::stats::NativeItemInstances {
        bonuses: server::item_instance::load::load_bonus_tables(&conn)
            .expect("load native ItemBonus and scaling tables; run scripts/item_instance_data.py"),
        enchants: server::item_instance::load::load_enchantment_tables(&conn).expect(
            "load native enchantments and GemProperties; run scripts/item_instance_data.py",
        ),
    });
    let forever = load_forever_item_scaling_tables(path).unwrap_or_else(|error| {
        panic!(
            "load isolated Forever scaling from {}: {error}; run scripts/skyborne_scaling_data.py",
            path.display()
        )
    });
    commands.insert_resource(ForeverItemScalingTables(forever));
}

#[cfg(test)]
#[path = "item_stats_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "item_scaling_parity_tests.rs"]
mod parity_tests;
