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
const SPELL_ATTR0_CASTABLE_WHILE_MOVING: u32 = 0x10000000;

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

struct SpellLookups {
    cast_times: HashMap<u32, f32>,
    ranges: HashMap<u32, f32>,
    cooldowns: HashMap<u32, f32>,
    effects: HashMap<u32, Vec<EffectRow>>,
    power_costs: HashMap<u32, (i32, u32)>,
}

struct SpellRow {
    id: u32,
    name: String,
    school_mask: u32,
    cast_time_idx: u32,
    range_idx: u32,
    attributes_0: u32,
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

fn load_spell_lookups(conn: &Connection) -> SpellLookups {
    SpellLookups {
        cast_times: load_cast_times(conn),
        ranges: load_ranges(conn),
        cooldowns: load_cooldowns(conn),
        effects: load_effects(conn),
        power_costs: load_power_costs(conn),
    }
}

fn load_spell_rows(conn: &Connection) -> Vec<SpellRow> {
    let mut stmt = conn
        .prepare(
            "SELECT n.ID, n.Name_lang, m.SchoolMask, m.CastingTimeIndex, m.RangeIndex,
                    m.Attributes_0
             FROM spell_name n
             INNER JOIN spell_misc m ON m.SpellID = n.ID AND m.DifficultyID = 0
             WHERE n.Name_lang != ''",
        )
        .expect("Failed to prepare spell query");

    let rows = stmt
        .query_map([], |row| {
            Ok(SpellRow {
                id: row.get(0)?,
                name: row.get(1)?,
                school_mask: row.get(2)?,
                cast_time_idx: row.get(3)?,
                range_idx: row.get(4)?,
                attributes_0: row.get::<_, i64>(5)? as u32,
            })
        })
        .expect("Failed to query spells");

    rows.map(|row| row.unwrap()).collect()
}

fn build_spell_effects(
    effect_rows: Option<&Vec<EffectRow>>,
) -> ([Option<SpellEffectDef>; 3], SpellTarget) {
    let mut spell_effects: [Option<SpellEffectDef>; 3] = [None, None, None];
    let mut target = SpellTarget::Hostile;

    if let Some(effect_rows) = effect_rows {
        for effect in effect_rows.iter().take(3) {
            let idx = effect.effect_index as usize;
            if idx < 3 {
                spell_effects[idx] = map_effect(
                    effect.effect_type,
                    effect.base_points,
                    effect.ap_coeff,
                    effect.sp_coeff,
                );
            }
            if effect.effect_index == 0 {
                target = target_from_implicit(effect.implicit_target_0);
            }
        }
    }

    (spell_effects, target)
}

fn load_spell_cost(lookups: &SpellLookups, spell_id: u32) -> Option<SpellCost> {
    lookups
        .power_costs
        .get(&spell_id)
        .and_then(|&(pt, amount)| {
            power_type(pt).map(|resource| SpellCost {
                resource,
                amount: amount as f32,
            })
        })
}

fn build_spell_data(row: SpellRow, lookups: &SpellLookups) -> SpellData {
    let (effects, target) = build_spell_effects(lookups.effects.get(&row.id));
    let cast_time = lookups
        .cast_times
        .get(&row.cast_time_idx)
        .copied()
        .unwrap_or(0.0);

    SpellData {
        id: row.id,
        name: row.name,
        school: school_from_mask(row.school_mask),
        cast_time,
        cooldown: lookups.cooldowns.get(&row.id).copied().unwrap_or(0.0),
        cost: load_spell_cost(lookups, row.id),
        target,
        range: lookups.ranges.get(&row.range_idx).copied().unwrap_or(30.0),
        cast_while_moving: row.attributes_0 & SPELL_ATTR0_CASTABLE_WHILE_MOVING != 0,
        interruptible: cast_time > 0.0,
        effects,
    }
}

/// Load spells from world.db into a SpellDatabase.
pub fn load_spell_database(conn: &Connection) -> SpellDatabase {
    let lookups = load_spell_lookups(conn);
    let spells = load_spell_rows(conn)
        .into_iter()
        .map(|row| (row.id, build_spell_data(row, &lookups)))
        .collect();
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
#[path = "spell_db_tests.rs"]
mod tests;
