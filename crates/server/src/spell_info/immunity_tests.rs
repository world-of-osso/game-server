//! Native TDB1733: loader and actual immunity boundary; missing-table compatibility.
use bevy::prelude::Entity;
use rusqlite::Connection;

use crate::auras::{AuraApplication, AuraSource};
use crate::spell_cast::test_harness::*;

const FROST_NOVA: u32 = 122;
const BLADESTORM: u32 = 227847;

fn load_native_bladestorm(include_immunities: bool) -> super::SpellInfo {
    let source = crate::test_data::world_db_path();
    let conn = Connection::open_in_memory().unwrap();
    let uri = format!("file:{}?mode=ro", source.display());
    conn.execute("ATTACH DATABASE ? AS native", [&uri]).unwrap();
    let tables: Vec<(String, String)> = {
        let mut query = conn
            .prepare(
                "SELECT name, sql FROM native.sqlite_master WHERE type='table'
             AND (name='spell' OR name LIKE 'spell_%' OR name='creature_immunities')",
            )
            .unwrap();
        query
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    };
    for (name, ddl) in tables {
        if name == "creature_immunities" && !include_immunities {
            continue;
        }
        conn.execute_batch(&ddl).unwrap();
        let columns: Vec<String> = {
            let mut query = conn
                .prepare(&format!("PRAGMA native.table_info(\"{name}\")"))
                .unwrap();
            query
                .query_map([], |row| row.get(1))
                .unwrap()
                .collect::<rusqlite::Result<_>>()
                .unwrap()
        };
        let filter = if columns.iter().any(|column| column == "SpellID") {
            " WHERE SpellID IN (122,227847)"
        } else if name == "spell" || name == "spell_name" {
            " WHERE ID IN (122,227847)"
        } else {
            ""
        };
        conn.execute(
            &format!("INSERT INTO \"{name}\" SELECT * FROM native.\"{name}\"{filter}"),
            [],
        )
        .unwrap();
    }
    super::load::load_spell_store(&conn)
        .unwrap()
        .get(BLADESTORM)
        .unwrap()
        .clone()
}

fn apply_bladestorm(h: &mut Harness, target: Entity) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: target,
            target,
            spell_id: BLADESTORM,
            effect_mask: 1 << 1,
            source: AuraSource::Cast,
        },
    );
}

#[test]
fn class_completeness_native_immunity_definition_purges_and_blocks_frost_nova_root() {
    let shield = load_native_bladestorm(true);
    for existing_root in [true, false] {
        let mut h = harness(vec![shield.clone()], &[FROST_NOVA], 8).unwrap();
        let npc = h.npc;
        if existing_root {
            h.cast_and_deliver(FROST_NOVA, Some(npc));
            assert!(crate::auras::has_aura(h.app.world(), npc, FROST_NOVA));
            apply_bladestorm(&mut h, npc);
        } else {
            apply_bladestorm(&mut h, npc);
            h.cast_and_deliver(FROST_NOVA, Some(npc));
        }
        assert!(!crate::auras::has_aura(h.app.world(), npc, FROST_NOVA));
    }
}

#[test]
fn class_completeness_absent_immunity_table_loads_without_crashing_or_inventing_protection() {
    let shield = load_native_bladestorm(false);
    let mut h = harness(vec![shield], &[FROST_NOVA], 8).unwrap();
    let npc = h.npc;
    apply_bladestorm(&mut h, npc);
    h.cast_and_deliver(FROST_NOVA, Some(npc));
    assert!(crate::auras::has_aura(h.app.world(), npc, FROST_NOVA));
}
