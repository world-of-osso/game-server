//! Loads retail spell definitions from normalized DB2 tables in world.db
//! (spell_name, spell_misc, spell_effect, spell_power, spell_cast_times,
//! spell_range, spell_cooldowns, spell_duration) into a runtime
//! `SpellDatabase` resource.
//!
//! Import pipeline: WoW retail CASC → DB2 CSV export → SQLite
//! (via `import-azerothcore.sh`) → this module loads at startup.

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

/// Map retail PowerType integer to our ResourceType.
fn power_type(val: i32) -> Option<ResourceType> {
    match val {
        0 => Some(ResourceType::Mana),
        1 => Some(ResourceType::Rage),
        2 => Some(ResourceType::Focus),
        3 => Some(ResourceType::Energy),
        4 => Some(ResourceType::ComboPoints),
        5 => Some(ResourceType::RunicPower),
        9 => Some(ResourceType::HolyPower),
        _ => None,
    }
}

/// Map retail SchoolMask bitmask to our SpellSchool.
/// Uses the highest-priority school if multiple bits are set.
fn school_from_mask(mask: u32) -> SpellSchool {
    if mask & 0x20 != 0 {
        SpellSchool::Shadow
    } else if mask & 0x40 != 0 {
        SpellSchool::Arcane
    } else if mask & 0x10 != 0 {
        SpellSchool::Frost
    } else if mask & 0x08 != 0 {
        SpellSchool::Nature
    } else if mask & 0x04 != 0 {
        SpellSchool::Fire
    } else if mask & 0x02 != 0 {
        SpellSchool::Holy
    } else {
        SpellSchool::Physical
    }
}

/// Map retail spell effect type ID to our SpellEffectDef.
fn map_effect(
    effect_type: u32,
    base_points: f32,
    ap_coeff: f32,
    sp_coeff: f32,
) -> Option<SpellEffectDef> {
    match effect_type {
        // SPELL_EFFECT_SCHOOL_DAMAGE = 2
        2 => Some(SpellEffectDef::SchoolDamage {
            base_min: base_points,
            base_max: base_points,
            ap_coefficient: ap_coeff,
            sp_coefficient: sp_coeff,
        }),
        // SPELL_EFFECT_HEAL = 10
        10 => Some(SpellEffectDef::Heal {
            base_min: base_points,
            base_max: base_points,
            ap_coefficient: ap_coeff,
            sp_coefficient: sp_coeff,
        }),
        // SPELL_EFFECT_ENERGIZE = 30
        30 => Some(SpellEffectDef::Energize {
            resource: ResourceType::Mana, // simplified; real mapping needs per-effect data
            amount: base_points,
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

/// Determine spell target type from retail ImplicitTarget values.
fn target_from_implicit(target_a: u32) -> SpellTarget {
    match target_a {
        1 => SpellTarget::Self_,              // TARGET_UNIT_CASTER
        5 | 21 | 25 => SpellTarget::Friendly, // TARGET_UNIT_TARGET_ALLY variants
        6 | 15 | 16 => SpellTarget::Hostile,  // TARGET_UNIT_TARGET_ENEMY variants
        _ => SpellTarget::Hostile,            // default to hostile for unknown
    }
}

/// Load spell_cast_times lookup: ID → Base cast time in seconds.
fn load_cast_times(conn: &Connection) -> HashMap<u32, f32> {
    let mut stmt = conn
        .prepare("SELECT ID, Base FROM spell_cast_times")
        .expect("Failed to prepare spell_cast_times query");
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, i32>(1)?)))
        .expect("Failed to query spell_cast_times");
    rows.map(|r| {
        let (id, base_ms) = r.unwrap();
        (id, base_ms as f32 / 1000.0)
    })
    .collect()
}

/// Load spell_range lookup: ID → max hostile range in yards.
fn load_ranges(conn: &Connection) -> HashMap<u32, f32> {
    let mut stmt = conn
        .prepare("SELECT ID, RangeMax_0 FROM spell_range")
        .expect("Failed to prepare spell_range query");
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, f32>(1)?)))
        .expect("Failed to query spell_range");
    rows.map(|r| r.unwrap()).collect()
}

/// Load spell_cooldowns: SpellID → RecoveryTime in seconds.
/// Uses the longer of RecoveryTime and CategoryRecoveryTime, DifficultyID=0 only.
fn load_cooldowns(conn: &Connection) -> HashMap<u32, f32> {
    let mut stmt = conn
        .prepare(
            "SELECT SpellID, RecoveryTime, CategoryRecoveryTime
             FROM spell_cooldowns
             WHERE DifficultyID = 0",
        )
        .expect("Failed to prepare spell_cooldowns query");
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, u32>(2)?,
            ))
        })
        .expect("Failed to query spell_cooldowns");
    let mut map = HashMap::new();
    for row in rows {
        let (spell_id, recovery, category_recovery) = row.unwrap();
        let cd_ms = recovery.max(category_recovery);
        if cd_ms > 0 {
            map.insert(spell_id, cd_ms as f32 / 1000.0);
        }
    }
    map
}

/// Intermediate row from spell_effect query.
struct EffectRow {
    effect_index: u32,
    effect_type: u32,
    base_points: f32,
    sp_coeff: f32,
    ap_coeff: f32,
    implicit_target_0: u32,
}

/// Load effects grouped by SpellID. Returns map of SpellID → sorted effects.
fn load_effects(conn: &Connection) -> HashMap<u32, Vec<EffectRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT SpellID, EffectIndex, Effect, EffectBasePointsF,
                    EffectBonusCoefficient, BonusCoefficientFromAP, ImplicitTarget_0
             FROM spell_effect
             WHERE DifficultyID = 0
             ORDER BY SpellID, EffectIndex",
        )
        .expect("Failed to prepare spell_effect query");

    let mut map: HashMap<u32, Vec<EffectRow>> = HashMap::new();
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                EffectRow {
                    effect_index: row.get(1)?,
                    effect_type: row.get(2)?,
                    base_points: row.get(3)?,
                    sp_coeff: row.get(4)?,
                    ap_coeff: row.get(5)?,
                    implicit_target_0: row.get(6)?,
                },
            ))
        })
        .expect("Failed to query spell_effect");

    for row in rows {
        let (spell_id, effect) = row.unwrap();
        map.entry(spell_id).or_default().push(effect);
    }
    map
}

/// Load power costs grouped by SpellID (first OrderIndex=0 entry only).
fn load_power_costs(conn: &Connection) -> HashMap<u32, (i32, u32)> {
    let mut stmt = conn
        .prepare(
            "SELECT SpellID, PowerType, ManaCost
             FROM spell_power
             WHERE OrderIndex = 0 AND ManaCost > 0",
        )
        .expect("Failed to prepare spell_power query");

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, u32>(2)?,
            ))
        })
        .expect("Failed to query spell_power");

    let mut map = HashMap::new();
    for row in rows {
        let (spell_id, power_type, mana_cost) = row.unwrap();
        map.insert(spell_id, (power_type, mana_cost));
    }
    map
}

/// Load spells from world.db into a SpellDatabase.
pub fn load_spell_database(conn: &Connection) -> SpellDatabase {
    let cast_times = load_cast_times(conn);
    let ranges = load_ranges(conn);
    let cooldowns = load_cooldowns(conn);
    let effects = load_effects(conn);
    let power_costs = load_power_costs(conn);

    // Join spell_name + spell_misc (DifficultyID=0) to get base spell data
    let mut stmt = conn
        .prepare(
            "SELECT n.ID, n.Name_lang, m.SchoolMask, m.CastingTimeIndex, m.RangeIndex,
                    m.Attributes_0, m.Speed
             FROM spell_name n
             INNER JOIN spell_misc m ON m.SpellID = n.ID AND m.DifficultyID = 0
             WHERE n.Name_lang != ''",
        )
        .expect("Failed to prepare spell query");

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,    // ID
                row.get::<_, String>(1)?, // Name
                row.get::<_, u32>(2)?,    // SchoolMask
                row.get::<_, u32>(3)?,    // CastingTimeIndex
                row.get::<_, u32>(4)?,    // RangeIndex
                row.get::<_, i64>(5)?,    // Attributes_0 (signed in DB)
                row.get::<_, f64>(6)?,    // Speed
            ))
        })
        .expect("Failed to query spells");

    let mut spells = HashMap::new();
    for row in rows {
        let (id, name, school_mask, cast_time_idx, range_idx, attributes_0_raw, _speed) =
            row.unwrap();
        let attributes_0 = attributes_0_raw as u32;

        // Build effects array (up to 3)
        let mut spell_effects: [Option<SpellEffectDef>; 3] = [None, None, None];
        let mut target = SpellTarget::Hostile;

        if let Some(effect_rows) = effects.get(&id) {
            for er in effect_rows.iter().take(3) {
                let idx = er.effect_index as usize;
                if idx < 3 {
                    spell_effects[idx] =
                        map_effect(er.effect_type, er.base_points, er.ap_coeff, er.sp_coeff);
                }
                // Use first effect's target for the spell target
                if er.effect_index == 0 {
                    target = target_from_implicit(er.implicit_target_0);
                }
            }
        }

        // Power cost
        let cost = power_costs.get(&id).and_then(|&(pt, amount)| {
            power_type(pt).map(|resource| SpellCost {
                resource,
                amount: amount as f32,
            })
        });

        // SPELL_ATTR0_CASTABLE_WHILE_MOVING = 0x10000000 (bit 28)
        let cast_while_moving = attributes_0 & 0x10000000 != 0;
        let cast_time = cast_times.get(&cast_time_idx).copied().unwrap_or(0.0);
        let range = ranges.get(&range_idx).copied().unwrap_or(30.0);
        let cooldown = cooldowns.get(&id).copied().unwrap_or(0.0);
        // Instant spells are always interruptible=false; cast-time spells default to true
        let interruptible = cast_time > 0.0;

        spells.insert(
            id,
            SpellData {
                id,
                name,
                school: school_from_mask(school_mask),
                cast_time,
                cooldown,
                cost,
                target,
                range,
                cast_while_moving,
                interruptible,
                effects: spell_effects,
            },
        );
    }

    SpellDatabase { spells }
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
    if let Some(spell) = db.get(12294) {
        debug!("Mortal Strike: {} (id={})", spell.name, spell.id);
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
    fn school_mask_fire() {
        assert_eq!(school_from_mask(0x04), SpellSchool::Fire);
    }

    #[test]
    fn school_mask_shadow() {
        assert_eq!(school_from_mask(0x20), SpellSchool::Shadow);
    }

    #[test]
    fn school_mask_arcane() {
        assert_eq!(school_from_mask(0x40), SpellSchool::Arcane);
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
    fn power_type_holy_power() {
        assert_eq!(power_type(9), Some(ResourceType::HolyPower));
    }

    #[test]
    fn power_type_unknown() {
        assert_eq!(power_type(99), None);
    }

    #[test]
    fn map_effect_school_damage() {
        let eff = map_effect(2, 500.0, 1.0, 0.8).unwrap();
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
        let eff = map_effect(10, 1000.0, 0.0, 1.2).unwrap();
        assert!(matches!(eff, SpellEffectDef::Heal { .. }));
    }

    #[test]
    fn map_effect_unknown_returns_none() {
        assert!(map_effect(999, 0.0, 0.0, 0.0).is_none());
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

    // --- Loading tests ---

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE spell_name (ID INTEGER PRIMARY KEY, Name_lang TEXT NOT NULL DEFAULT '');
             CREATE TABLE spell_misc (
                ID INTEGER PRIMARY KEY, Attributes_0 INTEGER NOT NULL DEFAULT 0,
                Attributes_1 INTEGER DEFAULT 0, Attributes_2 INTEGER DEFAULT 0,
                Attributes_3 INTEGER DEFAULT 0, Attributes_4 INTEGER DEFAULT 0,
                Attributes_5 INTEGER DEFAULT 0, Attributes_6 INTEGER DEFAULT 0,
                Attributes_7 INTEGER DEFAULT 0, Attributes_8 INTEGER DEFAULT 0,
                Attributes_9 INTEGER DEFAULT 0, Attributes_10 INTEGER DEFAULT 0,
                Attributes_11 INTEGER DEFAULT 0, Attributes_12 INTEGER DEFAULT 0,
                Attributes_13 INTEGER DEFAULT 0, Attributes_14 INTEGER DEFAULT 0,
                Attributes_15 INTEGER DEFAULT 0, Attributes_16 INTEGER DEFAULT 0,
                DifficultyID INTEGER NOT NULL DEFAULT 0,
                CastingTimeIndex INTEGER NOT NULL DEFAULT 1,
                DurationIndex INTEGER DEFAULT 0, PvPDurationIndex INTEGER DEFAULT 0,
                RangeIndex INTEGER NOT NULL DEFAULT 1,
                SchoolMask INTEGER NOT NULL DEFAULT 0,
                Speed REAL NOT NULL DEFAULT 0,
                LaunchDelay REAL DEFAULT 0, MinDuration REAL DEFAULT 0,
                SpellIconFileDataID INTEGER DEFAULT 0,
                ActiveIconFileDataID INTEGER DEFAULT 0,
                ContentTuningID INTEGER DEFAULT 0,
                ShowFutureSpellPlayerConditionID INTEGER DEFAULT 0,
                SpellVisualScript INTEGER DEFAULT 0,
                ActiveSpellVisualScript INTEGER DEFAULT 0,
                SpellID INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE spell_effect (
                ID INTEGER PRIMARY KEY, EffectAura INTEGER DEFAULT 0,
                DifficultyID INTEGER NOT NULL DEFAULT 0,
                EffectIndex INTEGER NOT NULL DEFAULT 0,
                Effect INTEGER NOT NULL DEFAULT 0,
                EffectAmplitude REAL DEFAULT 0, EffectAttributes INTEGER DEFAULT 0,
                EffectAuraPeriod INTEGER DEFAULT 0,
                EffectBonusCoefficient REAL NOT NULL DEFAULT 0,
                EffectChainAmplitude REAL DEFAULT 0, EffectChainTargets INTEGER DEFAULT 0,
                EffectItemType INTEGER DEFAULT 0, EffectMechanic INTEGER DEFAULT 0,
                EffectPointsPerResource REAL DEFAULT 0, EffectPos_facing REAL DEFAULT 0,
                EffectRealPointsPerLevel REAL DEFAULT 0,
                EffectTriggerSpell INTEGER DEFAULT 0,
                BonusCoefficientFromAP REAL NOT NULL DEFAULT 0,
                PvpMultiplier REAL DEFAULT 0, Coefficient REAL DEFAULT 0,
                Variance REAL DEFAULT 0, ResourceCoefficient REAL DEFAULT 0,
                GroupSizeBasePointsCoefficient REAL DEFAULT 0,
                EffectBasePointsF REAL NOT NULL DEFAULT 0,
                ScalingClass INTEGER DEFAULT 0,
                Node__Field_12_0_0_63534_001 INTEGER DEFAULT 0,
                EffectMiscValue_0 INTEGER DEFAULT 0, EffectMiscValue_1 INTEGER DEFAULT 0,
                EffectRadiusIndex_0 INTEGER DEFAULT 0, EffectRadiusIndex_1 INTEGER DEFAULT 0,
                EffectSpellClassMask_0 INTEGER DEFAULT 0, EffectSpellClassMask_1 INTEGER DEFAULT 0,
                EffectSpellClassMask_2 INTEGER DEFAULT 0, EffectSpellClassMask_3 INTEGER DEFAULT 0,
                ImplicitTarget_0 INTEGER NOT NULL DEFAULT 0,
                ImplicitTarget_1 INTEGER DEFAULT 0,
                SpellID INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE spell_power (
                ID INTEGER PRIMARY KEY, OrderIndex INTEGER NOT NULL DEFAULT 0,
                ManaCost INTEGER NOT NULL DEFAULT 0,
                ManaCostPerLevel INTEGER DEFAULT 0, ManaPerSecond INTEGER DEFAULT 0,
                PowerDisplayID INTEGER DEFAULT 0, AltPowerBarID INTEGER DEFAULT 0,
                PowerCostPct REAL DEFAULT 0, PowerCostMaxPct REAL DEFAULT 0,
                OptionalCostPct REAL DEFAULT 0, PowerPctPerSecond REAL DEFAULT 0,
                PowerType INTEGER NOT NULL DEFAULT 0,
                RequiredAuraSpellID INTEGER DEFAULT 0, OptionalCost INTEGER DEFAULT 0,
                SpellID INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE spell_cast_times (ID INTEGER PRIMARY KEY, Base INTEGER NOT NULL DEFAULT 0, Minimum INTEGER NOT NULL DEFAULT 0);
             CREATE TABLE spell_range (ID INTEGER PRIMARY KEY, DisplayName_lang TEXT DEFAULT '', DisplayNameShort_lang TEXT DEFAULT '', Flags INTEGER DEFAULT 0, RangeMin_0 REAL DEFAULT 0, RangeMin_1 REAL DEFAULT 0, RangeMax_0 REAL NOT NULL DEFAULT 0, RangeMax_1 REAL DEFAULT 0);
             CREATE TABLE spell_cooldowns (ID INTEGER PRIMARY KEY, DifficultyID INTEGER NOT NULL DEFAULT 0, CategoryRecoveryTime INTEGER DEFAULT 0, RecoveryTime INTEGER DEFAULT 0, StartRecoveryTime INTEGER DEFAULT 0, AuraSpellID INTEGER DEFAULT 0, SpellID INTEGER NOT NULL DEFAULT 0);
             CREATE TABLE spell_duration (ID INTEGER PRIMARY KEY, Duration INTEGER NOT NULL DEFAULT 0, MaxDuration INTEGER DEFAULT 0, DurationPerResource INTEGER DEFAULT 0);
             -- Seed lookup data used by tests
             INSERT INTO spell_cast_times VALUES (1, 0, 0);      -- instant
             INSERT INTO spell_cast_times VALUES (4, 1000, 1000); -- 1.0s
             INSERT INTO spell_cast_times VALUES (5, 1500, 1500); -- 1.5s
             INSERT INTO spell_cast_times VALUES (14, 3000, 3000); -- 3.0s
             INSERT INTO spell_range VALUES (1, 'Self', 'Self', 0, 0, 0, 0, 0);     -- self
             INSERT INTO spell_range VALUES (2, 'Melee', 'Melee', 0, 0, 0, 5, 5);   -- melee
             INSERT INTO spell_range VALUES (5, 'Long', 'Long', 0, 0, 0, 40, 40);   -- 40y
             ;",
        )
        .unwrap();
        conn
    }

    fn insert_spell(conn: &Connection, id: u32, name: &str, school: u32) {
        conn.execute(
            "INSERT INTO spell_name VALUES (?1, ?2)",
            rusqlite::params![id, name],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO spell_misc (ID, DifficultyID, CastingTimeIndex, RangeIndex, SchoolMask, Speed, Attributes_0, SpellID)
             VALUES (?1, 0, 1, 5, ?2, 0, 0, ?1)",
            rusqlite::params![id, school],
        )
        .unwrap();
    }

    fn insert_effect(
        conn: &Connection,
        effect_id: u32,
        spell_id: u32,
        index: u32,
        effect_type: u32,
        base_points: f32,
        sp_coeff: f32,
        ap_coeff: f32,
        target: u32,
    ) {
        conn.execute(
            "INSERT INTO spell_effect (ID, DifficultyID, EffectIndex, Effect, EffectBasePointsF,
             EffectBonusCoefficient, BonusCoefficientFromAP, ImplicitTarget_0, SpellID)
             VALUES (?1, 0, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                effect_id,
                index,
                effect_type,
                base_points,
                sp_coeff,
                ap_coeff,
                target,
                spell_id
            ],
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
        insert_spell(&conn, 585, "Smite", 0x02);
        insert_effect(&conn, 1, 585, 0, 2, 200.0, 0.7, 0.0, 6);
        let db = load_spell_database(&conn);
        assert_eq!(db.len(), 1);
        let spell = db.get(585).unwrap();
        assert_eq!(spell.name, "Smite");
        assert_eq!(spell.school, SpellSchool::Holy);
    }

    #[test]
    fn load_spell_with_ap_coefficient() {
        let conn = test_db();
        insert_spell(&conn, 12294, "Mortal Strike", 0x01);
        insert_effect(&conn, 1, 12294, 0, 2, 0.0, 0.0, 5.8, 6);
        let db = load_spell_database(&conn);
        let spell = db.get(12294).unwrap();
        match &spell.effects[0] {
            Some(SpellEffectDef::SchoolDamage { ap_coefficient, .. }) => {
                assert!((ap_coefficient - 5.8).abs() < 0.001);
            }
            other => panic!("expected SchoolDamage, got {other:?}"),
        }
    }

    #[test]
    fn load_spell_with_sp_coefficient() {
        let conn = test_db();
        insert_spell(&conn, 585, "Smite", 0x02);
        insert_effect(&conn, 1, 585, 0, 2, 200.0, 0.7143, 0.0, 6);
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
    fn load_spell_without_effects_has_none() {
        let conn = test_db();
        insert_spell(&conn, 100, "NoEffect", 0x01);
        let db = load_spell_database(&conn);
        let spell = db.get(100).unwrap();
        assert!(spell.effects[0].is_none());
        assert!(spell.effects[1].is_none());
        assert!(spell.effects[2].is_none());
    }

    #[test]
    fn load_multiple_spells() {
        let conn = test_db();
        insert_spell(&conn, 1, "Spell1", 0x01);
        insert_spell(&conn, 2, "Spell2", 0x20);
        insert_spell(&conn, 3, "Spell3", 0x40);
        let db = load_spell_database(&conn);
        assert_eq!(db.len(), 3);
        assert_eq!(db.get(1).unwrap().school, SpellSchool::Physical);
        assert_eq!(db.get(2).unwrap().school, SpellSchool::Shadow);
        assert_eq!(db.get(3).unwrap().school, SpellSchool::Arcane);
    }

    #[test]
    fn load_spell_heal_effect() {
        let conn = test_db();
        insert_spell(&conn, 1, "Heal", 0x02);
        // Override to cast time index 5 (1.5s) and self-target range index 1
        conn.execute(
            "UPDATE spell_misc SET CastingTimeIndex = 5, RangeIndex = 1 WHERE ID = 1",
            [],
        )
        .unwrap();
        insert_effect(&conn, 1, 1, 0, 10, 800.0, 0.0, 0.0, 1);
        let db = load_spell_database(&conn);
        let spell = db.get(1).unwrap();
        assert!(matches!(
            spell.effects[0],
            Some(SpellEffectDef::Heal { .. })
        ));
        assert_eq!(spell.target, SpellTarget::Self_);
        assert_eq!(spell.cast_time, 1.5); // CastingTimeIndex 5 = 1500ms
        assert!(spell.interruptible); // has cast time
    }

    #[test]
    fn load_spell_with_power_cost() {
        let conn = test_db();
        insert_spell(&conn, 1, "Costly", 0x01);
        conn.execute(
            "INSERT INTO spell_power (ID, OrderIndex, ManaCost, PowerType, SpellID) VALUES (1, 0, 300, 1, 1)",
            [],
        )
        .unwrap();
        let db = load_spell_database(&conn);
        let spell = db.get(1).unwrap();
        let cost = spell.cost.as_ref().unwrap();
        assert_eq!(cost.resource, ResourceType::Rage);
        assert_eq!(cost.amount, 300.0);
    }

    #[test]
    fn load_spell_zero_cost_is_none() {
        let conn = test_db();
        insert_spell(&conn, 1, "Free", 0x01);
        let db = load_spell_database(&conn);
        assert!(db.get(1).unwrap().cost.is_none());
    }

    #[test]
    fn load_spell_empty_name_excluded() {
        let conn = test_db();
        insert_spell(&conn, 1, "Valid", 0x01);
        conn.execute("INSERT INTO spell_name VALUES (2, '')", [])
            .unwrap();
        conn.execute(
            "INSERT INTO spell_misc (ID, DifficultyID, CastingTimeIndex, RangeIndex, SchoolMask, Speed, Attributes_0, SpellID)
             VALUES (2, 0, 1, 1, 1, 0, 0, 2)", [],
        ).unwrap();
        let db = load_spell_database(&conn);
        assert_eq!(db.len(), 1);
        assert!(db.get(2).is_none());
    }

    #[test]
    fn load_spell_unsupported_effect_is_none() {
        let conn = test_db();
        insert_spell(&conn, 1, "Weird", 0x01);
        insert_effect(&conn, 1, 1, 0, 999, 0.0, 0.0, 0.0, 6);
        let db = load_spell_database(&conn);
        let spell = db.get(1).unwrap();
        assert!(spell.effects[0].is_none());
    }

    #[test]
    fn load_spell_range_from_index() {
        let conn = test_db();
        // RangeIndex=5 → 40y (set by insert_spell default)
        insert_spell(&conn, 1, "Ranged", 0x01);
        let db = load_spell_database(&conn);
        assert_eq!(db.get(1).unwrap().range, 40.0);
    }

    #[test]
    fn load_spell_with_cooldown() {
        let conn = test_db();
        insert_spell(&conn, 1, "Strike", 0x01);
        conn.execute(
            "INSERT INTO spell_cooldowns (ID, DifficultyID, RecoveryTime, CategoryRecoveryTime, SpellID)
             VALUES (1, 0, 6000, 0, 1)",
            [],
        )
        .unwrap();
        let db = load_spell_database(&conn);
        assert_eq!(db.get(1).unwrap().cooldown, 6.0);
    }

    #[test]
    fn load_spell_no_cooldown_entry() {
        let conn = test_db();
        insert_spell(&conn, 1, "Free", 0x01);
        let db = load_spell_database(&conn);
        assert_eq!(db.get(1).unwrap().cooldown, 0.0);
    }

    #[test]
    fn load_spell_category_cooldown_wins() {
        let conn = test_db();
        insert_spell(&conn, 1, "CatCD", 0x01);
        conn.execute(
            "INSERT INTO spell_cooldowns (ID, DifficultyID, RecoveryTime, CategoryRecoveryTime, SpellID)
             VALUES (1, 0, 1000, 7500, 1)",
            [],
        )
        .unwrap();
        let db = load_spell_database(&conn);
        // max(1000, 7500) = 7500ms = 7.5s
        assert_eq!(db.get(1).unwrap().cooldown, 7.5);
    }

    #[test]
    fn get_missing_spell_returns_none() {
        let conn = test_db();
        let db = load_spell_database(&conn);
        assert!(db.get(99999).is_none());
    }

    #[test]
    fn load_spell_difficulty_zero_only() {
        let conn = test_db();
        insert_spell(&conn, 1, "Normal", 0x01);
        // Insert a difficulty variant — should be ignored
        conn.execute(
            "INSERT INTO spell_misc (ID, DifficultyID, CastingTimeIndex, RangeIndex, SchoolMask, Speed, Attributes_0, SpellID)
             VALUES (999, 1, 1, 1, 0x20, 0, 0, 1)", [],
        ).unwrap();
        insert_effect(&conn, 1, 1, 0, 2, 100.0, 0.0, 0.0, 6);
        // Insert difficulty effect — should be ignored
        conn.execute(
            "INSERT INTO spell_effect (ID, DifficultyID, EffectIndex, Effect, EffectBasePointsF,
             EffectBonusCoefficient, BonusCoefficientFromAP, ImplicitTarget_0, SpellID)
             VALUES (2, 1, 0, 2, 9999.0, 0, 0, 6, 1)",
            [],
        )
        .unwrap();
        let db = load_spell_database(&conn);
        let spell = db.get(1).unwrap();
        // Should use DifficultyID=0 values
        assert_eq!(spell.school, SpellSchool::Physical);
        match &spell.effects[0] {
            Some(SpellEffectDef::SchoolDamage { base_min, .. }) => {
                assert_eq!(*base_min, 100.0); // not 9999.0
            }
            other => panic!("expected SchoolDamage, got {other:?}"),
        }
    }
}
