//! TC a352b1fa SpellMgr::LoadSpellInfoServerside (SpellMgr.cpp): TDB
//! `serverside_spell` / `serverside_spell_effect` rows, retained unchanged as
//! `tdb_serverside_spell*` by `scripts/encounter_serverside_spells.py`.
//! Copies without those tables have no server-side spells. Only DifficultyID 0
//! is loaded, like the DB2 store. A server-side ID or effect that names a DB2
//! spell is refused, as TC refuses overriding file spells.

use bevy::prelude::warn;
use rusqlite::{Connection, Row};

use super::{Lookups, Spells, int, lookup, real, sint, uint};
use crate::spell_info::*;

const SPELLS: &str = "tdb_serverside_spell";
const EFFECTS: &str = "tdb_serverside_spell_effect";

/// Insert every server-side spell into the sorted `spells`; returns how many.
pub(super) fn load_serverside_spells(
    conn: &Connection,
    lookups: &Lookups,
    spells: &mut Spells,
) -> rusqlite::Result<usize> {
    let file_ids: Vec<u32> = spells.iter().map(|spell| spell.id).collect();
    let loaded = read_additional_spells(conn, lookups, &file_ids)?;
    let count = loaded.len();
    for spell in loaded {
        let index = spells
            .binary_search_by_key(&spell.id, |existing| existing.id)
            .expect_err("serverside spell IDs were checked against DB2");
        spells.insert(index, spell);
    }
    Ok(count)
}

pub(super) fn read_additional_spells(
    conn: &Connection,
    lookups: &Lookups,
    file_ids: &[u32],
) -> rusqlite::Result<Vec<SpellInfo>> {
    if !table_exists(conn, SPELLS)? {
        return Ok(Vec::new());
    }
    let mut loaded = read_spells(conn, lookups, file_ids)?;
    for effect_row in read_effects(conn, lookups)? {
        let (spell_id, effect) = effect_row;
        if file_ids.binary_search(&spell_id).is_ok() {
            warn!("Serverside spell {spell_id} effect references a DB2 spell, skipped");
            continue;
        }
        if let Some(spell) = loaded.iter_mut().find(|spell| spell.id == spell_id) {
            spell.effects.push(effect);
        }
    }
    Ok(loaded)
}

fn table_exists(conn: &Connection, name: &str) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        [name],
        |row| row.get(0),
    )
}

fn read_spells(
    conn: &Connection,
    lookups: &Lookups,
    file_ids: &[u32],
) -> rusqlite::Result<Vec<SpellInfo>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT s.*, COALESCE(c.MaxCharges, 0) AS MaxCharges,
                COALESCE(c.ChargeRecoveryTime, 0) AS ChargeRecoveryTime,
                COALESCE(c.TypeMask, 0) AS ChargeCategoryTypeMask
         FROM {SPELLS} s LEFT JOIN spell_category c ON c.ID = s.ChargeCategoryId
         WHERE s.DifficultyID = 0 ORDER BY s.Id"
    ))?;
    let mut rows = stmt.query([])?;
    let mut spells = Vec::new();
    while let Some(row) = rows.next()? {
        let id = col_u32(row, "Id")?;
        if file_ids.binary_search(&id).is_ok() {
            warn!("Serverside spell {id} is already loaded from DB2, skipped");
            continue;
        }
        spells.push(read_spell(row, lookups, id)?);
    }
    Ok(spells)
}

fn read_spell(row: &Row, lookups: &Lookups, id: u32) -> rusqlite::Result<SpellInfo> {
    let mut attributes = [0; 17];
    attributes[0] = col_u32(row, "Attributes")?;
    for (index, value) in attributes.iter_mut().enumerate().skip(1) {
        *value = col_u32(row, &format!("AttributesEx{}", ex_suffix(index)))?;
    }
    let stances = col_i64(row, "Stances")? as u64;
    let stances_not = col_i64(row, "StancesNot")? as u64;
    let proc_ppm = col_f32(row, "ProcBasePPM")?;
    Ok(SpellInfo {
        id,
        name: row.get("SpellName")?,
        misc: Some(SpellMisc {
            attributes,
            school_mask: col_u32(row, "SchoolMask")?,
            cast_time: lookup(&lookups.cast_times, col_u32(row, "CastingTimeIndex")?),
            duration: lookup(&lookups.durations, col_u32(row, "DurationIndex")?),
            pvp_duration: None,
            range: lookup(&lookups.ranges, col_u32(row, "RangeIndex")?),
            speed: col_f32(row, "Speed")?,
            launch_delay: col_f32(row, "LaunchDelay")?,
            min_duration: 0.0,
            icon_file_data_id: 0,
            active_icon_file_data_id: 0,
            content_tuning_id: col_u32(row, "ContentTuningId")?,
        }),
        aura_options: present(SpellAuraOptions {
            proc_definition_source: None,
            proc_family: None,
            cumulative_aura: col_u32(row, "StackAmount")?,
            proc_category_recovery_ms: col_i32(row, "ProcCooldown")?,
            proc_chance: col_u32(row, "ProcChance")?,
            proc_charges: col_i32(row, "ProcCharges")?,
            procs_per_minute_id: 0,
            procs_per_minute: (proc_ppm != 0.0).then_some(ProcsPerMinute {
                base_proc_rate: proc_ppm,
                flags: 0,
                ..Default::default()
            }),
            proc_type_mask: [col_u32(row, "ProcFlags")?, col_u32(row, "ProcFlags2")?],
        }),
        categories: present(SpellCategories {
            category: col_u32(row, "CategoryId")?,
            start_recovery_category: col_u32(row, "StartRecoveryCategory")?,
            charge_category: col_u32(row, "ChargeCategoryId")?,
            defense_type: col_u32(row, "DmgClass")?,
            diminish_type: 0,
            dispel_type: col_u32(row, "Dispel")?,
            mechanic: col_u32(row, "Mechanic")?,
            prevention_type: col_u32(row, "PreventionType")?,
            max_charges: col_u32(row, "MaxCharges")?,
            charge_recovery_ms: col_u32(row, "ChargeRecoveryTime")?,
            charge_type_mask: col_u32(row, "ChargeCategoryTypeMask")?,
        }),
        cooldowns: present(SpellCooldownInfo {
            recovery_ms: col_u32(row, "RecoveryTime")?,
            category_recovery_ms: col_u32(row, "CategoryRecoveryTime")?,
            start_recovery_ms: col_u32(row, "StartRecoveryTime")?,
            aura_spell_id: 0,
        }),
        levels: present(SpellLevels {
            base_level: col_u32(row, "BaseLevel")?,
            max_level: col_u32(row, "MaxLevel")?,
            spell_level: col_u32(row, "SpellLevel")?,
            max_passive_aura_level: 0,
        }),
        class_options: present(SpellClassOptions {
            spell_class_set: col_u32(row, "SpellFamilyName")?,
            spell_class_mask: [
                col_u32(row, "SpellFamilyFlags1")?,
                col_u32(row, "SpellFamilyFlags2")?,
                col_u32(row, "SpellFamilyFlags3")?,
                col_u32(row, "SpellFamilyFlags4")?,
            ],
            modal_next_spell: 0,
        }),
        shapeshift: present(SpellShapeshift {
            stance_bar_order: 0,
            shapeshift_exclude: split_u64(stances_not),
            shapeshift_mask: split_u64(stances),
        }),
        interrupts: present(SpellInterrupts {
            interrupt_flags: col_u32(row, "InterruptFlags")?,
            aura_interrupt_flags: [
                col_u32(row, "AuraInterruptFlags1")?,
                col_u32(row, "AuraInterruptFlags2")?,
            ],
            channel_interrupt_flags: [
                col_u32(row, "ChannelInterruptFlags1")?,
                col_u32(row, "ChannelInterruptFlags2")?,
            ],
        }),
        target_restrictions: present(SpellTargetRestrictions {
            cone_degrees: col_f32(row, "ConeAngle")?,
            max_targets: col_u32(row, "MaxAffectedTargets")?,
            max_target_level: col_u32(row, "MaxTargetLevel")?,
            target_creature_type: col_u32(row, "TargetCreatureType")?,
            targets: col_u32(row, "Targets")?,
            width: col_f32(row, "ConeWidth")?,
        }),
        equipped_items: equipped_items(row)?,
        ..SpellInfo::default()
    })
}

/// `AttributesEx`, `AttributesEx2` … `AttributesEx16` for attribute words 1..16.
fn ex_suffix(index: usize) -> String {
    if index == 1 {
        String::new()
    } else {
        index.to_string()
    }
}

/// TC keeps every source field on SpellInfo; an all-zero group equals an absent DB2 row.
fn present<T: Default + PartialEq>(value: T) -> Option<Box<T>> {
    (value != T::default()).then(|| Box::new(value))
}

/// `EquippedItemClass` -1 is no requirement, like an absent `spell_equipped_items` row.
fn equipped_items(row: &Row) -> rusqlite::Result<Option<Box<SpellEquippedItems>>> {
    let item_class = col_i32(row, "EquippedItemClass")?;
    if item_class < 0 {
        return Ok(None);
    }
    Ok(Some(Box::new(SpellEquippedItems {
        item_class,
        inventory_types: col_u32(row, "EquippedItemInventoryTypeMask")?,
        item_subclass: col_u32(row, "EquippedItemSubClassMask")?,
    })))
}

fn split_u64(value: u64) -> [u32; 2] {
    [value as u32, (value >> 32) as u32]
}

fn read_effects(conn: &Connection, lookups: &Lookups) -> rusqlite::Result<Vec<(u32, SpellEffect)>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT * FROM {EFFECTS} WHERE DifficultyID = 0 ORDER BY SpellID, EffectIndex"
    ))?;
    let mut rows = stmt.query([])?;
    let mut effects = Vec::new();
    while let Some(row) = rows.next()? {
        effects.push((col_u32(row, "SpellID")?, read_effect(row, lookups)?));
    }
    Ok(effects)
}

fn read_effect(row: &Row, lookups: &Lookups) -> rusqlite::Result<SpellEffect> {
    Ok(SpellEffect {
        index: col_u32(row, "EffectIndex")? as u8,
        immunity: None,
        effect: col_u32(row, "Effect")?,
        aura: col_u32(row, "EffectAura")?,
        base_points: col_f32(row, "EffectBasePoints")?,
        bonus_coefficient: col_f32(row, "EffectBonusCoefficient")?,
        bonus_coefficient_from_ap: col_f32(row, "BonusCoefficientFromAP")?,
        amplitude: col_f32(row, "EffectAmplitude")?,
        aura_period_ms: col_u32(row, "EffectAuraPeriod")?,
        chain_amplitude: col_f32(row, "EffectChainAmplitude")?,
        chain_targets: col_i32(row, "EffectChainTargets")?,
        item_type: col_u32(row, "EffectItemType")?,
        mechanic: col_u32(row, "EffectMechanic")?,
        points_per_resource: col_f32(row, "EffectPointsPerResource")?,
        real_points_per_level: col_f32(row, "EffectRealPointsPerLevel")?,
        pos_facing: col_f32(row, "EffectPosFacing")?,
        trigger_spell: col_u32(row, "EffectTriggerSpell")?,
        pvp_multiplier: col_f32(row, "PvpMultiplier")?,
        coefficient: col_f32(row, "Coefficient")?,
        variance: col_f32(row, "Variance")?,
        resource_coefficient: col_f32(row, "ResourceCoefficient")?,
        group_size_base_points_coefficient: col_f32(row, "GroupSizeBasePointsCoefficient")?,
        scaling_class: 0,
        attributes: col_u32(row, "EffectAttributes")?,
        misc_values: [
            col_i32(row, "EffectMiscValue1")?,
            col_i32(row, "EffectMiscValue2")?,
        ],
        radius: [
            lookup(&lookups.radii, col_u32(row, "EffectRadiusIndex1")?),
            lookup(&lookups.radii, col_u32(row, "EffectRadiusIndex2")?),
        ],
        spell_class_mask: [
            col_u32(row, "EffectSpellClassMask1")?,
            col_u32(row, "EffectSpellClassMask2")?,
            col_u32(row, "EffectSpellClassMask3")?,
            col_u32(row, "EffectSpellClassMask4")?,
        ],
        implicit_targets: [
            col_u32(row, "ImplicitTarget1")?,
            col_u32(row, "ImplicitTarget2")?,
        ],
    })
}

fn column(row: &Row, name: &str) -> rusqlite::Result<usize> {
    row.as_ref().column_index(name)
}

fn col_i64(row: &Row, name: &str) -> rusqlite::Result<i64> {
    int(row, column(row, name)?)
}

fn col_u32(row: &Row, name: &str) -> rusqlite::Result<u32> {
    uint(row, column(row, name)?)
}

fn col_i32(row: &Row, name: &str) -> rusqlite::Result<i32> {
    sint(row, column(row, name)?)
}

fn col_f32(row: &Row, name: &str) -> rusqlite::Result<f32> {
    real(row, column(row, name)?)
}
