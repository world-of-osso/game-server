//! Native TDB1733: loader and actual immunity boundary; missing-table compatibility.
use bevy::prelude::Entity;
use rusqlite::Connection;

use crate::auras::{AuraApplication, AuraSource, RemoveReason};
use crate::spell_cast::test_harness::*;

const FROST_NOVA: u32 = 122;
const BLADESTORM: u32 = 227847;
const BERSERK: u32 = 50334;
const DEATHS_ADVANCE: u32 = 48265;
const TAIL_SWIPE: u32 = 368970;
const THUNDERSTORM: u32 = 51490;
const IMMUNITY_PURGES_EFFECT: u32 = 0x0000_8000;
const SPELL_NO_IMMUNITIES: u32 = 0x2000_0000;

fn query_native_spell_tables(conn: &Connection) -> Vec<(String, String)> {
    let mut query = conn
        .prepare(
            "SELECT name, sql FROM native.sqlite_master WHERE type='table'
         AND (name='spell' OR name LIKE 'spell_%')",
        )
        .unwrap();
    query
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}

fn copy_native_spell_table(conn: &Connection, name: &str, ddl: &str) {
    conn.execute_batch(ddl).unwrap();
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
        " WHERE SpellID IN (122,227847,48265)"
    } else if name == "spell" || name == "spell_name" {
        " WHERE ID IN (122,227847,48265)"
    } else {
        ""
    };
    conn.execute(
        &format!("INSERT INTO \"{name}\" SELECT * FROM native.\"{name}\"{filter}"),
        [],
    )
    .unwrap();
}

fn insert_native_immunity_definitions(conn: &Connection) {
    // Exact TDB1210 definitions, independent of canonical import/deployment timing.
    conn.execute_batch(
        "CREATE TABLE creature_immunities (
        ID INTEGER PRIMARY KEY, SchoolMask INTEGER, DispelTypeMask INTEGER,
        MechanicsMask INTEGER, Effects TEXT, Auras TEXT,
        ImmuneAoE INTEGER, ImmuneChain INTEGER, Comment TEXT);
        INSERT INTO creature_immunities VALUES
        (1733,0,0,1234599078,'98,124,144,145','',0,0,'Immune CC+Stun+Move'),
        (1808,0,0,0,'98,124,144,145','',0,0,'Immune to knockback'),
        (1887,0,0,0,'98,124,144,145','',0,0,'Immune to knockback');",
    )
    .unwrap();
}

fn load_native_bladestorm(include_immunities: bool) -> super::SpellInfo {
    load_native_spell(BLADESTORM, include_immunities)
}

fn load_native_spell(spell_id: u32, include_immunities: bool) -> super::SpellInfo {
    let source = crate::test_data::world_db_path();
    let conn = Connection::open_in_memory().unwrap();
    let uri = format!("file:{}?mode=ro", source.display());
    conn.execute("ATTACH DATABASE ? AS native", [&uri]).unwrap();
    for (name, ddl) in query_native_spell_tables(&conn) {
        copy_native_spell_table(&conn, &name, &ddl);
    }
    if include_immunities {
        insert_native_immunity_definitions(&conn);
    }
    super::load_spell_store(&conn)
        .unwrap()
        .get(spell_id)
        .unwrap()
        .clone()
}

fn apply_immunity_effect(h: &mut Harness, target: Entity, spell_id: u32, index: u8) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: target,
            target,
            spell_id,
            effect_mask: 1 << index,
            source: AuraSource::Cast,
        },
    );
}

fn apply_bladestorm(h: &mut Harness, target: Entity) {
    apply_immunity_effect(h, target, BLADESTORM, 1);
}

#[derive(Debug, PartialEq)]
struct KnockbackOutcome {
    damage: bool,
    daze: bool,
    immune_logged: bool,
}

/// Native Death's Advance (1887: effects 98/124/144/145) or Bladestorm
/// (1733: same effects plus snare mechanic) shields the kobold from `spell`.
fn cast_against_immunity(shield: u32, index: u8, class: u8, spell: u32) -> KnockbackOutcome {
    let mut h = harness(vec![load_native_spell(shield, true)], &[spell], class).unwrap();
    let npc = h.npc;
    apply_immunity_effect(&mut h, npc, shield, index);
    assert!(crate::auras::has_aura(h.app.world(), npc, shield));
    let health = h.health(npc);
    h.cast_and_deliver(spell, None);
    let succeeded = h.inbox.go.iter().any(|go| go.spell_id == spell);
    assert!(succeeded, "{spell} must cast: {:?}", h.inbox.failed);
    KnockbackOutcome {
        damage: h.health(npc) < health,
        daze: crate::auras::has_aura(h.app.world(), npc, spell),
        immune_logged: h.inbox.log.iter().any(|event| {
            event.spell_id == Some(spell)
                && event.kind
                    == shared::protocol::CombatLogKind::Miss(shared::protocol::MissKind::Immune)
        }),
    }
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
    assert!(crate::auras::take_unhandled_auras(h.app.world_mut()).contains(&(BLADESTORM, 147)));
}

#[test]
fn class_completeness_native_immunity_removal_and_expiry_restore_root_hits() {
    let shield = load_native_bladestorm(true);
    for expire in [false, true] {
        let mut h = harness(vec![shield.clone()], &[FROST_NOVA], 8).unwrap();
        let npc = h.npc;
        apply_bladestorm(&mut h, npc);
        if expire {
            h.advance(121); // Native six-second duration.
        } else {
            crate::auras::remove_auras(h.app.world_mut(), npc, RemoveReason::Cancelled, |aura| {
                aura.spell_id == BLADESTORM
            });
        }
        assert!(!crate::auras::has_aura(h.app.world(), npc, BLADESTORM));
        h.cast_and_deliver(FROST_NOVA, Some(npc));
        assert!(crate::auras::has_aura(h.app.world(), npc, FROST_NOVA));
    }
}

#[test]
fn class_completeness_native_immunity_does_not_purge_without_the_authored_flag() {
    let mut shield = load_native_bladestorm(true);
    shield.misc.as_mut().unwrap().attributes[1] &= !IMMUNITY_PURGES_EFFECT;
    let mut h = harness(vec![shield], &[FROST_NOVA], 8).unwrap();
    let npc = h.npc;
    h.cast_and_deliver(FROST_NOVA, Some(npc));
    apply_bladestorm(&mut h, npc);
    assert!(crate::auras::has_aura(h.app.world(), npc, FROST_NOVA));
}

#[test]
fn class_completeness_native_immunity_respects_no_immunities_spell_attribute() {
    let shield = load_native_bladestorm(true);
    let mut nova = world_db().unwrap().store.get(FROST_NOVA).unwrap().clone();
    nova.misc.as_mut().unwrap().attributes[0] |= SPELL_NO_IMMUNITIES;
    let mut h = harness(vec![shield, nova], &[], 8).unwrap();
    let npc = h.npc;
    apply_bladestorm(&mut h, npc);
    h.cast_and_deliver(FROST_NOVA, Some(npc));
    assert!(crate::auras::has_aura(h.app.world(), npc, FROST_NOVA));
}

#[test]
fn class_completeness_native_missing_berserk_definition_stays_an_explicit_gap() {
    let mut h = harness(Vec::new(), &[BERSERK, FROST_NOVA], 8).unwrap();
    let npc = h.npc;
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: npc,
            target: npc,
            spell_id: BERSERK,
            effect_mask: 1 << 9,
            source: AuraSource::Cast,
        },
    );
    h.cast_and_deliver(FROST_NOVA, Some(npc));
    assert!(crate::auras::has_aura(h.app.world(), npc, FROST_NOVA));
    assert!(crate::auras::take_unhandled_auras(h.app.world_mut()).contains(&(BERSERK, 147)));
}

fn outcome(damage: bool, daze: bool, immune_logged: bool) -> KnockbackOutcome {
    KnockbackOutcome {
        damage,
        daze,
        immune_logged,
    }
}

#[test]
fn class_completeness_native_knockback_immunity_filters_only_knockback_effects() {
    // TC Spell::AddUnitTarget drops immune effects and keeps the target hit.
    let swipe = cast_against_immunity(DEATHS_ADVANCE, 2, 13, TAIL_SWIPE);
    assert_eq!(swipe, outcome(false, true, false));
    let storm = cast_against_immunity(DEATHS_ADVANCE, 2, 7, THUNDERSTORM);
    assert_eq!(storm, outcome(true, true, false));
}

#[test]
fn class_completeness_native_mechanic_and_effect_immunity_combine_per_target() {
    // UpdateSpellCastDataTargets logs immunity only for an empty effect mask.
    let swipe = cast_against_immunity(BLADESTORM, 1, 13, TAIL_SWIPE);
    assert_eq!(swipe, outcome(false, false, true));
    let storm = cast_against_immunity(BLADESTORM, 1, 7, THUNDERSTORM);
    assert_eq!(storm, outcome(true, false, false));
}
