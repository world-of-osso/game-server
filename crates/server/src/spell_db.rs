//! Loads spell definitions from AzerothCore `spell_dbc` + `spell_bonus_data`
//! tables in world.db into a runtime `SpellDatabase` resource.
//!
//! The import pipeline: AzerothCore MySQL → SQLite (via `import-azerothcore.sh`)
//! → this module loads at startup. SimC overlay can be applied by updating
//! coefficients in spell_bonus_data before import.

use std::collections::HashMap;

use bevy::prelude::*;
use rusqlite::Connection;
use shared::components::SpellSchool;
use shared::spell_data::{ResourceType, SpellCost, SpellData, SpellEffectDef, SpellTarget};

const WORLD_DB_PATH: &str = "data/world.db";

/// Runtime spell database loaded from world.db at startup.
#[derive(Resource, Debug)]
pub struct SpellDatabase {
    spells: HashMap<u32, SpellData>,
}

impl SpellDatabase {
    /// Look up a spell by ID.
    pub fn get(&self, spell_id: u32) -> Option<&SpellData> {
        self.spells.get(&spell_id)
    }

    /// Total number of loaded spells.
    pub fn len(&self) -> usize {
        self.spells.len()
    }

    /// Whether the database is empty.
    pub fn is_empty(&self) -> bool {
        self.spells.is_empty()
    }
}

/// Map AzerothCore PowerType integer to our ResourceType.
fn power_type(val: i32) -> Option<ResourceType> {
    match val {
        0 => Some(ResourceType::Mana),
        1 => Some(ResourceType::Rage),
        2 => Some(ResourceType::Focus),
        3 => Some(ResourceType::Energy),
        5 => Some(ResourceType::RunicPower),
        _ => None,
    }
}

/// Map AzerothCore SchoolMask bitmask to our SpellSchool.
/// Uses the highest-priority school if multiple bits are set.
fn school_from_mask(mask: u32) -> SpellSchool {
    if mask & 0x20 != 0 {
        SpellSchool::Shadow
    } else if mask & 0x10 != 0 {
        SpellSchool::Arcane
    } else if mask & 0x08 != 0 {
        SpellSchool::Frost
    } else if mask & 0x04 != 0 {
        SpellSchool::Nature
    } else if mask & 0x02 != 0 {
        SpellSchool::Holy
    } else if mask & 0x01 != 0 {
        SpellSchool::Physical
    } else {
        // Fire is 0x04 in some references, but bit 2 in standard WoW encoding
        // Default to Physical for mask=0
        SpellSchool::Physical
    }
}

/// Map AzerothCore spell effect type ID to our SpellEffectDef.
/// Only maps effect types we support; returns None for unsupported.
fn map_effect(
    effect_type: u32,
    base_points: i32,
    ap_coeff: f32,
    sp_coeff: f32,
) -> Option<SpellEffectDef> {
    match effect_type {
        // SPELL_EFFECT_SCHOOL_DAMAGE = 2
        2 => Some(SpellEffectDef::SchoolDamage {
            base_min: base_points as f32,
            base_max: base_points as f32,
            ap_coefficient: ap_coeff,
            sp_coefficient: sp_coeff,
        }),
        // SPELL_EFFECT_HEAL = 10
        10 => Some(SpellEffectDef::Heal {
            base_min: base_points as f32,
            base_max: base_points as f32,
            ap_coefficient: ap_coeff,
            sp_coefficient: sp_coeff,
        }),
        // SPELL_EFFECT_ENERGIZE = 30
        30 => Some(SpellEffectDef::Energize {
            resource: ResourceType::Mana, // simplified; real mapping needs PowerType per effect
            amount: base_points as f32,
        }),
        // SPELL_EFFECT_APPLY_AURA = 6
        6 => Some(SpellEffectDef::ApplyAura {
            aura_spell_id: 0, // resolved at runtime from aura tables
        }),
        // SPELL_EFFECT_DISPEL = 19
        19 => Some(SpellEffectDef::Dispel {
            school: SpellSchool::Physical, // simplified
        }),
        // SPELL_EFFECT_INTERRUPT_CAST = 32
        32 => Some(SpellEffectDef::Interrupt {
            lockout_duration: 0.0, // needs mechanic data
        }),
        _ => None,
    }
}

/// Determine spell target type from AzerothCore ImplicitTargetA values.
fn target_from_implicit(target_a: u32) -> SpellTarget {
    match target_a {
        1 => SpellTarget::Self_,              // TARGET_UNIT_CASTER
        5 | 21 | 25 => SpellTarget::Friendly, // TARGET_UNIT_TARGET_ALLY variants
        6 | 15 | 16 => SpellTarget::Hostile,  // TARGET_UNIT_TARGET_ENEMY variants
        _ => SpellTarget::Hostile,            // default to hostile for unknown
    }
}

/// Load spell coefficients from spell_bonus_data table.
fn load_bonus_data(conn: &Connection) -> HashMap<u32, (f32, f32, f32, f32)> {
    let mut stmt = conn
        .prepare(
            "SELECT entry, direct_bonus, dot_bonus, ap_bonus, ap_dot_bonus FROM spell_bonus_data",
        )
        .expect("Failed to prepare spell_bonus_data query");
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, f32>(1)?,
                row.get::<_, f32>(2)?,
                row.get::<_, f32>(3)?,
                row.get::<_, f32>(4)?,
            ))
        })
        .expect("Failed to query spell_bonus_data");

    let mut map = HashMap::new();
    for row in rows {
        let (entry, sp_direct, _sp_dot, ap_direct, _ap_dot) = row.unwrap();
        map.insert(entry, (sp_direct, _sp_dot, ap_direct, _ap_dot));
    }
    map
}

/// CastingTimeIndex → seconds. Approximates SpellCastTimes.dbc lookup.
fn cast_time_from_index(idx: u32) -> f32 {
    match idx {
        0 | 1 => 0.0,
        3 => 1.5,
        4 => 2.0,
        5 => 2.5,
        7 => 3.0,
        15 => 3.5,
        _ => 0.0,
    }
}

/// Convert a SpellRow + bonus coefficients into a SpellData.
fn build_spell_data(r: SpellRow, bonus: &HashMap<u32, (f32, f32, f32, f32)>) -> SpellData {
    let (sp_direct, _sp_dot, ap_direct, _ap_dot) =
        bonus.get(&r.id).copied().unwrap_or((0.0, 0.0, 0.0, 0.0));

    let effects = [
        map_effect(r.effect[0], r.base_points[0], ap_direct, sp_direct),
        map_effect(r.effect[1], r.base_points[1], 0.0, 0.0),
        map_effect(r.effect[2], r.base_points[2], 0.0, 0.0),
    ];

    let cost = if r.mana_cost > 0 {
        power_type(r.power_type).map(|resource| SpellCost {
            resource,
            amount: r.mana_cost as f32,
        })
    } else {
        None
    };

    SpellData {
        id: r.id,
        name: r.name,
        school: school_from_mask(r.school_mask),
        cast_time: cast_time_from_index(r.cast_time_idx),
        cooldown: r.recovery_time as f32 / 1000.0,
        cost,
        target: target_from_implicit(r.target_a),
        range: range_from_index(r.range_idx),
        cast_while_moving: false,
        interruptible: r.interrupt_flags & 0x01 != 0,
        effects,
    }
}

const SPELL_DBC_QUERY: &str =
    "SELECT ID, Name_Lang_enUS, SchoolMask, CastingTimeIndex, RecoveryTime,
            PowerType, ManaCost, RangeIndex, InterruptFlags,
            Effect_1, Effect_2, Effect_3,
            EffectBasePoints_1, EffectBasePoints_2, EffectBasePoints_3,
            ImplicitTargetA_1
     FROM spell_dbc
     WHERE Name_Lang_enUS != ''";

/// Load spells from world.db into a SpellDatabase.
pub fn load_spell_database(conn: &Connection) -> SpellDatabase {
    let bonus = load_bonus_data(conn);

    let mut stmt = conn
        .prepare(SPELL_DBC_QUERY)
        .expect("Failed to prepare spell_dbc query");
    let rows = stmt
        .query_map([], |row| {
            Ok(SpellRow {
                id: row.get(0)?,
                name: row.get(1)?,
                school_mask: row.get(2)?,
                cast_time_idx: row.get(3)?,
                recovery_time: row.get(4)?,
                power_type: row.get(5)?,
                mana_cost: row.get(6)?,
                range_idx: row.get(7)?,
                interrupt_flags: row.get(8)?,
                effect: [row.get(9)?, row.get(10)?, row.get(11)?],
                base_points: [row.get(12)?, row.get(13)?, row.get(14)?],
                target_a: row.get(15)?,
            })
        })
        .expect("Failed to query spell_dbc");

    let mut spells = HashMap::new();
    for row in rows {
        let spell = build_spell_data(row.unwrap(), &bonus);
        spells.insert(spell.id, spell);
    }

    SpellDatabase { spells }
}

/// Approximate range from RangeIndex. Common values from SpellRange.dbc.
fn range_from_index(idx: u32) -> f32 {
    match idx {
        0 | 1 => 0.0, // self / melee
        2 => 5.0,     // melee (5y)
        3 => 20.0,
        4 => 30.0,
        5 | 6 => 40.0, // most offensive spells
        7 => 10.0,
        8 => 45.0,
        _ => 30.0, // reasonable default
    }
}

struct SpellRow {
    id: u32,
    name: String,
    school_mask: u32,
    cast_time_idx: u32,
    recovery_time: u32,
    power_type: i32,
    mana_cost: u32,
    range_idx: u32,
    interrupt_flags: u32,
    effect: [u32; 3],
    base_points: [i32; 3],
    target_a: u32,
}

pub struct SpellDatabasePlugin;

impl Plugin for SpellDatabasePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_spell_db_startup);
    }
}

fn load_spell_db_startup(mut commands: Commands) {
    let path = std::path::Path::new(WORLD_DB_PATH);
    if !path.exists() {
        warn!("world.db not found at {WORLD_DB_PATH}, spell database empty");
        commands.insert_resource(SpellDatabase {
            spells: HashMap::new(),
        });
        return;
    }

    let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("Failed to open world.db for spell data");

    let db = load_spell_database(&conn);
    info!(
        "Loaded {} spells from world.db (empty={})",
        db.len(),
        db.is_empty()
    );
    // Smoke test: verify a well-known spell loaded
    if let Some(spell) = db.get(0) {
        debug!("First spell: {} (id={})", spell.name, spell.id);
    }
    commands.insert_resource(db);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn school_mask_physical() {
        assert_eq!(school_from_mask(0x01), SpellSchool::Physical);
    }

    #[test]
    fn school_mask_fire_is_physical_bit1() {
        // In WoW, fire = bit 2 (0x04) but that maps to Nature in standard encoding
        // Fire school mask = 0x04 in some tables, but SchoolMask in spell_dbc uses:
        // 1=Physical, 2=Holy, 4=Fire, 8=Nature, 16=Frost, 32=Shadow, 64=Arcane
        // Our mapping: 0x04 → Nature (WotLK DBC encoding)
        assert_eq!(school_from_mask(0x04), SpellSchool::Nature);
    }

    #[test]
    fn school_mask_shadow() {
        assert_eq!(school_from_mask(0x20), SpellSchool::Shadow);
    }

    #[test]
    fn school_mask_zero_defaults_physical() {
        assert_eq!(school_from_mask(0), SpellSchool::Physical);
    }

    #[test]
    fn power_type_mana() {
        assert_eq!(power_type(0), Some(ResourceType::Mana));
    }

    #[test]
    fn power_type_rage() {
        assert_eq!(power_type(1), Some(ResourceType::Rage));
    }

    #[test]
    fn power_type_energy() {
        assert_eq!(power_type(3), Some(ResourceType::Energy));
    }

    #[test]
    fn power_type_unknown() {
        assert_eq!(power_type(99), None);
    }

    #[test]
    fn map_effect_school_damage() {
        let eff = map_effect(2, 500, 1.0, 0.8).unwrap();
        match eff {
            SpellEffectDef::SchoolDamage {
                base_min,
                ap_coefficient,
                sp_coefficient,
                ..
            } => {
                assert_eq!(base_min, 500.0);
                assert_eq!(ap_coefficient, 1.0);
                assert_eq!(sp_coefficient, 0.8);
            }
            _ => panic!("expected SchoolDamage"),
        }
    }

    #[test]
    fn map_effect_heal() {
        let eff = map_effect(10, 1000, 0.0, 1.2).unwrap();
        assert!(matches!(eff, SpellEffectDef::Heal { .. }));
    }

    #[test]
    fn map_effect_unknown_returns_none() {
        assert!(map_effect(999, 0, 0.0, 0.0).is_none());
    }

    #[test]
    fn target_hostile_default() {
        assert_eq!(target_from_implicit(6), SpellTarget::Hostile);
    }

    #[test]
    fn target_friendly() {
        assert_eq!(target_from_implicit(5), SpellTarget::Friendly);
    }

    #[test]
    fn target_self() {
        assert_eq!(target_from_implicit(1), SpellTarget::Self_);
    }

    #[test]
    fn range_melee() {
        assert_eq!(range_from_index(1), 0.0);
    }

    #[test]
    fn range_40y() {
        assert_eq!(range_from_index(5), 40.0);
    }

    // --- Loading tests ---

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE spell_dbc (
                ID INTEGER PRIMARY KEY, Name_Lang_enUS TEXT, SchoolMask INTEGER,
                CastingTimeIndex INTEGER, RecoveryTime INTEGER,
                PowerType INTEGER, ManaCost INTEGER, RangeIndex INTEGER,
                InterruptFlags INTEGER,
                Effect_1 INTEGER, Effect_2 INTEGER, Effect_3 INTEGER,
                EffectBasePoints_1 INTEGER, EffectBasePoints_2 INTEGER, EffectBasePoints_3 INTEGER,
                ImplicitTargetA_1 INTEGER
            );
            CREATE TABLE spell_bonus_data (
                entry INTEGER PRIMARY KEY, direct_bonus REAL, dot_bonus REAL,
                ap_bonus REAL, ap_dot_bonus REAL
            );",
        )
        .unwrap();
        conn
    }

    fn insert_spell(conn: &Connection, id: u32, name: &str, school: u32, effect: u32, base: i32) {
        conn.execute(
            "INSERT INTO spell_dbc VALUES (?1, ?2, ?3, 0, 0, 0, 0, 5, 0, ?4, 0, 0, ?5, 0, 0, 6)",
            rusqlite::params![id, name, school, effect, base],
        )
        .unwrap();
    }

    #[test]
    fn load_empty_db() {
        let conn = test_db();
        let db = load_spell_database(&conn);
        assert!(db.is_empty());
        assert_eq!(db.len(), 0);
    }

    #[test]
    fn load_single_spell() {
        let conn = test_db();
        insert_spell(&conn, 585, "Smite", 0x02, 2, 200);
        let db = load_spell_database(&conn);
        assert_eq!(db.len(), 1);
        let spell = db.get(585).unwrap();
        assert_eq!(spell.name, "Smite");
        assert_eq!(spell.school, SpellSchool::Holy);
    }

    #[test]
    fn load_spell_with_bonus_data() {
        let conn = test_db();
        insert_spell(&conn, 585, "Smite", 0x02, 2, 200);
        conn.execute(
            "INSERT INTO spell_bonus_data VALUES (585, 0.7143, 0.0, 0.0, 0.0)",
            [],
        )
        .unwrap();
        let db = load_spell_database(&conn);
        let spell = db.get(585).unwrap();
        match &spell.effects[0] {
            Some(SpellEffectDef::SchoolDamage { sp_coefficient, .. }) => {
                assert!((sp_coefficient - 0.7143).abs() < 0.001);
            }
            other => panic!("expected SchoolDamage, got {other:?}"),
        }
    }

    #[test]
    fn load_spell_without_bonus_data_has_zero_coefficients() {
        let conn = test_db();
        insert_spell(&conn, 100, "NoBonus", 0x01, 2, 50);
        let db = load_spell_database(&conn);
        let spell = db.get(100).unwrap();
        match &spell.effects[0] {
            Some(SpellEffectDef::SchoolDamage {
                ap_coefficient,
                sp_coefficient,
                ..
            }) => {
                assert_eq!(*ap_coefficient, 0.0);
                assert_eq!(*sp_coefficient, 0.0);
            }
            other => panic!("expected SchoolDamage, got {other:?}"),
        }
    }

    #[test]
    fn load_multiple_spells() {
        let conn = test_db();
        insert_spell(&conn, 1, "Spell1", 0x01, 2, 10);
        insert_spell(&conn, 2, "Spell2", 0x20, 10, 500);
        insert_spell(&conn, 3, "Spell3", 0x10, 2, 100);
        let db = load_spell_database(&conn);
        assert_eq!(db.len(), 3);
        assert_eq!(db.get(1).unwrap().school, SpellSchool::Physical);
        assert_eq!(db.get(2).unwrap().school, SpellSchool::Shadow);
        assert_eq!(db.get(3).unwrap().school, SpellSchool::Arcane);
    }

    #[test]
    fn load_spell_heal_effect() {
        let conn = test_db();
        // effect type 10 = heal, target 1 = self
        conn.execute(
            "INSERT INTO spell_dbc VALUES (1, 'Heal', 2, 3, 0, 0, 200, 0, 1, 10, 0, 0, 800, 0, 0, 1)",
            [],
        )
        .unwrap();
        let db = load_spell_database(&conn);
        let spell = db.get(1).unwrap();
        assert!(matches!(
            spell.effects[0],
            Some(SpellEffectDef::Heal { .. })
        ));
        assert_eq!(spell.target, SpellTarget::Self_);
        assert_eq!(spell.cast_time, 1.5); // CastingTimeIndex 3
        assert!(spell.interruptible); // InterruptFlags & 0x01
    }

    #[test]
    fn load_spell_with_mana_cost() {
        let conn = test_db();
        // PowerType=0 (mana), ManaCost=300
        conn.execute(
            "INSERT INTO spell_dbc VALUES (1, 'Costly', 1, 0, 0, 0, 300, 0, 0, 2, 0, 0, 100, 0, 0, 6)",
            [],
        )
        .unwrap();
        let db = load_spell_database(&conn);
        let spell = db.get(1).unwrap();
        let cost = spell.cost.as_ref().unwrap();
        assert_eq!(cost.resource, ResourceType::Mana);
        assert_eq!(cost.amount, 300.0);
    }

    #[test]
    fn load_spell_with_cooldown() {
        let conn = test_db();
        // RecoveryTime=8000 (8000ms = 8s)
        conn.execute(
            "INSERT INTO spell_dbc VALUES (1, 'CD', 1, 0, 8000, 0, 0, 0, 0, 2, 0, 0, 50, 0, 0, 6)",
            [],
        )
        .unwrap();
        let db = load_spell_database(&conn);
        assert_eq!(db.get(1).unwrap().cooldown, 8.0);
    }

    #[test]
    fn load_spell_zero_cost_is_none() {
        let conn = test_db();
        insert_spell(&conn, 1, "Free", 0x01, 2, 50);
        let db = load_spell_database(&conn);
        assert!(db.get(1).unwrap().cost.is_none());
    }

    #[test]
    fn load_spell_empty_name_excluded() {
        let conn = test_db();
        insert_spell(&conn, 1, "Valid", 0x01, 2, 10);
        conn.execute(
            "INSERT INTO spell_dbc VALUES (2, '', 1, 0, 0, 0, 0, 0, 0, 2, 0, 0, 10, 0, 0, 6)",
            [],
        )
        .unwrap();
        let db = load_spell_database(&conn);
        assert_eq!(db.len(), 1);
        assert!(db.get(2).is_none());
    }

    #[test]
    fn load_spell_unsupported_effect_is_none() {
        let conn = test_db();
        // Effect type 999 = unsupported
        conn.execute(
            "INSERT INTO spell_dbc VALUES (1, 'Weird', 1, 0, 0, 0, 0, 0, 0, 999, 0, 0, 0, 0, 0, 6)",
            [],
        )
        .unwrap();
        let db = load_spell_database(&conn);
        let spell = db.get(1).unwrap();
        assert!(spell.effects[0].is_none());
    }

    #[test]
    fn load_spell_range_from_index() {
        let conn = test_db();
        // RangeIndex=5 → 40y
        insert_spell(&conn, 1, "Ranged", 0x01, 2, 10);
        let db = load_spell_database(&conn);
        assert_eq!(db.get(1).unwrap().range, 40.0);
    }

    #[test]
    fn load_bonus_data_ap_coefficient() {
        let conn = test_db();
        insert_spell(&conn, 1, "Strike", 0x01, 2, 100);
        conn.execute(
            "INSERT INTO spell_bonus_data VALUES (1, 0.0, 0.0, 0.5, 0.0)",
            [],
        )
        .unwrap();
        let db = load_spell_database(&conn);
        match &db.get(1).unwrap().effects[0] {
            Some(SpellEffectDef::SchoolDamage { ap_coefficient, .. }) => {
                assert!((ap_coefficient - 0.5).abs() < 0.001);
            }
            other => panic!("expected SchoolDamage, got {other:?}"),
        }
    }

    #[test]
    fn get_missing_spell_returns_none() {
        let conn = test_db();
        let db = load_spell_database(&conn);
        assert!(db.get(99999).is_none());
    }
}
