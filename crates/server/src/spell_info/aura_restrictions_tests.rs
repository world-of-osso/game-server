//! Raw native70391 aura restrictions through the production spell-store loader.
use rusqlite::Connection;
use serde_json::{Value, json};

const SCHEMA: &str = include_str!("../../../../scripts/db2_schema.sql");

fn open_spell_database() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(SCHEMA).unwrap();
    conn
}

fn insert_spell_name(conn: &Connection, spell_id: u32) {
    conn.execute(
        "INSERT INTO spell_name (ID, Name_lang) VALUES (?, 'Restriction fixture')",
        [spell_id],
    )
    .unwrap();
}

fn serialized_spell(conn: &Connection, spell_id: u32) -> Value {
    let store = super::load::load_spell_store(conn).expect("load spell store");
    serde_json::to_value(store.get(spell_id).expect("named spell")).unwrap()
}

fn expected_restrictions() -> Value {
    json!({
        "caster_aura_state": 0, "target_aura_state": 0,
        "exclude_caster_aura_state": 0, "exclude_target_aura_state": 0,
        "caster_aura_spell": 0, "target_aura_spell": 0,
        "exclude_caster_aura_spell": 0, "exclude_target_aura_spell": 0,
        "caster_aura_type": 0, "target_aura_type": 0,
        "exclude_caster_aura_type": 0, "exclude_target_aura_type": 0
    })
}

#[test]
fn aura_restrictions_loader_preserves_native_state_spell_and_type_values() {
    let conn = open_spell_database();
    // Exact SpellAuraRestrictions.csv rows from installed build12.1.5.70391.
    conn.execute_batch(
        "INSERT INTO spell_aura_restrictions VALUES
        (91,0,10,0,0,0,0,0,0,0,0,0,0,0,34428),
        (8972,0,0,2,0,0,0,0,0,0,0,0,0,0,163201),
        (14127,0,0,13,0,0,0,0,0,0,0,0,0,0,281000),
        (9155,0,0,0,0,0,170198,170198,170183,170183,0,0,0,0,170183),
        (24378,0,0,0,0,0,0,0,0,0,0,69,0,0,1293603),
        (21835,0,0,0,0,0,0,0,0,0,0,0,26,0,439683),
        (21439,0,0,0,0,1,0,0,0,0,0,0,0,485,440490),
        (2,0,0,0,12,0,0,0,0,0,0,0,0,0,66);",
    )
    .unwrap();
    let cases: &[(u32, &[(&str, u32)])] = &[
        (34428, &[("caster_aura_state", 10)]),
        (163201, &[("target_aura_state", 2)]),
        (281000, &[("target_aura_state", 13)]),
        (
            170183,
            &[
                ("caster_aura_spell", 170198),
                ("target_aura_spell", 170198),
                ("exclude_caster_aura_spell", 170183),
                ("exclude_target_aura_spell", 170183),
            ],
        ),
        (1293603, &[("target_aura_type", 69)]),
        (439683, &[("exclude_caster_aura_type", 26)]),
        (
            440490,
            &[
                ("exclude_target_aura_state", 1),
                ("exclude_target_aura_type", 485),
            ],
        ),
        (66, &[("exclude_caster_aura_state", 12)]),
    ];
    for (spell_id, fields) in cases {
        insert_spell_name(&conn, *spell_id);
        let mut expected = expected_restrictions();
        for (field, value) in *fields {
            expected[*field] = json!(value);
        }
        assert_eq!(
            serialized_spell(&conn, *spell_id)["aura_restrictions"],
            expected,
            "native restrictions for spell {spell_id}"
        );
    }
}

#[test]
fn aura_restrictions_loader_keeps_absent_rows_sparse_and_ignores_other_difficulties() {
    let conn = open_spell_database();
    for spell_id in [5308, 133, 281000] {
        insert_spell_name(&conn, spell_id);
    }
    conn.execute_batch(
        "INSERT INTO spell_aura_restrictions
        (ID, DifficultyID, TargetAuraState, SpellID) VALUES
        (21,0,2,5308), (14127,1,13,281000), (66,0,2,24275);",
    )
    .unwrap();
    let store = super::load::load_spell_store(&conn).unwrap();
    assert_eq!(store.len(), 3, "orphan restriction must not create a spell");
    assert!(store.get(24275).is_none());
    let mut expected = expected_restrictions();
    expected["target_aura_state"] = json!(2);
    assert_eq!(
        serde_json::to_value(store.get(5308).unwrap()).unwrap()["aura_restrictions"],
        expected
    );
    for spell_id in [133, 281000] {
        assert_eq!(
            serde_json::to_value(store.get(spell_id).unwrap()).unwrap()["aura_restrictions"],
            Value::Null,
            "no difficulty0 restrictions for spell {spell_id}"
        );
    }
}

#[test]
fn aura_restrictions_loader_rejects_unpublished_table() {
    let conn = open_spell_database();
    conn.execute_batch("DROP TABLE spell_aura_restrictions;")
        .unwrap();
    let error = super::load::load_spell_store(&conn).expect_err("missing required source table");
    assert!(
        error.to_string().contains("spell_aura_restrictions"),
        "{error}"
    );
}

#[test]
fn aura_restrictions_loader_rejects_incomplete_schema() {
    let conn = open_spell_database();
    conn.execute_batch("ALTER TABLE spell_aura_restrictions DROP COLUMN ExcludeTargetAuraType;")
        .unwrap();
    let error = super::load::load_spell_store(&conn).expect_err("missing required source column");
    assert!(
        error.to_string().contains("ExcludeTargetAuraType"),
        "{error}"
    );
}
