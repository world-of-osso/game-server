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

fn create_spell_test_schema(conn: &Connection) {
    create_spell_name_table(conn);
    create_spell_misc_table(conn);
    create_spell_effect_table(conn);
    create_spell_power_table(conn);
    create_spell_lookup_tables(conn);
}

fn create_spell_name_table(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE spell_name (
            ID INTEGER PRIMARY KEY,
            Name_lang TEXT NOT NULL DEFAULT ''
        );",
    )
    .unwrap();
}

fn create_spell_misc_table(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE spell_misc (
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
        );",
    )
    .unwrap();
}

fn create_spell_effect_table(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE spell_effect (
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
        );",
    )
    .unwrap();
}

fn create_spell_power_table(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE spell_power (
            ID INTEGER PRIMARY KEY, OrderIndex INTEGER NOT NULL DEFAULT 0,
            ManaCost INTEGER NOT NULL DEFAULT 0,
            ManaCostPerLevel INTEGER DEFAULT 0, ManaPerSecond INTEGER DEFAULT 0,
            PowerDisplayID INTEGER DEFAULT 0, AltPowerBarID INTEGER DEFAULT 0,
            PowerCostPct REAL DEFAULT 0, PowerCostMaxPct REAL DEFAULT 0,
            OptionalCostPct REAL DEFAULT 0, PowerPctPerSecond REAL DEFAULT 0,
            PowerType INTEGER NOT NULL DEFAULT 0,
            RequiredAuraSpellID INTEGER DEFAULT 0, OptionalCost INTEGER DEFAULT 0,
            SpellID INTEGER NOT NULL DEFAULT 0
        );",
    )
    .unwrap();
}

fn create_spell_lookup_tables(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE spell_cast_times (
            ID INTEGER PRIMARY KEY,
            Base INTEGER NOT NULL DEFAULT 0,
            Minimum INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE spell_range (
            ID INTEGER PRIMARY KEY,
            DisplayName_lang TEXT DEFAULT '',
            DisplayNameShort_lang TEXT DEFAULT '',
            Flags INTEGER DEFAULT 0,
            RangeMin_0 REAL DEFAULT 0,
            RangeMin_1 REAL DEFAULT 0,
            RangeMax_0 REAL NOT NULL DEFAULT 0,
            RangeMax_1 REAL DEFAULT 0
        );
        CREATE TABLE spell_cooldowns (
            ID INTEGER PRIMARY KEY,
            DifficultyID INTEGER NOT NULL DEFAULT 0,
            CategoryRecoveryTime INTEGER DEFAULT 0,
            RecoveryTime INTEGER DEFAULT 0,
            StartRecoveryTime INTEGER DEFAULT 0,
            AuraSpellID INTEGER DEFAULT 0,
            SpellID INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE spell_duration (
            ID INTEGER PRIMARY KEY,
            Duration INTEGER NOT NULL DEFAULT 0,
            MaxDuration INTEGER DEFAULT 0,
            DurationPerResource INTEGER DEFAULT 0
        );",
    )
    .unwrap();
}

fn seed_spell_test_lookups(conn: &Connection) {
    conn.execute_batch(
        "INSERT INTO spell_cast_times VALUES (1, 0, 0);
        INSERT INTO spell_cast_times VALUES (4, 1000, 1000);
        INSERT INTO spell_cast_times VALUES (5, 1500, 1500);
        INSERT INTO spell_cast_times VALUES (14, 3000, 3000);
        INSERT INTO spell_range VALUES (1, 'Self', 'Self', 0, 0, 0, 0, 0);
        INSERT INTO spell_range VALUES (2, 'Melee', 'Melee', 0, 0, 0, 5, 5);
        INSERT INTO spell_range VALUES (5, 'Long', 'Long', 0, 0, 0, 40, 40);",
    )
    .unwrap();
}

fn test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    create_spell_test_schema(&conn);
    seed_spell_test_lookups(&conn);
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

struct EffectInsert {
    effect_id: u32,
    spell_id: u32,
    index: u32,
    effect_type: u32,
    base_points: f32,
    sp_coeff: f32,
    ap_coeff: f32,
    target: u32,
}

fn insert_effect(conn: &Connection, effect: EffectInsert) {
    conn.execute(
        "INSERT INTO spell_effect (ID, DifficultyID, EffectIndex, Effect, EffectBasePointsF,
         EffectBonusCoefficient, BonusCoefficientFromAP, ImplicitTarget_0, SpellID)
         VALUES (?1, 0, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            effect.effect_id,
            effect.index,
            effect.effect_type,
            effect.base_points,
            effect.sp_coeff,
            effect.ap_coeff,
            effect.target,
            effect.spell_id
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
    insert_effect(
        &conn,
        EffectInsert {
            effect_id: 1,
            spell_id: 585,
            index: 0,
            effect_type: 2,
            base_points: 200.0,
            sp_coeff: 0.7,
            ap_coeff: 0.0,
            target: 6,
        },
    );
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
    insert_effect(
        &conn,
        EffectInsert {
            effect_id: 1,
            spell_id: 12294,
            index: 0,
            effect_type: 2,
            base_points: 0.0,
            sp_coeff: 0.0,
            ap_coeff: 5.8,
            target: 6,
        },
    );
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
    insert_effect(
        &conn,
        EffectInsert {
            effect_id: 1,
            spell_id: 585,
            index: 0,
            effect_type: 2,
            base_points: 200.0,
            sp_coeff: 0.7143,
            ap_coeff: 0.0,
            target: 6,
        },
    );
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
    conn.execute(
        "UPDATE spell_misc SET CastingTimeIndex = 5, RangeIndex = 1 WHERE ID = 1",
        [],
    )
    .unwrap();
    insert_effect(
        &conn,
        EffectInsert {
            effect_id: 1,
            spell_id: 1,
            index: 0,
            effect_type: 10,
            base_points: 800.0,
            sp_coeff: 0.0,
            ap_coeff: 0.0,
            target: 1,
        },
    );
    let db = load_spell_database(&conn);
    let spell = db.get(1).unwrap();
    assert!(matches!(
        spell.effects[0],
        Some(SpellEffectDef::Heal { .. })
    ));
    assert_eq!(spell.target, SpellTarget::Self_);
    assert_eq!(spell.cast_time, 1.5);
    assert!(spell.interruptible);
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
         VALUES (2, 0, 1, 1, 1, 0, 0, 2)",
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
    insert_spell(&conn, 1, "Weird", 0x01);
    insert_effect(
        &conn,
        EffectInsert {
            effect_id: 1,
            spell_id: 1,
            index: 0,
            effect_type: 999,
            base_points: 0.0,
            sp_coeff: 0.0,
            ap_coeff: 0.0,
            target: 6,
        },
    );
    let db = load_spell_database(&conn);
    let spell = db.get(1).unwrap();
    assert!(spell.effects[0].is_none());
}

#[test]
fn load_spell_range_from_index() {
    let conn = test_db();
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
    conn.execute(
        "INSERT INTO spell_misc (ID, DifficultyID, CastingTimeIndex, RangeIndex, SchoolMask, Speed, Attributes_0, SpellID)
         VALUES (999, 1, 1, 1, 0x20, 0, 0, 1)",
        [],
    )
    .unwrap();
    insert_effect(
        &conn,
        EffectInsert {
            effect_id: 1,
            spell_id: 1,
            index: 0,
            effect_type: 2,
            base_points: 100.0,
            sp_coeff: 0.0,
            ap_coeff: 0.0,
            target: 6,
        },
    );
    conn.execute(
        "INSERT INTO spell_effect (ID, DifficultyID, EffectIndex, Effect, EffectBasePointsF,
         EffectBonusCoefficient, BonusCoefficientFromAP, ImplicitTarget_0, SpellID)
         VALUES (2, 1, 0, 2, 9999.0, 0, 0, 6, 1)",
        [],
    )
    .unwrap();
    let db = load_spell_database(&conn);
    let spell = db.get(1).unwrap();
    assert_eq!(spell.school, SpellSchool::Physical);
    match &spell.effects[0] {
        Some(SpellEffectDef::SchoolDamage { base_min, .. }) => {
            assert_eq!(*base_min, 100.0);
        }
        other => panic!("expected SchoolDamage, got {other:?}"),
    }
}
