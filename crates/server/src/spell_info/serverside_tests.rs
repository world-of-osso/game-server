//! Runtime loading of TDB server-side spells from a prepared copy: the actual
//! Adarogg rows (scripts/tests/fixtures/adarogg-serverside-spells.json, captured
//! from the TDB dump by scripts/encounter_serverside_spells.py), with every DB2
//! lookup table read from the canonical world.db without writing it.

use rusqlite::{Connection, OpenFlags};
use serde_json::Value;

use super::*;

const FIXTURE: &str =
    include_str!("../../../../scripts/tests/fixtures/adarogg-serverside-spells.json");

/// In-memory copy holding the actual server-side tables, canonical DB2 attached read-only.
pub(crate) fn adarogg_serverside_copy() -> Connection {
    let conn = Connection::open_with_flags(
        ":memory:",
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_URI,
    )
    .unwrap();
    let canonical = crate::test_data::world_db_path();
    conn.execute(
        "ATTACH DATABASE ?1 AS canonical",
        [format!("file:{}?mode=ro", canonical.display())],
    )
    .unwrap();
    write_adarogg_serverside_tables(&conn);
    conn
}

/// The actual rows as `tdb_serverside_spell*` in `conn`'s main schema.
pub(crate) fn write_adarogg_serverside_tables(conn: &Connection) {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    for (table, columns) in fixture["schema"].as_object().unwrap() {
        let columns: Vec<&str> = columns
            .as_array()
            .unwrap()
            .iter()
            .map(|column| column.as_str().unwrap())
            .collect();
        insert_source_rows(conn, table, &columns, &fixture["rows"][table]);
    }
}

fn insert_source_rows(conn: &Connection, table: &str, columns: &[&str], rows: &Value) {
    let quoted: Vec<String> = columns
        .iter()
        .map(|column| format!("\"{column}\""))
        .collect();
    conn.execute(
        &format!("CREATE TABLE main.tdb_{table} ({})", quoted.join(",")),
        [],
    )
    .unwrap();
    let placeholders = vec!["?"; columns.len()].join(",");
    let sql = format!(
        "INSERT INTO main.tdb_{table} ({}) VALUES ({placeholders})",
        quoted.join(",")
    );
    for row in rows.as_array().unwrap() {
        let values: Vec<rusqlite::types::Value> = columns
            .iter()
            .map(|column| sql_value(&row[*column]))
            .collect();
        conn.execute(&sql, rusqlite::params_from_iter(values))
            .unwrap();
    }
}

fn sql_value(value: &Value) -> rusqlite::types::Value {
    use rusqlite::types::Value as Sql;
    match value {
        Value::Null => Sql::Null,
        Value::String(text) => Sql::Text(text.clone()),
        Value::Number(number) => number
            .as_i64()
            .map_or_else(|| Sql::Real(number.as_f64().unwrap()), Sql::Integer),
        other => panic!("unexpected source value {other}"),
    }
}

fn production_store() -> SpellStore {
    crate::spell_cast::test_harness::world_db()
        .expect("world.db")
        .store
        .clone()
}

#[test]
fn encounter_coverage_serverside_spells_load_every_modelled_source_field() {
    let mut store = production_store();
    if store.get(119297).is_none() {
        let added = store.append_serverside(&adarogg_serverside_copy()).unwrap();
        assert_eq!(added, 2);
    }

    let summon = store.get(119297).expect("119297 loaded");
    assert_eq!(summon.name, "Inferno Charge");
    let misc = summon.misc.as_ref().unwrap();
    assert_eq!(misc.attributes[0], 256);
    assert_eq!(misc.attributes[3], 256);
    assert_eq!(misc.school_mask, 1);
    assert_eq!(misc.duration.unwrap().duration_ms, 5_000, "DurationIndex28");
    assert_eq!(misc.range.unwrap().max_hostile, 40.0, "RangeIndex5");
    assert_eq!(misc.cast_time.unwrap().base_ms, 0, "CastingTimeIndex1");
    assert!(summon.equipped_items.is_none(), "EquippedItemClass-1");
    assert_eq!(summon.effects.len(), 1);
    let effect = &summon.effects[0];
    assert_eq!(effect.effect, 28);
    assert_eq!(effect.misc_values, [61409, 491]);
    assert_eq!(effect.implicit_targets, [53, 0]);
    assert_eq!(effect.chain_amplitude, 1.0);

    let charge = store.get(119298).expect("119298 loaded");
    let misc = charge.misc.as_ref().unwrap();
    assert_eq!(misc.attributes[8], 134_217_728);
    assert_eq!(misc.speed, 25.0);
    assert!(misc.duration.is_none(), "DurationIndex0");
    let effect = &charge.effects[0];
    assert_eq!(
        (effect.effect, effect.trigger_spell, effect.implicit_targets),
        (96, 119299, [46, 0])
    );

    let file = store.get(119405).unwrap();
    assert_eq!(
        file.effects[0].trigger_spell, 119298,
        "DB2 spells unchanged"
    );
}

#[test]
fn encounter_coverage_serverside_spell_cannot_override_a_db2_spell() {
    let conn = adarogg_serverside_copy();
    conn.execute_batch(
        "UPDATE main.tdb_serverside_spell SET Id = 119405, SpellName = 'Override' WHERE Id = 119297;
         UPDATE main.tdb_serverside_spell_effect SET SpellID = 119405 WHERE SpellID = 119297;",
    )
    .unwrap();
    let mut store = production_store();
    let before = store.get(119405).unwrap().clone();
    let new_charge = usize::from(store.get(119298).is_none());
    assert_eq!(store.append_serverside(&conn).unwrap(), new_charge);
    let after = store.get(119405).unwrap();
    assert_eq!(after.name, before.name);
    assert_eq!(after.effects.len(), before.effects.len());
    assert_eq!(after.effects[0].effect, before.effects[0].effect);
    assert!(
        store.get(119298).is_some(),
        "other server-side rows still load"
    );
}

#[test]
fn encounter_coverage_serverside_tables_absent_load_none() {
    let conn = Connection::open_with_flags(
        ":memory:",
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_URI,
    )
    .unwrap();
    conn.execute(
        "ATTACH DATABASE ?1 AS canonical",
        [format!(
            "file:{}?mode=ro",
            crate::test_data::world_db_path().display()
        )],
    )
    .unwrap();
    let mut store = production_store();
    let before = store.len();
    assert_eq!(store.append_serverside(&conn).unwrap(), 0);
    assert_eq!(store.len(), before);
}
