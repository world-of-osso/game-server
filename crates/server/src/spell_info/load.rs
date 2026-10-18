//! SQL loaders for `SpellStore`. Every per-spell table query selects
//! `SpellID` first; rows for spells missing from `spell_name` are skipped.

mod serverside;

use std::collections::HashMap;

use rusqlite::{Connection, Row};

use super::*;

/// Sorted by spell ID (`spell_name` primary key order).
type Spells = Vec<SpellInfo>;

pub(super) fn int(row: &Row, idx: usize) -> rusqlite::Result<i64> {
    row.get(idx)
}

pub(super) fn uint(row: &Row, idx: usize) -> rusqlite::Result<u32> {
    Ok(int(row, idx)? as u32)
}

fn sint(row: &Row, idx: usize) -> rusqlite::Result<i32> {
    Ok(int(row, idx)? as i32)
}

fn real(row: &Row, idx: usize) -> rusqlite::Result<f32> {
    Ok(row.get::<_, f64>(idx)? as f32)
}

fn uints<const N: usize>(row: &Row, first: usize) -> rusqlite::Result<[u32; N]> {
    let mut values = [0; N];
    for (offset, value) in values.iter_mut().enumerate() {
        *value = uint(row, first + offset)?;
    }
    Ok(values)
}

/// Run `sql`, collecting `(key, value)` pairs from each row.
fn collect<K, V>(
    conn: &Connection,
    sql: &str,
    read: impl Fn(&Row) -> rusqlite::Result<(K, V)>,
) -> rusqlite::Result<Vec<(K, V)>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| read(row))?;
    rows.collect()
}

/// Run a per-spell query (column 0 = SpellID) and apply each row to its spell.
fn fill<T>(
    conn: &Connection,
    spells: &mut Spells,
    sql: &str,
    read: impl Fn(&Row) -> rusqlite::Result<T>,
    apply: impl Fn(&mut SpellInfo, T),
) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(sql)?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        if let Some(spell) = find_mut(spells, uint(row, 0)?) {
            apply(spell, read(row)?);
        }
    }
    Ok(())
}

fn find_mut(spells: &mut Spells, id: u32) -> Option<&mut SpellInfo> {
    let index = spells.binary_search_by_key(&id, |spell| spell.id).ok()?;
    Some(&mut spells[index])
}

/// Small index tables referenced by `spell_misc`, `spell_effect`, `spell_aura_options`.
struct Lookups {
    cast_times: HashMap<u32, SpellCastTime>,
    durations: HashMap<u32, SpellDuration>,
    ranges: HashMap<u32, SpellRange>,
    radii: HashMap<u32, SpellRadius>,
    procs_per_minute: HashMap<u32, ProcsPerMinute>,
}

impl Lookups {
    fn load(conn: &Connection) -> rusqlite::Result<Self> {
        let cast_times = collect(
            conn,
            "SELECT ID, Base, Minimum FROM spell_cast_times",
            |r| {
                Ok((
                    uint(r, 0)?,
                    SpellCastTime {
                        base_ms: sint(r, 1)?,
                        minimum_ms: sint(r, 2)?,
                    },
                ))
            },
        )?;
        let durations = collect(
            conn,
            "SELECT ID, Duration, MaxDuration, DurationPerResource FROM spell_duration",
            |r| {
                Ok((
                    uint(r, 0)?,
                    SpellDuration {
                        duration_ms: sint(r, 1)?,
                        max_duration_ms: sint(r, 2)?,
                        per_resource_ms: sint(r, 3)?,
                    },
                ))
            },
        )?;
        let ranges = collect(
            conn,
            "SELECT ID, RangeMin_0, RangeMin_1, RangeMax_0, RangeMax_1, Flags FROM spell_range",
            |r| {
                Ok((
                    uint(r, 0)?,
                    SpellRange {
                        min_hostile: real(r, 1)?,
                        min_friendly: real(r, 2)?,
                        max_hostile: real(r, 3)?,
                        max_friendly: real(r, 4)?,
                        flags: uint(r, 5)?,
                    },
                ))
            },
        )?;
        let radii = collect(
            conn,
            "SELECT ID, Radius, RadiusPerLevel, RadiusMin, RadiusMax FROM spell_radius",
            |r| {
                Ok((
                    uint(r, 0)?,
                    SpellRadius {
                        radius: real(r, 1)?,
                        radius_per_level: real(r, 2)?,
                        radius_min: real(r, 3)?,
                        radius_max: real(r, 4)?,
                    },
                ))
            },
        )?;
        let procs_per_minute = collect(
            conn,
            "SELECT ID, BaseProcRate, Flags FROM spell_procs_per_minute",
            |r| {
                Ok((
                    uint(r, 0)?,
                    ProcsPerMinute {
                        base_proc_rate: real(r, 1)?,
                        flags: uint(r, 2)?,
                    },
                ))
            },
        )?;
        Ok(Self {
            cast_times: cast_times.into_iter().collect(),
            durations: durations.into_iter().collect(),
            ranges: ranges.into_iter().collect(),
            radii: radii.into_iter().collect(),
            procs_per_minute: procs_per_minute.into_iter().collect(),
        })
    }
}

fn lookup<T: Copy>(table: &HashMap<u32, T>, id: u32) -> Option<T> {
    table.get(&id).copied()
}

pub(super) fn load_spell_store(conn: &Connection) -> rusqlite::Result<SpellStore> {
    let lookups = Lookups::load(conn)?;
    let mut spells = load_names(conn)?;
    load_misc(conn, &lookups, &mut spells)?;
    load_effects(conn, &lookups, &mut spells)?;
    super::immunity::attach_definitions(conn, &mut spells)?;
    load_power_costs(conn, &mut spells)?;
    load_aura_options(conn, &lookups, &mut spells)?;
    load_aura_restrictions(conn, &mut spells)?;
    load_categories(conn, &mut spells)?;
    load_cooldowns(conn, &mut spells)?;
    load_levels(conn, &mut spells)?;
    load_scaling(conn, &mut spells)?;
    load_class_options(conn, &mut spells)?;
    load_shapeshift(conn, &mut spells)?;
    load_interrupts(conn, &mut spells)?;
    load_target_restrictions(conn, &mut spells)?;
    load_equipped_items(conn, &mut spells)?;
    load_empower_stages(conn, &mut spells)?;
    load_labels(conn, &mut spells)?;
    load_spell_visuals(conn, &mut spells)?;
    serverside::load_serverside_spells(conn, &lookups, &mut spells)?;
    finish(&mut spells);
    Ok(SpellStore { spells })
}

/// Add a copy's server-side spells to an already loaded store.
#[cfg(test)]
pub(super) fn append_serverside_spells(
    conn: &Connection,
    store: &mut SpellStore,
) -> rusqlite::Result<usize> {
    let lookups = Lookups::load(conn)?;
    let count = serverside::load_serverside_spells(conn, &lookups, &mut store.spells)?;
    finish(&mut store.spells);
    Ok(count)
}

/// Effects are scanned in table order (an index-ordered scan is ~3x slower).
fn finish(spells: &mut Spells) {
    for spell in spells.iter_mut() {
        spell.effects.sort_unstable_by_key(|effect| effect.index);
        apply_azure_jump_destination_fix(spell);
    }
}

/// TC a352b1fa SpellMgr.cpp:4940–4956 ApplySpellFix: these two effect0 targets
/// are TARGET_DEST_DEST, retaining the script-supplied elevated destination.
fn apply_azure_jump_destination_fix(spell: &mut SpellInfo) {
    const TARGET_DEST_DEST: u32 = 87;
    if !matches!(spell.id, 387981 | 388082) {
        return;
    }
    if let Some(effect) = spell.effects.iter_mut().find(|effect| effect.index == 0) {
        effect.implicit_targets[0] = TARGET_DEST_DEST;
    }
}

fn load_spell_visuals(conn: &Connection, spells: &mut Spells) -> rusqlite::Result<()> {
    fill(
        conn,
        spells,
        "SELECT DISTINCT SpellID FROM spell_x_spell_visual WHERE DifficultyID = 0 AND SpellVisualID > 0",
        |_| Ok(()),
        |spell, ()| spell.has_spell_visual = true,
    )
}

fn load_names(conn: &Connection) -> rusqlite::Result<Spells> {
    let count: usize = conn.query_row("SELECT COUNT(*) FROM spell_name", [], |r| r.get(0))?;
    let mut spells = Spells::with_capacity(count);
    let mut stmt = conn.prepare(
        "SELECT n.ID, COALESCE(n.Name_lang, ''), COALESCE(s.NameSubtext_lang, '')
         FROM spell_name n LEFT JOIN spell s ON s.ID = n.ID ORDER BY n.ID",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(r) = rows.next()? {
        spells.push(SpellInfo {
            id: uint(r, 0)?,
            name: r.get(1)?,
            subtext: r.get(2)?,
            ..SpellInfo::default()
        });
    }
    Ok(spells)
}

fn load_misc(conn: &Connection, lookups: &Lookups, spells: &mut Spells) -> rusqlite::Result<()> {
    fill(
        conn,
        spells,
        "SELECT SpellID, Attributes_0, Attributes_1, Attributes_2, Attributes_3, Attributes_4,
                Attributes_5, Attributes_6, Attributes_7, Attributes_8, Attributes_9,
                Attributes_10, Attributes_11, Attributes_12, Attributes_13, Attributes_14,
                Attributes_15, Attributes_16, SchoolMask, CastingTimeIndex, DurationIndex,
                PvPDurationIndex, RangeIndex, Speed, LaunchDelay, MinDuration,
                SpellIconFileDataID, ActiveIconFileDataID, ContentTuningID
         FROM spell_misc WHERE DifficultyID = 0",
        |r| {
            Ok(SpellMisc {
                attributes: uints(r, 1)?,
                school_mask: uint(r, 18)?,
                cast_time: lookup(&lookups.cast_times, uint(r, 19)?),
                duration: lookup(&lookups.durations, uint(r, 20)?),
                pvp_duration: lookup(&lookups.durations, uint(r, 21)?),
                range: lookup(&lookups.ranges, uint(r, 22)?),
                speed: real(r, 23)?,
                launch_delay: real(r, 24)?,
                min_duration: real(r, 25)?,
                icon_file_data_id: uint(r, 26)?,
                active_icon_file_data_id: uint(r, 27)?,
                content_tuning_id: uint(r, 28)?,
            })
        },
        |spell, misc| spell.misc = Some(misc),
    )
}

fn load_effects(conn: &Connection, lookups: &Lookups, spells: &mut Spells) -> rusqlite::Result<()> {
    // Exact capacity: growth slack on ~600k effects of 164 B is ~100 MB.
    fill(
        conn,
        spells,
        "SELECT SpellID, COUNT(*) FROM spell_effect WHERE DifficultyID = 0 GROUP BY SpellID",
        |r| Ok(int(r, 1)? as usize),
        |spell, count| spell.effects.reserve_exact(count),
    )?;
    fill(
        conn,
        spells,
        "SELECT SpellID, EffectIndex, Effect, EffectAura, EffectBasePointsF,
                EffectBonusCoefficient, BonusCoefficientFromAP, EffectAmplitude,
                EffectAuraPeriod, EffectChainAmplitude, EffectChainTargets, EffectItemType,
                EffectMechanic, EffectPointsPerResource, EffectRealPointsPerLevel,
                EffectPos_facing, EffectTriggerSpell, PvpMultiplier, Coefficient, Variance,
                ResourceCoefficient, GroupSizeBasePointsCoefficient, ScalingClass,
                EffectAttributes, EffectMiscValue_0, EffectMiscValue_1, EffectRadiusIndex_0,
                EffectRadiusIndex_1, EffectSpellClassMask_0, EffectSpellClassMask_1,
                EffectSpellClassMask_2, EffectSpellClassMask_3, ImplicitTarget_0,
                ImplicitTarget_1
         FROM spell_effect WHERE DifficultyID = 0",
        |r| {
            Ok(SpellEffect {
                index: uint(r, 1)? as u8,
                immunity: None,
                effect: uint(r, 2)?,
                aura: uint(r, 3)?,
                base_points: real(r, 4)?,
                bonus_coefficient: real(r, 5)?,
                bonus_coefficient_from_ap: real(r, 6)?,
                amplitude: real(r, 7)?,
                aura_period_ms: uint(r, 8)?,
                chain_amplitude: real(r, 9)?,
                chain_targets: sint(r, 10)?,
                item_type: uint(r, 11)?,
                mechanic: uint(r, 12)?,
                points_per_resource: real(r, 13)?,
                real_points_per_level: real(r, 14)?,
                pos_facing: real(r, 15)?,
                trigger_spell: uint(r, 16)?,
                pvp_multiplier: real(r, 17)?,
                coefficient: real(r, 18)?,
                variance: real(r, 19)?,
                resource_coefficient: real(r, 20)?,
                group_size_base_points_coefficient: real(r, 21)?,
                scaling_class: sint(r, 22)?,
                attributes: uint(r, 23)?,
                misc_values: [sint(r, 24)?, sint(r, 25)?],
                radius: [
                    lookup(&lookups.radii, uint(r, 26)?),
                    lookup(&lookups.radii, uint(r, 27)?),
                ],
                spell_class_mask: uints(r, 28)?,
                implicit_targets: uints(r, 32)?,
            })
        },
        |spell, effect| spell.effects.push(effect),
    )
}

fn load_power_costs(conn: &Connection, spells: &mut Spells) -> rusqlite::Result<()> {
    fill(
        conn,
        spells,
        "SELECT SpellID, OrderIndex, PowerType, ManaCost, ManaCostPerLevel, ManaPerSecond,
                PowerCostPct, PowerCostMaxPct, OptionalCost, OptionalCostPct,
                PowerPctPerSecond, RequiredAuraSpellID, PowerDisplayID, AltPowerBarID
         FROM spell_power ORDER BY SpellID, OrderIndex, ID",
        |r| {
            Ok(SpellPowerCost {
                order_index: uint(r, 1)? as u8,
                power_type: sint(r, 2)?,
                mana_cost: sint(r, 3)?,
                mana_cost_per_level: sint(r, 4)?,
                mana_per_second: sint(r, 5)?,
                power_cost_pct: real(r, 6)?,
                power_cost_max_pct: real(r, 7)?,
                optional_cost: sint(r, 8)?,
                optional_cost_pct: real(r, 9)?,
                power_pct_per_second: real(r, 10)?,
                required_aura_spell_id: uint(r, 11)?,
                power_display_id: uint(r, 12)?,
                alt_power_bar_id: uint(r, 13)?,
            })
        },
        |spell, cost| spell.power_costs.push(cost),
    )
}

fn load_aura_options(
    conn: &Connection,
    lookups: &Lookups,
    spells: &mut Spells,
) -> rusqlite::Result<()> {
    fill(
        conn,
        spells,
        "SELECT SpellID, CumulativeAura, ProcCategoryRecovery, ProcChance, ProcCharges,
                SpellProcsPerMinuteID, ProcTypeMask_0, ProcTypeMask_1
         FROM spell_aura_options WHERE DifficultyID = 0",
        |r| {
            let procs_per_minute_id = uint(r, 5)?;
            Ok(SpellAuraOptions {
                cumulative_aura: uint(r, 1)?,
                proc_category_recovery_ms: sint(r, 2)?,
                proc_chance: uint(r, 3)?,
                proc_charges: sint(r, 4)?,
                procs_per_minute_id,
                procs_per_minute: lookup(&lookups.procs_per_minute, procs_per_minute_id),
                proc_type_mask: uints(r, 6)?,
            })
        },
        |spell, options| spell.aura_options = Some(Box::new(options)),
    )
}

fn load_categories(conn: &Connection, spells: &mut Spells) -> rusqlite::Result<()> {
    fill(
        conn,
        spells,
        "SELECT s.SpellID, s.Category, s.StartRecoveryCategory, s.ChargeCategory,
                s.DefenseType, s.DiminishType, s.DispelType, s.Mechanic, s.PreventionType,
                COALESCE(c.MaxCharges, 0), COALESCE(c.ChargeRecoveryTime, 0)
         FROM spell_categories s LEFT JOIN spell_category c ON c.ID = s.ChargeCategory
         WHERE s.DifficultyID = 0",
        |r| {
            Ok(SpellCategories {
                category: uint(r, 1)?,
                start_recovery_category: uint(r, 2)?,
                charge_category: uint(r, 3)?,
                defense_type: uint(r, 4)?,
                diminish_type: uint(r, 5)?,
                dispel_type: uint(r, 6)?,
                mechanic: uint(r, 7)?,
                prevention_type: uint(r, 8)?,
                max_charges: uint(r, 9)?,
                charge_recovery_ms: uint(r, 10)?,
            })
        },
        |spell, categories| spell.categories = Some(Box::new(categories)),
    )
}

fn load_cooldowns(conn: &Connection, spells: &mut Spells) -> rusqlite::Result<()> {
    fill(
        conn,
        spells,
        "SELECT SpellID, RecoveryTime, CategoryRecoveryTime, StartRecoveryTime, AuraSpellID
         FROM spell_cooldowns WHERE DifficultyID = 0",
        |r| {
            Ok(SpellCooldownInfo {
                recovery_ms: uint(r, 1)?,
                category_recovery_ms: uint(r, 2)?,
                start_recovery_ms: uint(r, 3)?,
                aura_spell_id: uint(r, 4)?,
            })
        },
        |spell, cooldowns| spell.cooldowns = Some(Box::new(cooldowns)),
    )
}

fn load_levels(conn: &Connection, spells: &mut Spells) -> rusqlite::Result<()> {
    fill(
        conn,
        spells,
        "SELECT SpellID, BaseLevel, MaxLevel, SpellLevel, MaxPassiveAuraLevel
         FROM spell_levels WHERE DifficultyID = 0",
        |r| {
            Ok(SpellLevels {
                base_level: uint(r, 1)?,
                max_level: uint(r, 2)?,
                spell_level: uint(r, 3)?,
                max_passive_aura_level: uint(r, 4)?,
            })
        },
        |spell, levels| spell.levels = Some(Box::new(levels)),
    )
}

fn load_scaling(conn: &Connection, spells: &mut Spells) -> rusqlite::Result<()> {
    fill(
        conn,
        spells,
        "SELECT SpellID, MinScalingLevel, MaxScalingLevel FROM spell_scaling",
        |r| {
            Ok(SpellScaling {
                min_scaling_level: uint(r, 1)?,
                max_scaling_level: uint(r, 2)?,
            })
        },
        |spell, scaling| spell.scaling = Some(scaling),
    )
}

fn load_class_options(conn: &Connection, spells: &mut Spells) -> rusqlite::Result<()> {
    fill(
        conn,
        spells,
        "SELECT SpellID, SpellClassSet, SpellClassMask_0, SpellClassMask_1, SpellClassMask_2,
                SpellClassMask_3, ModalNextSpell
         FROM spell_class_options",
        |r| {
            Ok(SpellClassOptions {
                spell_class_set: uint(r, 1)?,
                spell_class_mask: uints(r, 2)?,
                modal_next_spell: uint(r, 6)?,
            })
        },
        |spell, options| spell.class_options = Some(Box::new(options)),
    )
}

fn load_shapeshift(conn: &Connection, spells: &mut Spells) -> rusqlite::Result<()> {
    fill(
        conn,
        spells,
        "SELECT SpellID, StanceBarOrder, ShapeshiftExclude_0, ShapeshiftExclude_1,
                ShapeshiftMask_0, ShapeshiftMask_1
         FROM spell_shapeshift",
        |r| {
            Ok(SpellShapeshift {
                stance_bar_order: sint(r, 1)?,
                shapeshift_exclude: uints(r, 2)?,
                shapeshift_mask: uints(r, 4)?,
            })
        },
        |spell, shapeshift| spell.shapeshift = Some(Box::new(shapeshift)),
    )
}

fn load_interrupts(conn: &Connection, spells: &mut Spells) -> rusqlite::Result<()> {
    fill(
        conn,
        spells,
        "SELECT SpellID, InterruptFlags, AuraInterruptFlags_0, AuraInterruptFlags_1,
                ChannelInterruptFlags_0, ChannelInterruptFlags_1
         FROM spell_interrupts WHERE DifficultyID = 0",
        |r| {
            Ok(SpellInterrupts {
                interrupt_flags: uint(r, 1)?,
                aura_interrupt_flags: uints(r, 2)?,
                channel_interrupt_flags: uints(r, 4)?,
            })
        },
        |spell, interrupts| spell.interrupts = Some(Box::new(interrupts)),
    )
}

fn load_aura_restrictions(conn: &Connection, spells: &mut Spells) -> rusqlite::Result<()> {
    fill(
        conn,
        spells,
        "SELECT SpellID, CasterAuraState, TargetAuraState, ExcludeCasterAuraState,
                ExcludeTargetAuraState, CasterAuraSpell, TargetAuraSpell, ExcludeCasterAuraSpell,
                ExcludeTargetAuraSpell, CasterAuraType, TargetAuraType, ExcludeCasterAuraType,
                ExcludeTargetAuraType
         FROM spell_aura_restrictions WHERE DifficultyID = 0",
        |r| {
            Ok(SpellAuraRestrictions {
                caster_aura_state: uint(r, 1)?,
                target_aura_state: uint(r, 2)?,
                exclude_caster_aura_state: uint(r, 3)?,
                exclude_target_aura_state: uint(r, 4)?,
                caster_aura_spell: uint(r, 5)?,
                target_aura_spell: uint(r, 6)?,
                exclude_caster_aura_spell: uint(r, 7)?,
                exclude_target_aura_spell: uint(r, 8)?,
                caster_aura_type: uint(r, 9)?,
                target_aura_type: uint(r, 10)?,
                exclude_caster_aura_type: uint(r, 11)?,
                exclude_target_aura_type: uint(r, 12)?,
            })
        },
        |spell, restrictions| spell.aura_restrictions = Some(Box::new(restrictions)),
    )
}

fn load_target_restrictions(conn: &Connection, spells: &mut Spells) -> rusqlite::Result<()> {
    fill(
        conn,
        spells,
        "SELECT SpellID, ConeDegrees, MaxTargets, MaxTargetLevel, TargetCreatureType,
                Targets, Width
         FROM spell_target_restrictions WHERE DifficultyID = 0",
        |r| {
            Ok(SpellTargetRestrictions {
                cone_degrees: real(r, 1)?,
                max_targets: uint(r, 2)?,
                max_target_level: uint(r, 3)?,
                target_creature_type: uint(r, 4)?,
                targets: uint(r, 5)?,
                width: real(r, 6)?,
            })
        },
        |spell, restrictions| spell.target_restrictions = Some(Box::new(restrictions)),
    )
}

fn load_equipped_items(conn: &Connection, spells: &mut Spells) -> rusqlite::Result<()> {
    fill(
        conn,
        spells,
        "SELECT SpellID, EquippedItemClass, EquippedItemInvTypes, EquippedItemSubclass
         FROM spell_equipped_items",
        |r| {
            Ok(SpellEquippedItems {
                item_class: sint(r, 1)?,
                inventory_types: uint(r, 2)?,
                item_subclass: uint(r, 3)?,
            })
        },
        |spell, items| spell.equipped_items = Some(Box::new(items)),
    )
}

fn load_empower_stages(conn: &Connection, spells: &mut Spells) -> rusqlite::Result<()> {
    fill(
        conn,
        spells,
        "SELECT e.SpellID, s.Stage, s.DurationMs
         FROM spell_empower_stage s JOIN spell_empower e ON e.ID = s.SpellEmpowerID
         ORDER BY e.SpellID, s.Stage",
        |r| {
            Ok(EmpowerStage {
                stage: uint(r, 1)?,
                duration_ms: uint(r, 2)?,
            })
        },
        |spell, stage| spell.empower_stages.push(stage),
    )
}

fn load_labels(conn: &Connection, spells: &mut Spells) -> rusqlite::Result<()> {
    fill(
        conn,
        spells,
        "SELECT SpellID, LabelID FROM spell_label ORDER BY SpellID, LabelID",
        |r| uint(r, 1),
        |spell, label| spell.labels.push(label),
    )
}
