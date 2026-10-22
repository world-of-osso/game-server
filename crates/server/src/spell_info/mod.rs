//! Server spell data model loaded from the Retail DB2 tables in world.db
//! (build 12.1.0.69933, `scripts/db2_schema.sql`), DifficultyID 0 only.
//!
//! `SpellStore` indexes every `spell_name` row by spell ID. Values are raw
//! DB2 values: IDs, flags and masks are not translated, times are
//! milliseconds, and index columns (cast time, duration, range, radius,
//! procs-per-minute, empower) are resolved to their referenced rows.
//! `ClassSpells` holds per-class learnable and specialization spells and the
//! class trait tree.
//!
//! `crate::spell_cast` resolves casts from these records.

#[cfg(test)]
mod aura_restrictions_tests;
mod class_data;
mod immunity;
#[cfg(test)]
mod immunity_tests;
mod load;
pub mod positivity;
#[cfg(test)]
pub(crate) mod serverside_tests;
mod trait_data;

use std::path::Path;
use std::time::Instant;

use bevy::prelude::*;
use rusqlite::Connection;
use serde::Serialize;

pub(crate) use class_data::MAX_LEVEL;
#[cfg(test)]
pub(crate) use class_data::load_class_spells;
pub use class_data::{ClassInfo, ClassSpells, SpecInfo};
pub use immunity::SpellImmunityDefinition;
pub(crate) use trait_data::load_trait_tree_by_id;
pub use trait_data::{
    COND_FLAG_GATE, COND_FLAG_SUFFICIENT, TraitCond, TraitCondType, TraitCost, TraitEdgeType,
    TraitNode, TraitNodeEntry, TraitNodeType, TraitTree,
};

#[cfg(not(test))]
pub(crate) const WORLD_DB_PATH: &str = "data/world.db";
#[cfg(test)]
pub(crate) const WORLD_DB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/world.db");

/// One already matched charged spell modifier, retained by its launched cast.
#[derive(Debug, Clone)]
pub(crate) struct ChargedSpellModifier {
    pub op: i32,
    pub percent: bool,
    pub amount: f32,
}

/// One spell's DB2 data at DifficultyID 0. Sparse tables are boxed so the
/// ~400k spells without such rows stay small.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SpellInfo {
    pub id: u32,
    pub name: String,
    /// `spell.NameSubtext_lang` (rank text such as "Racial").
    pub subtext: String,
    /// At least one DifficultyID-0 SpellXSpellVisual has a nonzero SpellVisualID.
    pub has_spell_visual: bool,
    pub misc: Option<SpellMisc>,
    /// Every `spell_effect` row, sorted by `EffectIndex` (indexes may be sparse).
    pub effects: Vec<SpellEffect>,
    /// Runtime casting item's effective (squished) level, not a DB2 spell field.
    /// Kept on the per-cast clone through cast bars, missiles, and aura snapshots.
    #[serde(skip)]
    pub(crate) cast_item_level: Option<u16>,
    /// Source SPELLVALUE_AURA_STACK on this cast, retained through missile snapshots.
    #[serde(skip)]
    pub(crate) cast_aura_stacks: Option<u8>,
    /// Source CastSpellExtraArgs::OriginalCaster, not the actual casting unit.
    #[serde(skip)]
    pub(crate) cast_original_caster: Option<Entity>,
    /// Retail item actually used by this cast; delayed casts credit only on execution.
    #[serde(skip)]
    pub(crate) cast_retail_item_id: Option<u32>,
    /// Per-cast ground point; preserved in queued/cast snapshots, never DB2 metadata.
    #[serde(skip)]
    pub(crate) cast_destination: Option<[f32; 3]>,
    /// Owned bag GUID on this cast's queued/active snapshot.
    #[serde(skip)]
    pub(crate) cast_target_item_guid: Option<u64>,
    /// Source TRIGGERED_IGNORE_SPELL_AND_CATEGORY_CD (0x2), on per-cast clones only.
    /// CheckCast skips SpellHistory::IsReady; HandleCooldowns does no writes.
    #[serde(skip)]
    pub(crate) cast_ignore_cooldowns: bool,
    /// Successful-launch charged modifiers; None means not yet snapshotted.
    #[serde(skip)]
    pub(crate) cast_charge_mods: Option<Vec<ChargedSpellModifier>>,
    /// Script-owned cast-time proc instances that actually modified admission.
    /// Retained through hardcasts/missiles; triggered casts do not acquire markers.
    #[serde(skip)]
    pub(crate) cast_proc_mods: Vec<u32>,
    /// Every `spell_power` row, sorted by `OrderIndex`.
    pub power_costs: Vec<SpellPowerCost>,
    pub aura_options: Option<Box<SpellAuraOptions>>,
    pub aura_restrictions: Option<Box<SpellAuraRestrictions>>,
    pub categories: Option<Box<SpellCategories>>,
    pub cooldowns: Option<Box<SpellCooldownInfo>>,
    pub levels: Option<Box<SpellLevels>>,
    /// `spell_scaling`: the level range its SpellScaling effect amounts scale over.
    pub scaling: Option<SpellScaling>,
    pub class_options: Option<Box<SpellClassOptions>>,
    pub shapeshift: Option<Box<SpellShapeshift>>,
    pub interrupts: Option<Box<SpellInterrupts>>,
    pub target_restrictions: Option<Box<SpellTargetRestrictions>>,
    pub equipped_items: Option<Box<SpellEquippedItems>>,
    /// `spell_empower_stage` rows via `spell_empower`, sorted by stage.
    pub empower_stages: Vec<EmpowerStage>,
    /// `spell_label.LabelID` rows, ascending (matched by `*_MODIFIER_BY_SPELL_LABEL` auras).
    pub labels: Vec<u32>,
}

/// `spell_misc` with its index columns resolved.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SpellMisc {
    pub attributes: [u32; 17],
    /// Full school bitmask (1 Physical, 2 Holy, 4 Fire, 8 Nature, 16 Frost, 32 Shadow, 64 Arcane).
    pub school_mask: u32,
    pub cast_time: Option<SpellCastTime>,
    pub duration: Option<SpellDuration>,
    pub pvp_duration: Option<SpellDuration>,
    pub range: Option<SpellRange>,
    pub speed: f32,
    pub launch_delay: f32,
    pub min_duration: f32,
    pub icon_file_data_id: u32,
    pub active_icon_file_data_id: u32,
    pub content_tuning_id: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct SpellCastTime {
    pub base_ms: i32,
    pub minimum_ms: i32,
}

/// `spell_duration`; `-1` is infinite.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct SpellDuration {
    pub duration_ms: i32,
    pub max_duration_ms: i32,
    pub per_resource_ms: i32,
}

/// `spell_range`: `RangeMin/Max_0` are hostile, `_1` friendly (TrinityCore `GetMaxRange(positive)`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct SpellRange {
    pub min_hostile: f32,
    pub min_friendly: f32,
    pub max_hostile: f32,
    pub max_friendly: f32,
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct SpellRadius {
    pub radius: f32,
    pub radius_per_level: f32,
    pub radius_min: f32,
    pub radius_max: f32,
}

/// One `spell_effect` row.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SpellEffect {
    pub index: u8,
    /// Resolved TDB creature_immunities row for aura147; missing IDs stay None.
    pub immunity: Option<Box<SpellImmunityDefinition>>,
    /// SpellEffects ID (e.g. 2 school damage, 6 apply aura, 38 dispel, 68 interrupt).
    pub effect: u32,
    /// AuraType ID for aura-applying effects.
    pub aura: u32,
    pub base_points: f32,
    /// Spell power coefficient (`EffectBonusCoefficient`).
    pub bonus_coefficient: f32,
    pub bonus_coefficient_from_ap: f32,
    pub amplitude: f32,
    pub aura_period_ms: u32,
    pub chain_amplitude: f32,
    /// `EffectChainTargets`: units the effect jumps to, the first included (signed:
    /// some rows hold -10).
    pub chain_targets: i32,
    pub item_type: u32,
    pub mechanic: u32,
    pub points_per_resource: f32,
    pub real_points_per_level: f32,
    pub pos_facing: f32,
    pub trigger_spell: u32,
    pub pvp_multiplier: f32,
    pub coefficient: f32,
    pub variance: f32,
    pub resource_coefficient: f32,
    pub group_size_base_points_coefficient: f32,
    pub scaling_class: i32,
    pub attributes: u32,
    pub misc_values: [i32; 2],
    pub radius: [Option<SpellRadius>; 2],
    pub spell_class_mask: [u32; 4],
    pub implicit_targets: [u32; 2],
}

/// One `spell_power` row, raw. `power_type` is the DB2 Powers ID
/// (0 Mana, 1 Rage, 2 Focus, 3 Energy, 4 ComboPoints, 6 RunicPower,
/// 7 SoulShards, 8 LunarPower, 9 HolyPower, 11 Maelstrom, 12 Chi,
/// 13 Insanity, 16 ArcaneCharges, 17 Fury, 18 Pain, 19 Essence).
/// Rage and Runic Power amounts are stored in tenths: Slam 1464 has
/// `mana_cost` 200 for 20 Rage, Death Strike 49998 has 450 for 45 Runic
/// Power. They are kept unscaled here.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SpellPowerCost {
    pub order_index: u8,
    pub power_type: i32,
    pub mana_cost: i32,
    pub mana_cost_per_level: i32,
    pub mana_per_second: i32,
    /// Percent of base pool (Fireball 133: 1.0 = 1% of base mana).
    pub power_cost_pct: f32,
    pub power_cost_max_pct: f32,
    pub optional_cost: i32,
    pub optional_cost_pct: f32,
    pub power_pct_per_second: f32,
    /// The cost applies only while the caster has this aura (0 = always).
    pub required_aura_spell_id: u32,
    pub power_display_id: u32,
    pub alt_power_bar_id: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SpellAuraOptions {
    /// Max stacks (`CumulativeAura`).
    pub cumulative_aura: u32,
    pub proc_category_recovery_ms: i32,
    pub proc_chance: u32,
    pub proc_charges: i32,
    pub procs_per_minute_id: u32,
    pub procs_per_minute: Option<ProcsPerMinute>,
    pub proc_type_mask: [u32; 2],
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct ProcsPerMinute {
    pub base_proc_rate: f32,
    pub flags: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SpellCategories {
    pub category: u32,
    /// GCD category (`StartRecoveryCategory`).
    pub start_recovery_category: u32,
    pub charge_category: u32,
    pub defense_type: u32,
    pub diminish_type: u32,
    pub dispel_type: u32,
    pub mechanic: u32,
    pub prevention_type: u32,
    /// `spell_category.MaxCharges` of `charge_category` (0 = not charge-based).
    pub max_charges: u32,
    /// `spell_category.ChargeRecoveryTime` of `charge_category`, ms per charge.
    pub charge_recovery_ms: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SpellCooldownInfo {
    pub recovery_ms: u32,
    pub category_recovery_ms: u32,
    /// GCD length (`StartRecoveryTime`).
    pub start_recovery_ms: u32,
    pub aura_spell_id: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SpellLevels {
    pub base_level: u32,
    pub max_level: u32,
    pub spell_level: u32,
    pub max_passive_aura_level: u32,
}

/// `SpellScalingEntry`: 0 leaves that end of the level range open.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct SpellScaling {
    pub min_scaling_level: u32,
    pub max_scaling_level: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SpellClassOptions {
    pub spell_class_set: u32,
    pub spell_class_mask: [u32; 4],
    pub modal_next_spell: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SpellShapeshift {
    pub stance_bar_order: i32,
    pub shapeshift_exclude: [u32; 2],
    pub shapeshift_mask: [u32; 2],
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SpellInterrupts {
    pub interrupt_flags: u32,
    pub aura_interrupt_flags: [u32; 2],
    pub channel_interrupt_flags: [u32; 2],
}

/// Raw required/excluded state, spell and aura-type IDs from `spell_aura_restrictions`.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SpellAuraRestrictions {
    pub caster_aura_state: u32,
    pub target_aura_state: u32,
    pub exclude_caster_aura_state: u32,
    pub exclude_target_aura_state: u32,
    pub caster_aura_spell: u32,
    pub target_aura_spell: u32,
    pub exclude_caster_aura_spell: u32,
    pub exclude_target_aura_spell: u32,
    pub caster_aura_type: u32,
    pub target_aura_type: u32,
    pub exclude_caster_aura_type: u32,
    pub exclude_target_aura_type: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SpellTargetRestrictions {
    pub cone_degrees: f32,
    pub max_targets: u32,
    pub max_target_level: u32,
    pub target_creature_type: u32,
    pub targets: u32,
    pub width: f32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SpellEquippedItems {
    pub item_class: i32,
    pub inventory_types: u32,
    pub item_subclass: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct EmpowerStage {
    pub stage: u32,
    pub duration_ms: u32,
}

/// Every spell in world.db, sorted by spell ID (binary-searched; a `HashMap`
/// of 360-byte values costs ~40% more memory at this load factor).
#[derive(Resource, Debug, Default)]
#[cfg_attr(test, derive(Clone))]
pub struct SpellStore {
    spells: Vec<SpellInfo>,
}

impl SpellInfo {
    /// TC SpellInfo.cpp:1842–1857: cast eligibility, not aura persistence after death.
    pub(crate) fn allows_dead_target(&self) -> bool {
        const ALLOW_DEAD_TARGET: u32 = 0x0000_0001;
        // TARGET_FLAG_CORPSE_ENEMY | TARGET_FLAG_UNIT_DEAD | TARGET_FLAG_CORPSE_ALLY.
        const DEAD_TARGET_FLAGS: u32 = 0x0000_0200 | 0x0000_0400 | 0x0000_8000;
        // TARGET_CORPSE_SRC_AREA_ENEMY, TARGET_CORPSE_SRC_AREA_RAID, TARGET_CORPSE_TARGET_ALLY.
        const CORPSE_SELECTORS: [u32; 3] = [93, 119, 121];

        let attributes = self.misc.as_ref().map_or(0, |misc| misc.attributes[2]);
        if attributes & ALLOW_DEAD_TARGET != 0 {
            return true;
        }
        let target_flags = self
            .target_restrictions
            .as_ref()
            .map_or(0, |targets| targets.targets);
        if target_flags & DEAD_TARGET_FLAGS != 0 {
            return true;
        }
        self.effects.iter().any(|effect| {
            effect.effect != 0
                && effect
                    .implicit_targets
                    .iter()
                    .any(|target| CORPSE_SELECTORS.contains(target))
        })
    }

    /// Full school bitmask; 0 without a `spell_misc` row.
    pub fn school_mask(&self) -> u32 {
        self.misc.as_ref().map_or(0, |misc| misc.school_mask)
    }
}

impl SpellStore {
    #[cfg(test)]
    pub(crate) fn fixture(mut spells: Vec<SpellInfo>) -> Self {
        spells.sort_by_key(|spell| spell.id);
        Self { spells }
    }

    /// Production's server-side loader against an independent prepared copy.
    #[cfg(test)]
    pub(crate) fn append_serverside(&mut self, conn: &Connection) -> rusqlite::Result<usize> {
        load::append_serverside_spells(conn, self)
    }

    /// Resolve an explicit ID or a unique exact name; ambiguous ranks require an ID.
    pub(crate) fn resolve(&self, id: Option<u32>, name: &str) -> Option<&SpellInfo> {
        if let Some(id) = id {
            return self.get(id);
        }
        if name.is_empty() {
            return None;
        }
        let mut matches = self
            .spells
            .iter()
            .filter(|spell| spell.name.eq_ignore_ascii_case(name));
        let spell = matches.next()?;
        matches.next().is_none().then_some(spell)
    }

    pub fn get(&self, spell_id: u32) -> Option<&SpellInfo> {
        let index = self
            .spells
            .binary_search_by_key(&spell_id, |spell| spell.id)
            .ok()?;
        Some(&self.spells[index])
    }

    pub fn len(&self) -> usize {
        self.spells.len()
    }
}

/// Load every spell and the class spell lists from world.db.
pub fn load_spell_data(conn: &Connection) -> rusqlite::Result<(SpellStore, ClassSpells)> {
    Ok((
        load::load_spell_store(conn)?,
        class_data::load_class_spells(conn)?,
    ))
}

pub struct SpellInfoPlugin;

impl Plugin for SpellInfoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_spell_data_startup);
    }
}

fn load_spell_data_startup(mut commands: Commands) {
    let path = Path::new(WORLD_DB_PATH);
    if !path.exists() {
        warn!("world.db not found at {WORLD_DB_PATH}, spell store empty");
        commands.insert_resource(SpellStore::default());
        commands.insert_resource(ClassSpells::default());
        return;
    }
    let started = Instant::now();
    let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("Failed to open world.db for spell data");
    let (store, classes) = load_spell_data(&conn).expect("Failed to load spell data from world.db");
    info!(
        "Loaded {} spells and {} classes from world.db in {:?}",
        store.len(),
        classes.classes.len(),
        started.elapsed()
    );
    commands.insert_resource(store);
    commands.insert_resource(classes);
}

#[cfg(test)]
mod tests;
