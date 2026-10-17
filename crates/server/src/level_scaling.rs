//! Retail creature level scaling (docs/specs/level-scaling.md), as TrinityCore
//! master (a352b1fa) implements it. A tuned creature (`ContentTuningID` != 0)
//! has one health pool evaluated at its native level (the top of its
//! ContentTuning range); every unit fights it at its own level clamped to the
//! range (`shared::level_scaling::LevelScaling::level_for_target`):
//! - damage it deals is multiplied by ExpectedStat auto-attack DPS at the
//!   target's level over DPS at its native level (Unit::DealDamageMods,
//!   Unit.cpp:815; Creature::GetDamageMultiplierForTarget, Creature.cpp:3198-3206);
//! - damage it takes is divided by its max health at the attacker's level over
//!   its max health at its native level (Unit::DealDamage, Unit.cpp:830;
//!   Creature::GetHealthMultiplierForTarget, Creature.cpp:3181-3189).
//!
//! So a level 10 and a level 30 player take the same share of Hogger's one
//! health pool with a hit of the same share of their level's creature health.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use shared::components::UnitLevel;
use shared::level_scaling::{LevelScaling, MAX_LEVEL};

/// `EXPANSION_LEVEL_CURRENT` (-1) resolves to `CURRENT_EXPANSION`, Midnight
/// (CreatureData.h:470-473, SharedDefines.h:107).
const CURRENT_EXPANSION: i32 = 11;
/// ExpectedStat rows with this ExpansionID apply to every expansion without
/// its own row (DB2Manager::EvaluateExpectedStat, DB2Stores.cpp:2514-2516).
const DEFAULT_EXPANSION: i32 = -2;
/// Highest level a tuned creature is fought at (`MAX_LEVEL + 3`).
const TOP_LEVEL: u8 = MAX_LEVEL + 3;

/// The creature stats of one ExpectedStat row, or the multipliers of one
/// ExpectedStatMod row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureStats {
    pub health: f32,
    pub auto_attack_dps: f32,
    pub armor: f32,
    pub armor_constant: f32,
}

#[derive(Debug, Clone, Copy)]
enum Stat {
    Health,
    AutoAttackDps,
    Armor,
    ArmorConstant,
}

impl CreatureStats {
    fn get(&self, stat: Stat) -> f32 {
        match stat {
            Stat::Health => self.health,
            Stat::AutoAttackDps => self.auto_attack_dps,
            Stat::Armor => self.armor,
            Stat::ArmorConstant => self.armor_constant,
        }
    }
}

/// ExpectedStat, ExpectedStatMod and ContentTuningXExpected from world.db.
#[derive(Debug, Clone, Default)]
pub struct ExpectedStats {
    /// Keyed by `(Lvl, ExpansionID)`; a later ID replaces an earlier one
    /// (DB2Manager::LoadStores `_expectedStatsByLevel`, DB2Stores.cpp:1324-1325).
    pub by_level: HashMap<(u8, i32), CreatureStats>,
    /// ExpectedStatMod by ID.
    pub mods: HashMap<u32, CreatureStats>,
    /// ContentTuningID -> ExpectedStatMod IDs (DB2Stores.cpp:1293-1295).
    pub content_tuning_mods: HashMap<u32, Vec<u32>>,
}

impl ExpectedStats {
    /// DB2Manager::EvaluateExpectedStat (DB2Stores.cpp:2512-2618) for the
    /// creature stats; 1 when the level has no row.
    fn evaluate(
        &self,
        stat: Stat,
        level: u8,
        expansion: i32,
        content_tuning_id: u32,
        unit_class: u8,
    ) -> f32 {
        let Some(row) = self
            .by_level
            .get(&(level, expansion))
            .or_else(|| self.by_level.get(&(level, DEFAULT_EXPANSION)))
        else {
            return 1.0;
        };
        let tuning_mods = self.content_tuning_mods.get(&content_tuning_id);
        let class_mod = class_mod_id(unit_class).and_then(|id| self.mods.get(&id));
        tuning_mods
            .into_iter()
            .flatten()
            .filter_map(|id| self.mods.get(id))
            .chain(class_mod)
            .fold(row.get(stat), |value, mods| value * mods.get(stat))
    }

    /// Source base health and armor before template modifiers, including tuning/class mods.
    pub(crate) fn creature_health_armor(
        &self,
        level: u8,
        health_scaling_expansion: i32,
        content_tuning_id: u32,
        unit_class: u8,
    ) -> (f32, f32) {
        let expansion = resolve_health_expansion(health_scaling_expansion);
        (
            self.evaluate(
                Stat::Health,
                level,
                expansion,
                content_tuning_id,
                unit_class,
            ),
            self.evaluate(Stat::Armor, level, expansion, content_tuning_id, unit_class),
        )
    }

    /// Creature::GetBaseArmorForLevel (Creature.cpp:3208-3214) of an untuned
    /// template: ContentTuning 0, HealthScalingExpansion 0.
    pub fn untuned_armor(&self, level: u8, unit_class: u8, armor_modifier: f32) -> f32 {
        self.untuned_armor_for_expansion(level, 0, unit_class, armor_modifier)
    }

    /// Creature::GetBaseArmorForLevel with the template's authored expansion.
    pub fn untuned_armor_for_expansion(
        &self,
        level: u8,
        health_scaling_expansion: i32,
        unit_class: u8,
        armor_modifier: f32,
    ) -> f32 {
        let expansion = resolve_health_expansion(health_scaling_expansion);
        self.evaluate(Stat::Armor, level, expansion, 0, unit_class) * armor_modifier
    }

    /// The ArmorConstant a player attacker of every level 0..=MAX_LEVEL + 3
    /// meets: ExpansionID -2, no ContentTuning, CLASS_NONE
    /// (Unit::CalcArmorReducedDamage, Unit.cpp:1760).
    pub fn armor_constants(&self) -> ArmorConstants {
        ArmorConstants(
            (0..=TOP_LEVEL)
                .map(|level| self.evaluate(Stat::ArmorConstant, level, DEFAULT_EXPANSION, 0, 0))
                .collect(),
        )
    }
}

/// ExpectedStat spell damage, auto-attack DPS and PlayerMana by `(Lvl,
/// ExpansionID)`: spell damage and effect8 base-point percentage budgets
/// (`SpellEffectInfo::CalcBaseValue` → `EvaluateExpectedStat(stat, level,
/// expansion, 0, CLASS_NONE)`, which applies no ExpectedStatMod).
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct CreatureSpellStats {
    pub by_level: HashMap<(u8, i32), (f32, f32)>,
    /// ExpectedStat PlayerMana, used by effect8's native base-value branch.
    pub player_mana_by_level: HashMap<(u8, i32), f32>,
    /// `ContentTuning.ExpansionID` by ID: a spell's `ContentTuningID` picks the
    /// expansion (-2 without one).
    pub tuning_expansion: HashMap<u32, i32>,
}

impl CreatureSpellStats {
    /// Effect8's PlayerMana budget; source expansion lookup and missing-row value1.
    pub fn player_mana(&self, level: u8, content_tuning_id: u32) -> f32 {
        let expansion = self
            .tuning_expansion
            .get(&content_tuning_id)
            .copied()
            .unwrap_or(DEFAULT_EXPANSION);
        self.player_mana_by_level
            .get(&(level, expansion))
            .or_else(|| self.player_mana_by_level.get(&(level, DEFAULT_EXPANSION)))
            .copied()
            .unwrap_or(1.0)
    }

    /// `CreatureSpellDamage` (or `CreatureAutoAttackDps`) at `level` for a spell of
    /// ContentTuning `content_tuning_id`; 1 when the level has no row
    /// (DB2Manager::EvaluateExpectedStat).
    pub fn evaluate(&self, auto_attack_dps: bool, level: u8, content_tuning_id: u32) -> f32 {
        let expansion = self
            .tuning_expansion
            .get(&content_tuning_id)
            .copied()
            .unwrap_or(DEFAULT_EXPANSION);
        self.by_level
            .get(&(level, expansion))
            .or_else(|| self.by_level.get(&(level, DEFAULT_EXPANSION)))
            .map_or(
                1.0,
                |&(spell_damage, dps)| {
                    if auto_attack_dps { dps } else { spell_damage }
                },
            )
    }
}

/// ExpectedStat ArmorConstant by attacker level (index = level).
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct ArmorConstants(pub Vec<f32>);

impl ArmorConstants {
    pub fn at(&self, level: u8) -> Option<f32> {
        self.0.get(usize::from(level.min(TOP_LEVEL))).copied()
    }
}

/// A creature's armor at its own level (UNIT_MOD_ARMOR BASE_VALUE,
/// Creature::UpdateLevelDependantStats, Creature.cpp:1665-1666).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct CreatureArmor(pub f32);

/// ExpectedStatMod rows EvaluateExpectedStat applies per creature class
/// (DB2Stores.cpp:2521-2538): warrior 4, paladin 2, rogue 3, mage 1.
fn class_mod_id(unit_class: u8) -> Option<u32> {
    match unit_class {
        1 => Some(4),
        2 => Some(2),
        4 => Some(3),
        8 => Some(1),
        _ => None,
    }
}

fn resolve_health_expansion(expansion: i32) -> i32 {
    match expansion {
        -1 => CURRENT_EXPANSION,
        expansion => expansion,
    }
}

/// `creature_template_difficulty` inputs of a tuned template.
#[derive(Debug, Clone, Copy)]
pub struct TuningInputs {
    pub content_tuning_id: u32,
    pub health_scaling_expansion: i32,
    pub unit_class: u8,
    pub health_modifier: f32,
    pub damage_modifier: f32,
    pub armor_modifier: f32,
}

/// A tuned template's max health and base auto-attack DPS at every level
/// 0..=MAX_LEVEL + 3 (index = level).
#[derive(Debug, PartialEq)]
pub struct LevelStats {
    max_health: Vec<f32>,
    base_damage: Vec<f32>,
    armor: Vec<f32>,
    damage_modifier: f32,
}

impl LevelStats {
    pub fn build(stats: &ExpectedStats, inputs: &TuningInputs) -> Self {
        let expansion = resolve_health_expansion(inputs.health_scaling_expansion);
        let eval = |stat, level| {
            stats.evaluate(
                stat,
                level,
                expansion,
                inputs.content_tuning_id,
                inputs.unit_class,
            )
        };
        Self {
            // Creature::GetMaxHealthByLevel (Creature.cpp:3173-3179).
            max_health: (0..=TOP_LEVEL)
                .map(|level| (eval(Stat::Health, level) * inputs.health_modifier).ceil())
                .collect(),
            // Creature::GetBaseDamageForLevel (Creature.cpp:3191-3196).
            base_damage: (0..=TOP_LEVEL)
                .map(|level| eval(Stat::AutoAttackDps, level))
                .collect(),
            // Creature::GetBaseArmorForLevel (Creature.cpp:3208-3214).
            armor: (0..=TOP_LEVEL)
                .map(|level| eval(Stat::Armor, level) * inputs.armor_modifier)
                .collect(),
            damage_modifier: inputs.damage_modifier,
        }
    }

    pub fn max_health(&self, level: u8) -> f32 {
        self.max_health[usize::from(level.min(TOP_LEVEL))]
    }

    pub fn base_damage(&self, level: u8) -> f32 {
        self.base_damage[usize::from(level.min(TOP_LEVEL))]
    }

    pub fn armor(&self, level: u8) -> f32 {
        self.armor[usize::from(level.min(TOP_LEVEL))]
    }

    /// Main-hand swing damage range at `level` for a swing every
    /// `attack_secs`: weapon damage base DPS and 1.5 × base DPS
    /// (Creature::UpdateLevelDependantStats, Creature.cpp:1650-1656), times
    /// DamageModifier and the attack time (Creature::CalculateMinMaxDamage,
    /// StatSystem.cpp:1138-1147). Creature attack power is not modelled.
    pub fn swing_damage_range(&self, level: u8, attack_secs: f32) -> (f32, f32) {
        let min = self.base_damage(level) * self.damage_modifier * attack_secs;
        (min, min * 1.5)
    }
}

/// A tuned creature's per-level stats (shared by every spawn of its template).
#[derive(Component, Debug, Clone)]
pub struct ScaledStats(pub Arc<LevelStats>);

/// The level-scaling components a damage or level query reads.
pub(crate) type ScalingItem<'a> = (
    Option<&'a UnitLevel>,
    Option<&'a LevelScaling>,
    Option<&'a ScaledStats>,
);

/// A unit's level and, when it is a tuned creature, its scaling.
#[derive(Debug, Clone, Copy)]
pub(crate) struct UnitScaling<'a> {
    /// `UnitLevel` (GetEffectiveLevel); a tuned creature's native level.
    level: u8,
    tuned: Option<(&'a LevelScaling, &'a LevelStats)>,
}

impl<'a> UnitScaling<'a> {
    pub fn from_item((level, scaling, stats): ScalingItem<'a>) -> Self {
        Self {
            level: level.map_or(1, |level| level.0),
            tuned: scaling.zip(stats.map(|stats| &*stats.0)),
        }
    }

    pub fn of(world: &'a World, unit: Entity) -> Self {
        Self::from_item((world.get(unit), world.get(unit), world.get(unit)))
    }

    /// Unit::GetLevelForTarget / Creature::GetLevelForTarget.
    pub fn level_against(&self, target: &UnitScaling) -> u8 {
        self.tuned.map_or(self.level, |(scaling, _)| {
            scaling.level_for_target(target.level)
        })
    }

    /// A tuned creature's swing damage range at its own level.
    pub fn swing_damage_range(&self, attack_secs: f32) -> Option<(f32, f32)> {
        self.tuned
            .map(|(_, stats)| stats.swing_damage_range(self.level, attack_secs))
    }

    /// Creature::GetDamageMultiplierForTarget: base DPS at the target's level
    /// over base DPS at the creature's level.
    fn damage_multiplier(&self, target: &UnitScaling) -> f32 {
        self.tuned.map_or(1.0, |(scaling, stats)| {
            stats.base_damage(self.level_against(target))
                / stats.base_damage(scaling.native_level())
        })
    }

    /// `armor` (the creature's own) × Creature::GetArmorMultiplierForTarget
    /// (Creature.cpp:3216-3224): a tuned creature's own armor is its base armor
    /// at its native level, so the product is its base armor at the attacker's level.
    pub fn armor_against(&self, armor: f32, attacker: &UnitScaling) -> f32 {
        self.tuned.map_or(armor, |(_, stats)| {
            stats.armor(self.level_against(attacker))
        })
    }

    /// Creature::GetHealthMultiplierForTarget: max health at the attacker's
    /// level over the creature's own (CreateHealth, the native-level max).
    pub(crate) fn health_multiplier(&self, attacker: &UnitScaling) -> f32 {
        self.tuned.map_or(1.0, |(scaling, stats)| {
            stats.max_health(self.level_against(attacker))
                / stats.max_health(scaling.native_level())
        })
    }
}

/// Unit::DealDamageMods (Unit.cpp:814-815): damage `attacker` deals to
/// `victim`, times the attacker's damage multiplier for the victim; what the
/// combat log, threat and rage see.
pub(crate) fn damage_dealt(attacker: &UnitScaling, victim: &UnitScaling, damage: f32) -> f32 {
    (damage * attacker.damage_multiplier(victim)).trunc()
}

/// Unit::DealDamage (Unit.cpp:829-830): health `victim` loses from `damage`
/// dealt by `attacker`, divided by the victim's health multiplier for the attacker.
pub(crate) fn health_loss(attacker: &UnitScaling, victim: &UnitScaling, damage: f32) -> f32 {
    (damage / victim.health_multiplier(attacker)).trunc()
}

/// Combat-log damage beyond remaining health, in the logged damage's units.
/// Use the actual truncated health loss rather than recomputing its scaling.
pub(crate) fn damage_overkill(amount: f32, scaled_loss: f32, health_before: f32) -> f32 {
    if scaled_loss <= health_before {
        return 0.0;
    }
    let excess_fraction = (scaled_loss - health_before) / scaled_loss;
    amount * excess_fraction
}

#[cfg(test)]
#[path = "level_scaling_tests.rs"]
pub(crate) mod tests;
