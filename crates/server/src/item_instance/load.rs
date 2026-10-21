//! Read the native item-instance tables published by scripts/item_instance_data.py.
use super::bonuses::{ItemBonus, ItemBonusTables, OffsetCurve, ScalingConfig};
use super::enchants::{EnchantEffect, Enchantment, EnchantmentTables};
use rusqlite::{Connection, Row};
use std::collections::HashMap;

pub fn load_bonus_tables(conn: &Connection) -> rusqlite::Result<ItemBonusTables> {
    Ok(ItemBonusTables {
        entries: load_bonus_entries(conn)?,
        scaling_configs: load_map(
            conn,
            "SELECT ID,ItemOffsetCurveID,ItemLevel,ItemSquishEraID FROM item_scaling_config",
            |row| {
                Ok((
                    row.get(0)?,
                    ScalingConfig {
                        offset_curve: row.get(1)?,
                        item_level: row.get(2)?,
                        squish_era: row.get(3)?,
                    },
                ))
            },
        )?,
        offset_curves: load_map(
            conn,
            "SELECT ID,CurveID,Offset FROM item_offset_curve",
            |row| {
                Ok((
                    row.get(0)?,
                    OffsetCurve {
                        curve: row.get(1)?,
                        offset: row.get(2)?,
                    },
                ))
            },
        )?,
        curves: load_curves(conn)?,
    })
}

pub fn load_enchantment_tables(conn: &Connection) -> rusqlite::Result<EnchantmentTables> {
    let effects: Vec<String> = (0..3)
        .flat_map(|index| {
            [
                format!("Effect_{index}"),
                format!("EffectPointsMin_{index}"),
                format!("EffectArg_{index}"),
                format!("EffectScalingPoints_{index}"),
            ]
        })
        .collect();
    let sql = format!(
        "SELECT ID,ScalingClass,MinLevel,MaxLevel,{} FROM spell_item_enchantment",
        effects.join(",")
    );
    Ok(EnchantmentTables {
        enchants: load_map(conn, &sql, read_enchantment)?,
        gems: load_map(
            conn,
            "SELECT item.ID,gem.Enchant_ID FROM item_sparse item JOIN gem_properties gem ON item.Gem_properties=gem.ID WHERE item.Gem_properties<>0",
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?,
    })
}

fn read_enchantment(row: &Row) -> rusqlite::Result<(u32, Enchantment)> {
    let mut effects = [EnchantEffect::default(); 3];
    for (index, effect) in effects.iter_mut().enumerate() {
        let first = 4 + index * 4;
        *effect = EnchantEffect {
            kind: row.get(first)?,
            points: row.get(first + 1)?,
            argument: row.get(first + 2)?,
            coefficient: row.get(first + 3)?,
        };
    }
    Ok((
        row.get(0)?,
        Enchantment {
            effects,
            scaling_class: row.get(1)?,
            min_level: row.get(2)?,
            max_level: row.get(3)?,
        },
    ))
}

fn load_map<T>(
    conn: &Connection,
    sql: &str,
    row: impl FnMut(&Row) -> rusqlite::Result<(u32, T)>,
) -> rusqlite::Result<HashMap<u32, T>> {
    conn.prepare(sql)?.query_map([], row)?.collect()
}

fn load_bonus_entries(conn: &Connection) -> rusqlite::Result<HashMap<u32, Vec<ItemBonus>>> {
    let mut statement = conn.prepare("SELECT ParentItemBonusListID,Type,Value_0,Value_1,Value_2,Value_3 FROM item_bonus ORDER BY ParentItemBonusListID,OrderIndex,ID")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            ItemBonus {
                kind: row.get(1)?,
                values: [row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?],
            },
        ))
    })?;
    let mut entries: HashMap<u32, Vec<ItemBonus>> = HashMap::new();
    for row in rows {
        let (id, entry) = row?;
        entries.entry(id).or_default().push(entry);
    }
    Ok(entries)
}

fn load_curves(conn: &Connection) -> rusqlite::Result<HashMap<u32, Vec<(f64, f64)>>> {
    let mut statement =
        conn.prepare("SELECT CurveID,Pos_0,Pos_1 FROM curve_point ORDER BY CurveID,OrderIndex")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, u32>(0)?, (row.get(1)?, row.get(2)?)))
    })?;
    let mut curves: HashMap<u32, Vec<(f64, f64)>> = HashMap::new();
    for row in rows {
        let (id, point) = row?;
        curves.entry(id).or_default().push(point);
    }
    Ok(curves)
}

#[cfg(test)]
mod tests {
    use super::super::bonuses::apply_item_bonuses;
    use super::*;
    use shared::item_scaling::ItemScaling;

    #[test]
    fn item_instance_sqlite_native_gem_and_scaled_enchant_rows_reach_item_stats() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE spell_item_enchantment (ID INTEGER PRIMARY KEY,ScalingClass INTEGER,MinLevel INTEGER,MaxLevel INTEGER,Effect_0 INTEGER,Effect_1 INTEGER,Effect_2 INTEGER,EffectPointsMin_0 INTEGER,EffectPointsMin_1 INTEGER,EffectPointsMin_2 INTEGER,EffectArg_0 INTEGER,EffectArg_1 INTEGER,EffectArg_2 INTEGER,EffectScalingPoints_0 REAL,EffectScalingPoints_1 REAL,EffectScalingPoints_2 REAL);
          INSERT INTO spell_item_enchantment VALUES(7987,-7,0,80,5,0,0,0,0,0,71,0,0,1.48670006,0,0),(8157,-7,0,80,5,5,0,0,0,0,32,40,0,0.475744992,0.208138004,0);
          CREATE TABLE gem_properties(ID INTEGER PRIMARY KEY,Enchant_ID INTEGER,Type INTEGER);
          INSERT INTO gem_properties VALUES(4400,8157,14);
          CREATE TABLE item_sparse(ID INTEGER PRIMARY KEY,Gem_properties INTEGER);
          INSERT INTO item_sparse VALUES(240910,4400);").unwrap();
        let tables = load_enchantment_tables(&conn).unwrap();
        let gem = super::super::enchants::gem_stats(240910, 90, &tables);
        assert!(
            gem.is_ok(),
            "native ItemSparse/GemProperties join must reach its enchantment"
        );
        assert_eq!(gem.unwrap().stats.secondary.crit, 16.0);
        let chest = super::super::enchants::enchantment_stats(7987, 90, &tables);
        assert!(chest.is_ok());
        assert_eq!(chest.unwrap().stats.primary.strength, 50.0);
    }

    #[test]
    fn item_instance_sqlite_native_config_and_rank_rows_resolve_mid1_bracers_level() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE item_bonus (ID INTEGER PRIMARY KEY,Value_0 INTEGER,Value_1 INTEGER,Value_2 INTEGER,Value_3 INTEGER,ParentItemBonusListID INTEGER,Type INTEGER,OrderIndex INTEGER);
          INSERT INTO item_bonus VALUES(25352,273,70,0,0,12066,49,0),(25960,264,100,0,0,12214,49,0),(30327,271,65,0,0,13622,49,0),(30571,13,2,4,0,9627,52,0);
          CREATE TABLE item_scaling_config (ID INTEGER PRIMARY KEY,ItemOffsetCurveID INTEGER,ItemLevel INTEGER,RequiredLevel INTEGER,ItemSquishEraID INTEGER,Flags INTEGER);
          INSERT INTO item_scaling_config VALUES(273,47,246,90,2,0),(264,47,246,90,2,0),(271,47,272,90,2,0);
          CREATE TABLE item_offset_curve (ID INTEGER PRIMARY KEY,CurveID INTEGER,Offset INTEGER);
          INSERT INTO item_offset_curve VALUES(47,88583,0);
          CREATE TABLE curve_point (ID INTEGER PRIMARY KEY,CurveID INTEGER,OrderIndex INTEGER,Pos_0 REAL,Pos_1 REAL);
          INSERT INTO curve_point VALUES(1,88583,0,0,0),(2,88583,1,1300,1300);").unwrap();
        let tables = load_bonus_tables(&conn).unwrap();
        let item = ItemScaling {
            item_level: 197,
            ..Default::default()
        };
        let result = apply_item_bonuses(&item, &[12066, 12214, 13622, 9627], &tables);
        assert!(
            result.is_ok(),
            "native rows must load through SQLite, not become base items"
        );
        assert_eq!(result.unwrap().scaling.item_level, 285);
    }
}
