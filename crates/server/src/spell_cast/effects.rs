//! Spell effect handlers, keyed by `SpellEffect.effect` (SpellEffects ID).
//!
//! Amounts: `BasePoints + BonusCoefficient × spell power +
//! BonusCoefficientFromAP × attack power`. Spell power, attack power and crit
//! chance come from the caster's `CharacterStats` (`stats.rs`, aura stat
//! modifiers included); units without it (NPCs) have 0.
//! Amounts take the caster's spell modifiers (Points/PointsIndexN on base
//! points, BonusCoefficient on coefficients); damage and healing then take
//! HealingAndDamage (or PeriodicHealingAndDamage) mods and the aura damage and
//! healing multipliers (`AuraStatMods`), then target immunity and absorbs.
//! CritChance mods add to the crit chance; a crit adds 100% of the amount
//! (retail 200%), scaled by CritDamageAndHealing mods.
//! School damage and heals vary by ± `Variance` / 2 (Spell::EffectSchoolDMG,
//! SpellEffects.cpp:565-567; EffectHeal, :1280-1282). The caster's
//! versatility raises all damage and healing (Unit::SpellDamagePctDone,
//! Unit.cpp:6944-6946). Physical damage takes the target's armor unless the
//! spell has a bleed mechanic (Unit::CalculateSpellDamageTaken, Unit.cpp:1212-1213;
//! SpellMgr.cpp:3036-3038).
//!
//! Weapon hits (58, 121, 31) roll the main hand's `[min, max]` and add
//! `AP / 6 × speed`; 121 NORMALIZED_WEAPON_DMG uses the normalized speed of the
//! weapon type (SimC `action_t::calculate_weapon_damage`). Main-hand damage is
//! `CharacterStats.main_hand` (`item_stats.rs`), unarmed 1-2 at 2.0 s.

use std::collections::HashSet;

use bevy::prelude::*;
use lightyear::prelude::ReplicationDirtyQueue;
use shared::casting::CastState;
use shared::components::{Health, Npc, Player, PowerType};
use shared::protocol::{CombatLogEvent, CombatLogKind, MissKind};
use shared::spell_data::CastFailReason;

use super::{SpellHistory, log_event};
use crate::auras::procs::{self, AuraEvent};
use crate::auras::{AuraStatMods, SpellModOp, spell_mods};
use crate::creature_scripts::{TrainingDummy, health_loss};
use crate::item_stats::WeaponDamage;
use crate::level_scaling::{ArmorConstants, CreatureArmor, UnitScaling};
use crate::spell_info::{SpellEffect, SpellInfo};
use crate::stats::CharacterStats;

/// TC SharedDefines.h592: ignore armor/crit/block damage-taken adjustments.
const ATTR4_IGNORE_DAMAGE_TAKEN_MODIFIERS: u32 = 0x100;

pub const EFFECT_INSTAKILL: u32 = 1;
pub const EFFECT_SCHOOL_DAMAGE: u32 = 2;
pub const EFFECT_DAMAGE_FROM_MAX_HEALTH_PCT: u32 = 165;
/// `SPELL_EFFECT_DUMMY`: no generic effect (`Spell::EffectDummy` runs scripts only).
pub const EFFECT_DUMMY: u32 = 3;
pub const EFFECT_POWER_DRAIN: u32 = 8;
pub const EFFECT_HEALTH_LEECH: u32 = 9;
pub const EFFECT_HEAL: u32 = 10;
pub const EFFECT_KILL_CREDIT_PERSONAL: u32 = 90;
pub const EFFECT_KILL_CREDIT: u32 = 134;
pub const EFFECT_PERSISTENT_AREA_AURA: u32 = 27;
pub const EFFECT_ENERGIZE: u32 = 30;
pub const EFFECT_WEAPON_PERCENT_DAMAGE: u32 = 31;
pub const EFFECT_TRIGGER_MISSILE: u32 = 32;
pub const EFFECT_DISPEL: u32 = 38;
pub const EFFECT_WEAPON_DAMAGE: u32 = 58;
pub const EFFECT_SEND_EVENT: u32 = 61;
pub const EFFECT_TRIGGER_SPELL: u32 = 64;
pub const EFFECT_INTERRUPT_CAST: u32 = 68;
pub const EFFECT_NORMALIZED_WEAPON_DMG: u32 = 121;
pub const EFFECT_HEAL_PCT: u32 = 136;
pub const EFFECT_FORCE_CAST: u32 = 140;
/// TC a352b1fa SpellEffects.cpp256/295 share EffectRemoveAura.
pub const EFFECT_REMOVE_AURA: u32 = 164;
pub const EFFECT_REMOVE_AURA_2: u32 = 203;
pub const EFFECT_TELEPORT_UNITS: u32 = 252;
pub const EFFECT_TELEPORT_UNITS_LEGACY: u32 = 15;
pub const EFFECT_LEAP: u32 = 29;
pub const EFFECT_MODIFY_CHARGES: u32 = 293;
pub const EFFECT_ATTACK_ME: u32 = 114;

/// Triggered spells may trigger further spells up to this depth.
pub const MAX_TRIGGER_DEPTH: u8 = 3;

/// One resolving spell.
pub struct EffectCtx<'a> {
    pub caster: Entity,
    pub spell: &'a SpellInfo,
    /// 0 for the requested spell, +1 per `TRIGGER_SPELL` hop.
    pub depth: u8,
    /// The resolving effect's destination (`targets::effect_dest`).
    pub dest: Option<super::targets::Dest>,
    /// The target's chain damage and healing multiplier (1 unchained).
    pub chain_multiplier: f32,
}

pub type EffectHandler = fn(&mut World, &EffectCtx, &SpellEffect, Entity);

/// The effect registry. Aura effects (`crate::auras::AURA_EFFECTS`, e.g. 6
/// APPLY_AURA) are resolved per target by `resolve_effects`, not here.
pub fn handler(effect_id: u32) -> Option<EffectHandler> {
    Some(match effect_id {
        EFFECT_INSTAKILL => instakill,
        EFFECT_SCHOOL_DAMAGE => school_damage,
        EFFECT_DAMAGE_FROM_MAX_HEALTH_PCT => damage_from_max_health_pct,
        33 | 95 => crate::gathering::effect_gather,
        87 => crate::pvp::isle_siege::effect_damage,
        EFFECT_POWER_DRAIN => power_drain,
        EFFECT_HEALTH_LEECH => health_leech,
        EFFECT_KILL_CREDIT_PERSONAL | EFFECT_KILL_CREDIT => quest_kill_credit,
        EFFECT_HEAL => heal,
        crate::death::resurrection::EFFECT_RESURRECT
        | crate::death::resurrection::EFFECT_RESURRECT_WITH_AURA => {
            crate::death::resurrection::send_offer
        }
        EFFECT_ENERGIZE => energize,
        41 => super::teleport::jump_to_unit,
        96 => super::charge::hit,
        98 | 144 => super::knockback::knock_back,
        124 => super::knockback::pull_towards,
        EFFECT_WEAPON_PERCENT_DAMAGE => weapon_percent_damage,
        EFFECT_DISPEL => dispel,
        EFFECT_WEAPON_DAMAGE | EFFECT_NORMALIZED_WEAPON_DMG => weapon_damage,
        // Spell::EffectTriggerMissileSpell (SpellEffects.cpp:725-778): the
        // missile's spell hits the unit target, like TRIGGER_SPELL.
        EFFECT_TRIGGER_SPELL | EFFECT_TRIGGER_MISSILE => trigger_spell,
        EFFECT_SEND_EVENT => send_game_event,
        EFFECT_INTERRUPT_CAST => interrupt_cast,
        EFFECT_HEAL_PCT => heal_pct,
        EFFECT_FORCE_CAST => force_cast,
        EFFECT_REMOVE_AURA | EFFECT_REMOVE_AURA_2 => remove_aura,
        EFFECT_TELEPORT_UNITS | EFFECT_TELEPORT_UNITS_LEGACY | EFFECT_LEAP => teleport_units,
        EFFECT_MODIFY_CHARGES => modify_charges,
        EFFECT_ATTACK_ME => attack_me,
        crate::profession::EFFECT_CREATE_ITEM => crate::profession::effect_create_item,
        crate::profession::EFFECT_LEARN_SPELL => crate::profession::effect_learn_spell,
        crate::profession::EFFECT_SKILL_STEP => crate::profession::effect_skill_step,
        crate::meeting_stone::summon::EFFECT_SUMMON_PLAYER => {
            crate::meeting_stone::summon::effect_summon_player
        }
        crate::hunter_pet::EFFECT_TAME_CREATURE => crate::hunter_pet::effect_tame_creature,
        crate::hunter_pet::EFFECT_DISMISS_PET => crate::hunter_pet::effect_dismiss_pet,
        _ => return None,
    })
}

/// TC EffectInstaKill hit-target branch. The existing Unit::Kill path handles
/// death/AI/credit once; instakill packet details and player CHEAT_GOD are unmodelled.
fn instakill(world: &mut World, ctx: &EffectCtx, _effect: &SpellEffect, target: Entity) {
    crate::combat::kill_unit(world, ctx.caster, target);
}

/// TC EffectKillCreditPersonal/EffectKillCredit: credit the hit player, not the caster
/// or the NPC hit. Only effect134 shares with that player's nearby group.
fn quest_kill_credit(world: &mut World, _ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    if world.get::<shared::components::Player>(target).is_none() {
        return;
    }
    crate::quest::event_credit(
        world,
        target,
        target,
        crate::quest_data::ObjectiveKind::Monster,
        effect.misc_values[0] as u32,
        effect.effect == EFFECT_KILL_CREDIT,
    );
}

/// TC EffectSendEvent → GameEvents::TriggerForPlayer: event is the authored MiscValue.
pub(super) fn send_game_event(
    world: &mut World,
    ctx: &EffectCtx,
    effect: &SpellEffect,
    _target: Entity,
) {
    if let Ok(event) = u32::try_from(effect.misc_values[0]) {
        crate::achievement::record_player_game_event(world, ctx.caster, event);
    }
}

fn attack_me(world: &mut World, ctx: &EffectCtx, _effect: &SpellEffect, target: Entity) {
    crate::creature_aggro::taunt(world, target, ctx.caster);
}

pub type CastEffectHandler = fn(&mut World, &EffectCtx, &SpellEffect);

/// Effects TrinityCore resolves once per cast (`SPELL_EFFECT_HANDLE_HIT`),
/// whatever units the effect's targets select.
pub fn cast_handler(effect_id: u32) -> Option<CastEffectHandler> {
    Some(match effect_id {
        super::disenchant::EFFECT_DISENCHANT => super::disenchant::effect_disenchant,
        254 => super::teleport::jump_to_destination,
        crate::game_objects::summoned::EFFECT_TRANSMITTED => {
            crate::game_objects::summoned::spawn_transmitted
        }
        crate::game_objects::summoned::EFFECT_SUMMON_OBJECT_WILD => {
            crate::game_objects::summoned::spawn_wild
        }
        crate::hunter_pet::EFFECT_SUMMON_PET => crate::hunter_pet::effect_summon_pet,
        crate::hunter_pet::EFFECT_RESURRECT_PET => crate::hunter_pet::effect_revive_pet,
        super::summon::EFFECT_SUMMON => super::summon::summon_creature,
        super::area_trigger::EFFECT_CREATE_AREATRIGGER => super::area_trigger::create_area_trigger,
        _ => return None,
    })
}

/// Effects that select targets and resolve: registry handlers, once-per-cast
/// handlers and aura effects.
pub fn is_handled(effect: &SpellEffect) -> bool {
    handler(effect.effect).is_some()
        || cast_handler(effect.effect).is_some()
        || crate::auras::is_aura_effect(effect)
}

/// `(spell id, effect id)` pairs already logged as unhandled.
#[derive(Resource, Debug, Default)]
pub struct UnhandledEffects(pub HashSet<(u32, u32)>);

pub fn log_unhandled(world: &mut World, spell_id: u32, effect_id: u32) {
    let mut logged = world.resource_mut::<UnhandledEffects>();
    if logged.0.insert((spell_id, effect_id)) {
        warn!("Spell {spell_id}: effect {effect_id} has no handler, skipped");
    }
}

/// `(spell power, attack power)` of `unit`; `healing` selects its healing power.
pub(crate) fn power_stats(world: &World, unit: Entity, healing: bool) -> (f32, f32) {
    if let Some(wolf) = world.get::<super::guardian::SpiritWolf>(unit) {
        let owner = world
            .get::<CharacterStats>(wolf.owner)
            .expect("Spirit Wolf owner requires CharacterStats for inherited power");
        let power = super::guardian::SpiritWolf::owner_power(owner.attack_power);
        return (power, power);
    }
    if let Some(summon) = world.get::<super::summon::SummonedCreature>(unit) {
        if world
            .get::<crate::hunter_pet::SummonedDemon>(unit)
            .is_some()
        {
            return demon_power_stats(world, summon.owner);
        }
    }
    if let Some(pet) = world.get::<crate::hunter_pet::HunterPet>(unit) {
        if world
            .get::<crate::hunter_pet::SummonedDemon>(unit)
            .is_some()
            || crate::hunter_pet::is_summoned_pet_owner(world, pet.owner)
        {
            // TC StatSystem.cpp1358: controlled SUMMON_PET uses the IsPet branch.
            // Water Elemental's separate40% branch is for non-pet guardians, not56.
            return demon_power_stats(world, pet.owner);
        }
        return (0.0, hunter_pet_attack_power(world, pet.owner));
    }
    world
        .get::<CharacterStats>(unit)
        .map_or((0.0, 0.0), |stats| {
            let power = if healing {
                stats.healing_power
            } else {
                stats.spell_power
            };
            (power, stats.attack_power)
        })
}

/// Guardian::UpdateAttackPowerAndDamage (TC StatSystem.cpp:1358-1367): demons
/// inherit 15% spell bonus and 57% AP from the owner's fire/shadow spell power.
/// CharacterStats currently exposes one spell-power value, not school-specific values.
fn demon_power_stats(world: &World, owner: Entity) -> (f32, f32) {
    let power = world
        .get::<CharacterStats>(owner)
        .map_or(0.0, |stats| stats.spell_power.max(0.0));
    (
        power * DEMON_OWNER_SPELL_POWER,
        power * DEMON_OWNER_ATTACK_POWER,
    )
}

const DEMON_OWNER_SPELL_POWER: f32 = 0.15;
const DEMON_OWNER_ATTACK_POWER: f32 = 0.57;

/// Heals and absorbs take healing power: Spell::EffectHeal (SpellEffects.cpp:1305),
/// periodic heals (SpellAuraEffects.cpp:5891) and absorbs (SpellAuraEffects.cpp:807,
/// `SpellAbsorbBonusDone` = `SpellBaseHealingBonusDone`, Unit.cpp:7641-7644).
fn takes_healing_power(effect: &SpellEffect) -> bool {
    match effect.effect {
        EFFECT_HEAL => true,
        _ if crate::auras::is_aura_effect(effect) => matches!(
            effect.aura,
            crate::auras::handlers::PERIODIC_HEAL | crate::auras::handlers::SCHOOL_ABSORB
        ),
        _ => false,
    }
}

/// `Guardian::UpdateAttackPowerAndDamage` for a hunter pet (StatSystem.cpp:1336-1345): its
/// owner's ranged attack power × 0.22. The pet's own `2 × Strength - 20` needs `pet_levelstats`,
/// which is not imported.
const HUNTER_PET_OWNER_AP: f32 = 0.22;

fn hunter_pet_attack_power(world: &World, owner: Entity) -> f32 {
    world
        .get::<CharacterStats>(owner)
        .map_or(0.0, |stats| stats.ranged_attack_power * HUNTER_PET_OWNER_AP)
}

const SPELL_ATTR0_SCALES_WITH_CREATURE_LEVEL: u32 = 0x0008_0000;
const SPELL_ATTR10_USE_SPELL_BASE_LEVEL_FOR_SCALING: u32 = 0x0000_1000;
const SPELL_ATTR11_SCALES_WITH_ITEM_LEVEL: u32 = 0x0000_0004;
const SPELL_ATTR12_USE_FLOAT_VALUES_FOR_SCALING_AMOUNTS: u32 = 0x0020_0000;

/// `SpellEffectInfo::GetScalingExpectedStat` for the types a creature spell scales:
/// school damage and damage auras scale with `CreatureSpellDamage` (true = the
/// `CreatureAutoAttackDps` of `SPELL_ATTR0_SCALES_WITH_CREATURE_LEVEL`).
fn scales_with_spell_damage(effect: &SpellEffect) -> bool {
    match effect.effect {
        EFFECT_SCHOOL_DAMAGE | EFFECT_WEAPON_DAMAGE => true,
        _ if crate::auras::is_aura_effect(effect) => {
            effect.aura == crate::auras::PERIODIC_DAMAGE_AURA
        }
        _ => false,
    }
}

/// `SpellEffectInfo::CalcBaseValue` (SpellInfo.cpp:662-760). An effect with a scaling
/// `Coefficient` is its scaling class column of gtSpellScaling at the caster's level
/// (clamped to the spell's SpellScaling range, or its base level with
/// `SPELL_ATTR10_USE_SPELL_BASE_LEVEL_FOR_SCALING`) × the coefficient, at least 1 when
/// positive. Otherwise a creature's damage effect is `BasePoints`% of ExpectedStat
/// CreatureSpellDamage at its level, and everything else `BasePoints` (player class
/// spells carry their damage in coefficients). Both round unless
/// `SPELL_ATTR12_USE_FLOAT_VALUES_FOR_SCALING_AMOUNTS`. Item-level spells instead
/// use RandPropPoints at the casting item's effective level. Not modelled:
/// the target's level of `SPELL_ATTR8_USE_TARGETS_LEVEL_FOR_SPELL_SCALING` and the casting item's
/// level multipliers of classes -6 and -7.
fn calc_base_value(world: &World, caster: Entity, spell: &SpellInfo, effect: &SpellEffect) -> f32 {
    let attribute = |index: usize| spell.misc.as_ref().map_or(0, |misc| misc.attributes[index]);
    let level = world
        .get::<shared::components::UnitLevel>(caster)
        .map_or(1, |level| level.0);
    let round = |value: f32| {
        if attribute(12) & SPELL_ATTR12_USE_FLOAT_VALUES_FOR_SCALING_AMOUNTS != 0 {
            value
        } else {
            value.round()
        }
    };
    if effect.coefficient != 0.0 {
        if attribute(11) & SPELL_ATTR11_SCALES_WITH_ITEM_LEVEL != 0 {
            return round(item_spell_scaling_value(world, spell, effect));
        }
        let base_level = spell.levels.as_ref().map_or(0, |levels| levels.base_level);
        let level = if base_level > 0
            && attribute(10) & SPELL_ATTR10_USE_SPELL_BASE_LEVEL_FOR_SCALING != 0
        {
            base_level
        } else {
            u32::from(level)
        };
        return round(spell_scaling_value(spell, effect, level));
    }
    let creature = world.get::<Npc>(caster).is_some();
    let stats = world.get_resource::<crate::level_scaling::CreatureSpellStats>();
    let (true, Some(stats)) = (creature && scales_with_spell_damage(effect), stats) else {
        return effect.base_points;
    };
    let auto_attack = attribute(0) & SPELL_ATTR0_SCALES_WITH_CREATURE_LEVEL != 0;
    let tuning = spell.misc.as_ref().map_or(0, |misc| misc.content_tuning_id);
    round(stats.evaluate(auto_attack, level, tuning) * effect.base_points / 100.0)
}

/// Item-level branch of TC SpellEffectInfo::CalcBaseValue: classes -8/-9 use
/// DamageReplaceStatF/DamageSecondaryF; other nonzero classes use a rare chest budget.
fn item_spell_scaling_value(world: &World, spell: &SpellInfo, effect: &SpellEffect) -> f32 {
    if effect.scaling_class == 0 {
        return 0.0;
    }
    // TC's effectiveItemLevel is 1 when the spell has no casting item.
    let level = spell.cast_item_level.unwrap_or(1);
    let tables = world.resource::<crate::item_stats::ItemScalingTables>();
    let row = tables
        .spell_budgets
        .get(&level)
        .expect("casting item level in RandPropPoints");
    let column = match effect.scaling_class {
        -8 => 0,
        -9 => 1,
        _ => 2,
    };
    let value = row[column] * effect.coefficient;
    if value > 0.0 && value < 1.0 {
        1.0
    } else {
        value
    }
}

/// The SpellScaling amount of `effect` at `level`, before rounding: the
/// `GetSpellScalingColumnForClass` column of its scaling class (0 for a class column,
/// all 0 in this build, or class 0) × `Coefficient`, raised to 1 when in (0, 1).
pub(crate) fn spell_scaling_value(spell: &SpellInfo, effect: &SpellEffect, level: u32) -> f32 {
    use crate::spell_scaling_data::SPELL_SCALING;
    let mut level = level;
    if let Some(scaling) = spell.scaling {
        if scaling.min_scaling_level > level {
            level = scaling.min_scaling_level;
        }
        if scaling.max_scaling_level != 0 && scaling.max_scaling_level < level {
            level = scaling.max_scaling_level;
        }
    }
    let column = match effect.scaling_class {
        -1 | -7 => Some(0),
        -2 => Some(1),
        -3 => Some(2),
        -4 => Some(3),
        -5 => Some(4),
        -6 => Some(5),
        -8 => Some(6),
        -9 => Some(7),
        -10 => Some(8),
        _ => None,
    };
    let row = (level as usize)
        .checked_sub(1)
        .and_then(|index| SPELL_SCALING.get(index));
    let value = match (row, column) {
        (Some(row), Some(column)) => row[column] * effect.coefficient,
        _ => 0.0,
    };
    if value > 0.0 && value < 1.0 {
        1.0
    } else {
        value
    }
}

///TC a352b1fa SpellInfo.cpp869-956: whether the effect uses an expected-stat
///budget, which excludes the separate RealPointsPerLevel branch.
fn uses_expected_stat(effect: &SpellEffect) -> bool {
    match effect.effect {
        2 | 7 | 8 | 9 | 10 | 17 | 58 | 75 => true,
        30 | 62 => effect.misc_values[0] == 0,
        6 | 27 | 35 | 65 | 119 | 128 | 129 | 143 | 174 | 202 | 271 => match effect.aura {
            24 | 35 | 73 | 85 | 162 | 418 => effect.misc_values[0] == 0,
            3 | 8 | 13 | 14 | 15 | 22 | 29 | 34 | 43 | 53 | 59 | 62 | 64 | 69 | 83 | 84 | 97
            | 99 | 102 | 115 | 123 | 124 | 131 | 135 | 161 | 180 | 189 | 230 | 250 | 301 | 465 => {
                true
            }
            _ => false,
        },
        _ => false,
    }
}

///Native CalcValue: clamp caster level to MaxLevel, subtract max(BaseLevel,
///SpellLevel), then add the authored per-level points before point modifiers.
fn level_scaled_points(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
) -> f32 {
    if effect.coefficient != 0.0
        || uses_expected_stat(effect)
        || effect.real_points_per_level == 0.0
    {
        return 0.0;
    }
    let level = if world.get::<Player>(caster).is_some() {
        world
            .get::<shared::components::Level>(caster)
            .map(|level| u32::from(level.0))
    } else {
        world
            .get::<shared::components::UnitLevel>(caster)
            .map(|level| u32::from(level.0))
    };
    let Some(level) = level else {
        return 0.0;
    }; // Non-unit casters have no level contribution.
    let maximum = spell.levels.as_ref().map_or(0, |levels| levels.max_level);
    let capped = if maximum > 0 {
        level.min(maximum)
    } else {
        level
    };
    let baseline = spell
        .levels
        .as_ref()
        .map_or(0, |levels| levels.base_level.max(levels.spell_level));
    capped.saturating_sub(baseline) as f32 * effect.real_points_per_level
}

/// Base points (`calc_base_value`) with the caster's Points and PointsIndexN mods.
pub(crate) fn modded_base_points(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
) -> f32 {
    let base_points = calc_base_value(world, caster, spell, effect);
    modify_base_points(world, caster, spell, effect, base_points)
}

/// TC CalcValue applies variance before level contributions and point modifiers.
pub(super) fn roll_base_points(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
) -> f32 {
    let base_points = calc_base_value(world, caster, spell, effect);
    let delta = (effect.variance * 0.5).abs();
    let rolled_points = if delta == 0.0 {
        base_points
    } else {
        let roll = rand::Rng::gen_range(&mut rand::thread_rng(), -delta..=delta);
        base_points + base_points * roll
    };
    modify_base_points(world, caster, spell, effect, rolled_points)
}

fn modify_base_points(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
    base_points: f32,
) -> f32 {
    let base_points = base_points
        + level_scaled_points(world, caster, spell, effect)
        + crate::auras::mastery::effect_bonus(world, caster, spell, effect);
    let base = spell_mods::modify(world, caster, spell, SpellModOp::Points, base_points);
    match SpellModOp::points_index(effect.index) {
        Some(op) => spell_mods::modify(world, caster, spell, op, base),
        None => base,
    }
}

/// Effect amount before crit: `modded_base_points` plus SP/AP coefficients
/// with BonusCoefficient mods.
pub(crate) fn scaled_amount(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
) -> f32 {
    modded_base_points(world, caster, spell, effect)
        + effect_power_bonus(world, caster, spell, effect)
}

/// Native SP/AP contribution, separate from stored aura base points. Damage
/// shields resolve this at retaliation using the original aura caster.
pub(crate) fn effect_power_bonus(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
) -> f32 {
    let (spell_power, melee_attack_power) = power_stats(world, caster, takes_healing_power(effect));
    // Unit.cpp6880–6890 chooses RAP for non-melee ranged weapon spells only.
    let attack_power = if is_ranged_weapon_spell(spell)
        && spell
            .categories
            .as_ref()
            .is_none_or(|categories| categories.defense_type != DEFENSE_MELEE)
    {
        world
            .get::<CharacterStats>(caster)
            .map_or(0.0, |stats| stats.ranged_attack_power)
    } else {
        melee_attack_power
    };
    let modify = |op, value| spell_mods::modify(world, caster, spell, op, value);
    let coefficient = modify(SpellModOp::BonusCoefficient, effect.bonus_coefficient);
    let ap_coefficient = modify(
        SpellModOp::BonusCoefficient,
        effect.bonus_coefficient_from_ap,
    );
    coefficient * spell_power + ap_coefficient * attack_power
}

/// One main-hand weapon hit. A creature (no `CharacterStats`) rolls its own weapon
/// damage (`combat::creature_weapon_damage`, Unit::CalculateDamage).
fn weapon_hit(world: &World, caster: Entity, normalized: bool) -> f32 {
    if world.get::<CharacterStats>(caster).is_none() && world.get::<Npc>(caster).is_some() {
        return crate::combat::creature_weapon_damage(&UnitScaling::of(world, caster));
    }
    let (_, attack_power) = power_stats(world, caster, false);
    let weapon = world
        .get::<CharacterStats>(caster)
        .map_or(WeaponDamage::UNARMED, |stats| stats.main_hand);
    let speed = if normalized {
        weapon.kind.normalized_speed()
    } else {
        weapon.speed
    };
    let roll = weapon.min + rand::random::<f32>() * (weapon.max - weapon.min);
    roll + attack_power / crate::melee::ATTACK_POWER_PER_DPS * speed
}

/// `amount` × (1 + a uniform roll in ±`variance` / 2) (SpellEffectInfo::CalcValue,
/// SpellInfo.cpp:560-567).
fn with_variance(amount: f32, variance: f32) -> f32 {
    let delta = (variance * 0.5).abs();
    if delta == 0.0 {
        return amount;
    }
    amount * (1.0 + rand::Rng::gen_range(&mut rand::thread_rng(), -delta..=delta))
}

/// 1 + the caster's versatility (players only).
pub(crate) fn versatility_multiplier(world: &World, caster: Entity) -> f32 {
    world
        .get::<CharacterStats>(caster)
        .map_or(1.0, |stats| 1.0 + stats.versatility_pct / 100.0)
}

const MECHANIC_BLEED: u32 = 15;

/// Unit::IsDamageReducedByArmor (Unit.cpp:1680-1687): physical damage of a
/// spell without a bleed mechanic.
fn reduced_by_armor(spell: &SpellInfo) -> bool {
    let bleed = spell
        .categories
        .as_ref()
        .is_some_and(|categories| categories.mechanic == MECHANIC_BLEED)
        || spell
            .effects
            .iter()
            .any(|effect| effect.mechanic == MECHANIC_BLEED);
    spell.school_mask() & 1 != 0 && !bleed
}

/// The target's armor against the caster: a creature's own armor at the
/// caster's level, a player's gear armor.
fn armor_reduced(world: &World, caster: Entity, target: Entity, amount: f32) -> f32 {
    let (caster_scaling, target_scaling) = (
        UnitScaling::of(world, caster),
        UnitScaling::of(world, target),
    );
    let armor = match (
        world.get::<CreatureArmor>(target),
        world.get::<CharacterStats>(target),
    ) {
        (Some(armor), _) => {
            let multiplier = crate::auras::base_armor::pet_multiplier(
                world.get::<crate::stats::StatModifiers>(target),
                world.get::<crate::hunter_pet::HunterPet>(target),
                world.get::<super::summon::SummonedCreature>(target),
            );
            let modifiers = world.get::<crate::stats::StatModifiers>(target);
            target_scaling.armor_against(armor.0 * multiplier, &caster_scaling)
                * crate::auras::base_armor::total_multiplier(modifiers)
        }
        (None, Some(stats)) => stats.armor,
        (None, None) => 0.0,
    };
    crate::combat::armor_mitigated(
        amount,
        armor,
        &caster_scaling,
        &target_scaling,
        world.get_resource::<ArmorConstants>(),
    )
}

/// TC SpellInfo::IsRangedWeaponSpell; ItemTemplate.h weapon subclass mask.
fn is_ranged_weapon_spell(spell: &SpellInfo) -> bool {
    const HUNTER_FAMILY: u32 = 9;
    const HUNTER_NON_WEAPON_FLAG: u32 = 0x1000_0000;
    const RANGED_WEAPON_SUBCLASSES: u32 = (1 << 2) | (1 << 3) | (1 << 18);
    const USES_RANGED_SLOT: u32 = 0x2;
    let hunter_weapon = spell.class_options.as_ref().is_some_and(|options| {
        options.spell_class_set == HUNTER_FAMILY
            && options.spell_class_mask[1] & HUNTER_NON_WEAPON_FLAG == 0
    });
    let ranged_item = spell
        .equipped_items
        .as_ref()
        .is_some_and(|items| items.item_subclass & RANGED_WEAPON_SUBCLASSES != 0);
    hunter_weapon || ranged_item || super::attributes(spell, 0) & USES_RANGED_SLOT != 0
}

const DEFENSE_MELEE: u32 = 2;
const DEFENSE_RANGED: u32 = 3;

/// Unit::SpellCritChanceTaken plus SpellInfo::GetAttackType (a352b1fa).
fn takes_melee_crit_modifier(spell: &SpellInfo) -> bool {
    match spell
        .categories
        .as_ref()
        .map(|categories| categories.defense_type)
    {
        Some(DEFENSE_MELEE) => true,
        Some(DEFENSE_RANGED) => !is_ranged_weapon_spell(spell),
        _ => false,
    }
}

/// SharedDefines.h:539, SPELL_ATTR2_CANT_CRIT.
const SPELL_ATTR2_CANT_CRIT: u32 = 0x2000_0000;

/// CritChance mods and script values precede the victim's melee crit modifier.
fn crit_chance(world: &World, caster: Entity, target: Entity, spell: &SpellInfo) -> f32 {
    if super::attributes(spell, 2) & SPELL_ATTR2_CANT_CRIT != 0 {
        return 0.0;
    }
    let chance =
        super::scripts::crit_chance(world, caster, target, spell.id).unwrap_or_else(|| {
            let base = world
                .get::<CharacterStats>(caster)
                .map_or(0.0, |stats| stats.crit_pct);
            spell_mods::modify(world, caster, spell, SpellModOp::CritChance, base)
        });
    let modifier = if takes_melee_crit_modifier(spell) {
        world
            .get::<crate::auras::ActiveAuras>(target)
            .map_or(0.0, |auras| {
                auras.modifier(crate::auras::handlers::MOD_ATTACKER_MELEE_CRIT_CHANCE)
            })
    } else {
        0.0
    };
    (chance + modifier).max(0.0)
}

fn with_crit(
    world: &World,
    caster: Entity,
    target: Entity,
    spell: &SpellInfo,
    amount: f32,
) -> (f32, bool) {
    let crit = rand::random::<f32>() * 100.0 < crit_chance(world, caster, target, spell);
    if !crit {
        return (amount, false);
    }
    let bonus = spell_mods::modify(
        world,
        caster,
        spell,
        SpellModOp::CritDamageAndHealing,
        amount,
    );
    (amount + bonus, true)
}

fn uses_weapon_crit_bonus_order(spell: &SpellInfo) -> bool {
    spell
        .categories
        .as_ref()
        .is_some_and(|categories| matches!(categories.defense_type, DEFENSE_MELEE | DEFENSE_RANGED))
}

/// TC Unit.cpp:1222-1237/7297-7324:163 multiplies the whole critical hit;
/// CritDamageAndHealing mods apply before163 for weapons, after163 for magic.
fn with_damage_crit(
    world: &World,
    caster: Entity,
    target: Entity,
    spell: &SpellInfo,
    amount: f32,
) -> (f32, bool) {
    let crit = rand::random::<f32>() * 100.0 < crit_chance(world, caster, target, spell);
    if !crit {
        return (amount, false);
    }
    let multiplier = world
        .get::<AuraStatMods>(caster)
        .map_or(1.0, |mods| mods.critical_damage(spell.school_mask()));
    let weapon_order = uses_weapon_crit_bonus_order(spell);
    let base_bonus = if weapon_order {
        amount
    } else {
        amount * 2.0 * multiplier - amount
    };
    let bonus = spell_mods::modify(
        world,
        caster,
        spell,
        SpellModOp::CritDamageAndHealing,
        base_bonus,
    );
    let total = amount + bonus;
    let total = if weapon_order {
        total * multiplier
    } else {
        total
    };
    (total, true)
}

fn school_damage(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let amount = with_variance(
        scaled_amount(world, ctx.caster, ctx.spell, effect),
        effect.variance,
    ) * ctx.chain_multiplier;
    let (amount, crit) = with_damage_crit(world, ctx.caster, target, ctx.spell, amount);
    deal_effect_damage(world, ctx, effect, target, amount, crit, false);
}

/// TC SpellEffects5165–5173/Unit.h809: integer percent of the target's maximum.
/// This effect bypasses SpellDamageBonusDone/Taken, unlike EffectSchoolDMG.
fn damage_from_max_health_pct(
    world: &mut World,
    ctx: &EffectCtx,
    effect: &SpellEffect,
    target: Entity,
) {
    let Some(health) = world.get::<Health>(target) else {
        return;
    };
    let maximum = health.max;
    let percent = modded_base_points(world, ctx.caster, ctx.spell, effect);
    let amount = (maximum * percent / 100.0).trunc() * ctx.chain_multiplier;
    let (amount, crit) = with_damage_crit(world, ctx.caster, target, ctx.spell, amount);
    deal_damage_and_report(
        world,
        ctx,
        target,
        DamageInput {
            amount,
            crit,
            periodic: false,
            resolved_source: true,
            effect: Some(effect),
            bonus_caster: ctx.caster,
        },
    );
}

/// 58 WEAPON_DAMAGE and 121 NORMALIZED_WEAPON_DMG: weapon hit + BasePoints.
fn weapon_damage(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let normalized = effect.effect == EFFECT_NORMALIZED_WEAPON_DMG;
    let base =
        (weapon_hit(world, ctx.caster, normalized) + effect.base_points) * ctx.chain_multiplier;
    let (amount, crit) = with_damage_crit(world, ctx.caster, target, ctx.spell, base);
    deal_effect_damage(world, ctx, effect, target, amount, crit, false);
}

/// 31 WEAPON_PERCENT_DAMAGE: BasePoints% of a weapon hit.
fn weapon_percent_damage(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let base =
        weapon_hit(world, ctx.caster, false) * effect.base_points / 100.0 * ctx.chain_multiplier;
    let (amount, crit) = with_damage_crit(world, ctx.caster, target, ctx.spell, base);
    deal_effect_damage(world, ctx, effect, target, amount, crit, false);
}

fn log_amount(ctx: &EffectCtx, target: Entity, kind: CombatLogKind) -> CombatLogEvent {
    CombatLogEvent {
        source: Some(ctx.caster.to_bits()),
        target: Some(target.to_bits()),
        spell_id: Some(ctx.spell.id),
        school_mask: ctx.spell.school_mask(),
        amount: 0,
        overflow: 0,
        absorbed: 0,
        resisted: 0,
        blocked: 0,
        crit: false,
        glancing: false,
        periodic: false,
        extra_spell_id: None,
        timestamp_unix_ms: 0,
        kind,
    }
}

pub(crate) fn log_mechanic_immune(world: &mut World, ctx: &EffectCtx, target: Entity) {
    log_event(
        world,
        log_amount(ctx, target, CombatLogKind::Miss(MissKind::Immune)),
    );
}

fn mark_dirty(world: &World, entity: Entity) {
    if let Some(dirty) = world.get_resource::<ReplicationDirtyQueue>() {
        dirty.mark(entity);
    }
}

/// Spell mods (HealingAndDamage or PeriodicHealingAndDamage) on the base amount.
fn modded_amount(world: &World, ctx: &EffectCtx, amount: f32, periodic: bool) -> f32 {
    let op = if periodic {
        SpellModOp::PeriodicHealingAndDamage
    } else {
        SpellModOp::HealingAndDamage
    };
    spell_mods::modify(world, ctx.caster, ctx.spell, op, amount)
}

fn stat_mods(world: &World, unit: Entity) -> Option<&AuraStatMods> {
    world.get::<AuraStatMods>(unit)
}

/// Apply damage through the shared death path after spell mods, damage
/// multipliers, immunity and absorbs; logs Damage (+ Death) and pushes
/// damage events for procs and interrupts.
pub fn deal_damage(
    world: &mut World,
    ctx: &EffectCtx,
    target: Entity,
    amount: f32,
    crit: bool,
    periodic: bool,
) {
    let damage = DamageInput {
        amount,
        crit,
        periodic,
        resolved_source: false,
        effect: None,
        bonus_caster: ctx.caster,
    };
    deal_damage_and_report(world, ctx, target, damage);
}

pub(crate) fn deal_effect_damage(
    world: &mut World,
    ctx: &EffectCtx,
    effect: &SpellEffect,
    target: Entity,
    amount: f32,
    crit: bool,
    periodic: bool,
) {
    let damage = DamageInput {
        amount,
        crit,
        periodic,
        resolved_source: false,
        effect: Some(effect),
        bonus_caster: ctx.caster,
    };
    deal_damage_and_report(world, ctx, target, damage);
}

struct DamageInput<'a> {
    amount: f32,
    crit: bool,
    periodic: bool,
    resolved_source: bool,
    /// Authored effect for native damage; script/copy amounts carry no effect selector.
    effect: Option<&'a SpellEffect>,
    /// DAMAGE_SHIELD attributes damage to its owner but bonuses to its aura caster.
    bonus_caster: Entity,
}

/// TC Unit::DealMeleeDamage: shield owner damages the attacker; the original
/// aura caster supplies SpellDamageBonusDone/Taken. Shield damage cannot crit.
pub(crate) fn deal_damage_shield(
    world: &mut World,
    ctx: &EffectCtx,
    aura_caster: Entity,
    target: Entity,
    amount: f32,
) {
    let damage = DamageInput {
        amount,
        crit: false,
        periodic: false,
        resolved_source: false,
        effect: None,
        bonus_caster: aura_caster,
    };
    deal_damage_and_report(world, ctx, target, damage);
}

/// Leech copies effective damage, after mitigation/absorption and excluding overkill.
/// Its DB2 amplitude is a multiplier, not a second spell-power or crit roll.
pub(crate) fn deal_leech(
    world: &mut World,
    ctx: &EffectCtx,
    effect: &SpellEffect,
    target: Entity,
    amount: f32,
    crit: bool,
    periodic: bool,
) {
    let damage = DamageInput {
        amount,
        crit,
        periodic,
        resolved_source: false,
        effect: Some(effect),
        bonus_caster: ctx.caster,
    };
    let dealt = deal_damage_and_report(world, ctx, target, damage);
    if dealt > 0.0 {
        let healing = (dealt * effect.amplitude).max(0.0).round();
        apply_resolved_heal(world, ctx, ctx.caster, healing, false, periodic);
    }
}

fn health_leech(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let base = with_variance(
        scaled_amount(world, ctx.caster, ctx.spell, effect),
        effect.variance,
    ) * ctx.chain_multiplier;
    let (amount, crit) = with_damage_crit(world, ctx.caster, target, ctx.spell, base);
    deal_leech(world, ctx, effect, target, amount, crit, false);
}

/// A named copy has already consumed the source's damage modifiers and crit.
pub(crate) fn deal_copied_damage(world: &mut World, ctx: &EffectCtx, target: Entity, amount: f32) {
    let damage = DamageInput {
        amount,
        crit: false,
        periodic: false,
        resolved_source: true,
        effect: None,
        bonus_caster: ctx.caster,
    };
    deal_damage_and_report(world, ctx, target, damage);
}

/// TC Unit.cpp:6922–6929/6962; SharedDefines.h:576/687.
fn creature_type_damage_multiplier(
    world: &World,
    caster: Entity,
    target: Entity,
    spell: &SpellInfo,
) -> f32 {
    if ignores_caster_damage_modifiers(spell) {
        return 1.0;
    }
    let Some(mods) = stat_mods(world, caster).filter(|mods| mods.has_damage_done_versus()) else {
        return 1.0;
    };
    let kind = crate::unit_creature_type::get(world, target);
    mods.damage_done_versus(crate::unit_creature_type::mask(kind))
}

/// TC Unit.cpp6980–6984: sum matching aura276, effect mechanic before spell mechanic.
fn mechanic_damage_multiplier(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: Option<&SpellEffect>,
) -> f32 {
    const MOD_DAMAGE_DONE_FOR_MECHANIC: u32 = 276;
    let mechanic = effect.filter(|effect| effect.mechanic != 0).map_or_else(
        || spell.categories.as_ref().map_or(0, |row| row.mechanic),
        |effect| effect.mechanic,
    );
    if mechanic == 0 {
        return 1.0;
    }
    let percent = world
        .get::<crate::auras::ActiveAuras>(caster)
        .map_or(0.0, |auras| {
            auras
                .iter()
                .map(|aura| {
                    aura.effects
                        .iter()
                        .filter(|effect| {
                            effect.aura_type == MOD_DAMAGE_DONE_FOR_MECHANIC
                                && effect.misc_values[0] == mechanic as i32
                        })
                        .map(|effect| effect.amount * aura.stack_multiplier())
                        .sum::<f32>()
                })
                .sum::<f32>()
        });
    1.0 + percent / 100.0
}

/// Unit.cpp1243–1257: spell blocks double integer percentage before truncating
/// the resulting amount, with a full-block cap. Critical block is not a crit hit.
fn spell_block_damage(world: &World, ctx: &EffectCtx, target: Entity, amount: f32) -> (f32, f32) {
    let mut percent = spell_block_percent(world, ctx.caster, target).trunc();
    if crate::combat::roll_critical_block(world.get::<crate::auras::ActiveAuras>(target)) {
        percent *= 2.0;
    }
    let blocked = (amount * percent / 100.0).trunc().min(amount);
    (amount - blocked, blocked)
}

fn spell_block_percent(world: &World, caster: Entity, target: Entity) -> f32 {
    if world.get::<shared::components::Player>(target).is_none() {
        assert!(
            world.get::<shared::components::Npc>(target).is_some(),
            "blocked creature retains native NPC identity"
        );
        return crate::melee::CREATURE_BLOCKED_PCT;
    }
    let stats = world
        .get::<CharacterStats>(target)
        .expect("blocked player retains native shield stats");
    let caster = UnitScaling::of(world, caster);
    let victim = UnitScaling::of(world, target);
    let constant = world
        .resource::<ArmorConstants>()
        .at(caster.level_against(&victim))
        .expect("native armor constant for blocked spell caster level");
    stats.defense.blocked_pct(constant)
}

/// TC Unit.cpp6922–6929: these attributes exclude percent-done modifiers.
fn ignores_caster_damage_modifiers(spell: &SpellInfo) -> bool {
    const IGNORE_CASTER_MODIFIERS: u32 = 0x2000_0000;
    super::attributes(spell, 3) & IGNORE_CASTER_MODIFIERS != 0
        || super::attributes(spell, 6) & IGNORE_CASTER_MODIFIERS != 0
}

fn deal_damage_and_report(
    world: &mut World,
    ctx: &EffectCtx,
    target: Entity,
    damage: DamageInput,
) -> f32 {
    let DamageInput {
        amount,
        crit,
        periodic,
        resolved_source,
        effect,
        bonus_caster,
    } = damage;
    if !super::targets::is_alive(world, target) {
        return 0.0;
    }
    let school = ctx.spell.school_mask();
    if crate::auras::is_immune(world, target, school) {
        let mut event = log_amount(ctx, target, CombatLogKind::Miss(MissKind::Immune));
        event.periodic = periodic;
        log_event(world, event);
        return 0.0;
    }
    let amount = if resolved_source {
        amount
    } else {
        super::scripts::damage_amount(ctx.spell.id)
            .map_or(amount, |modify| modify(world, ctx.caster, target, amount))
    };
    let done = if resolved_source || ignores_caster_damage_modifiers(ctx.spell) {
        1.0
    } else {
        stat_mods(world, bonus_caster).map_or(1.0, |mods| mods.damage_done(school))
            * versatility_multiplier(world, bonus_caster)
            * crate::auras::summon_damage::multiplier(world, bonus_caster)
            * creature_type_damage_multiplier(world, bonus_caster, target, ctx.spell)
            * mechanic_damage_multiplier(world, bonus_caster, ctx.spell, effect)
    };
    let taken = if effect.is_some_and(|effect| effect.effect == EFFECT_DAMAGE_FROM_MAX_HEALTH_PCT) {
        crate::auras::damage_resilience_multiplier(world, target, ctx.spell)
    } else {
        crate::auras::damage_taken_multiplier(world, target, bonus_caster, ctx.spell)
    };
    let bonus_ctx = EffectCtx {
        caster: bonus_caster,
        ..*ctx
    };
    let amount = (if resolved_source {
        amount
    } else {
        modded_amount(world, &bonus_ctx, amount, periodic)
    } * done
        * taken)
        .max(0.0)
        .round();
    let amount = if periodic {
        amount
    } else {
        crate::auras::area_avoidance::reduce_damage(world, target, ctx.spell, effect, false, amount)
    };
    let amount = if super::attributes(ctx.spell, 4) & ATTR4_IGNORE_DAMAGE_TAKEN_MODIFIERS == 0
        && reduced_by_armor(ctx.spell)
    {
        armor_reduced(world, ctx.caster, target, amount)
    } else {
        amount
    };
    let blockable = !periodic
        && !resolved_source
        && super::attributes(ctx.spell, 4) & ATTR4_IGNORE_DAMAGE_TAKEN_MODIFIERS == 0;
    let (amount, blocked) =
        if blockable && super::hit_was_blocked(world, ctx.caster, ctx.spell.id, target) {
            spell_block_damage(world, ctx, target, amount)
        } else {
            (amount, 0.0)
        };
    let amount = if periodic {
        crate::auras::area_avoidance::reduce_damage(world, target, ctx.spell, effect, true, amount)
    } else {
        amount
    };
    let (amount, absorbed) = crate::auras::absorb_damage(world, target, school, amount);
    let amount = crate::auras::damage_transfer::route(world, ctx.caster, target, school, amount);
    let (caster_scaling, target_scaling) = (
        UnitScaling::of(world, ctx.caster),
        UnitScaling::of(world, target),
    );
    let amount = crate::level_scaling::damage_dealt(&caster_scaling, &target_scaling, amount);
    let scaled_loss = crate::level_scaling::health_loss(&caster_scaling, &target_scaling, amount);
    let training_dummy = world.get::<TrainingDummy>(target).is_some();
    let duel = world.get::<crate::duel::DuelOpponent>(target).copied();
    let Some(health) = world.get::<Health>(target) else {
        return 0.0;
    };
    let before = health.current;
    let incoming_loss = health_loss(training_dummy, scaled_loss);
    let scripted_loss = crate::scripted_ai::limit_scripted_loss(
        world.get::<crate::scripted_ai::CreatureScriptName>(target),
        before,
        incoming_loss,
    );
    let (loss, duel_defeated) = crate::duel::duel_health_loss(
        duel.as_ref(),
        ctx.caster,
        before,
        crate::combat::limit_unkillable_loss(
            before,
            scripted_loss,
            world.get::<crate::combat::Unkillable>(target).is_some() && ctx.caster != target,
        ),
    );
    if incoming_loss > 0.0 {
        crate::scripted_ai::damage_taken(world, target, Some(ctx.caster), before, incoming_loss);
    }
    let lethal = crate::auras::apply_attack_damage(world, target, loss);
    crate::achievement::record_damage(world, ctx.caster, target, before, amount, loss);
    if loss > 0.0 {
        crate::creature_tap::record_damage(world, target, ctx.caster, before, loss);
    }
    if duel_defeated {
        world.entity_mut(target).insert(crate::duel::DuelDefeated);
    }
    let overkill = crate::level_scaling::damage_overkill(amount, scaled_loss, before);
    mark_dirty(world, target);
    world
        .entity_mut(target)
        .insert(crate::combat::LastDamager(ctx.caster));
    // `Unit::DealDamage` (Unit.cpp:1085-1092) starts no combat itself: the spell's
    // launch did (`spell_cast::starts_combat`), and threat does for creatures. So a
    // periodic tick refreshes no PvP or training dummy combat.
    let damage_shield = ctx
        .spell
        .effects
        .iter()
        .any(|effect| effect.aura == crate::auras::handlers::DAMAGE_SHIELD);
    if !periodic && loss > 0.0 && !damage_shield {
        crate::creature_aggro::note_damage_taken(world, target);
    }
    if !periodic {
        crate::stand_state::stand_on_damage(world, target);
    }
    crate::creature_aggro::add_school_threat(world, target, ctx.caster, amount, school);
    let mut event = log_amount(ctx, target, CombatLogKind::Damage);
    event.amount = amount as i32;
    event.blocked = blocked as i32;
    event.overflow = overkill as i32;
    event.absorbed = absorbed as i32;
    event.crit = crit;
    event.periodic = periodic;
    log_event(world, event);
    // Proc events carry the damage in the target's health units (level scaling), which
    // breakable crowd control compares with the target's max health.
    // A fully absorbed damage effect still landed. Preserve that target in
    // the hit ledger for per-hit scripts, without inventing damage procs.
    let recorded_hit =
        !periodic && super::record_hit_damage(world, ctx.caster, ctx.spell.id, target, scaled_loss);
    if amount > 0.0 && !recorded_hit {
        push_damage_event(world, ctx, target, periodic, scaled_loss);
    }
    if lethal {
        kill(world, ctx.caster, target, ctx.depth == 0);
    }
    let effective = (amount - overkill).max(0.0);
    if !resolved_source && !periodic && effective > 0.0 {
        super::scripts::copy_havoc(world, ctx, target, effective);
    }
    effective
}

/// Damage outside a spell's hit event: periodic ticks, and damage by a spell that is
/// not resolving (scripts).
fn push_damage_event(
    world: &mut World,
    ctx: &EffectCtx,
    target: Entity,
    periodic: bool,
    damage: f32,
) {
    let (actor, taken) = if periodic {
        (
            procs::PROC_DEAL_HARMFUL_PERIODIC,
            procs::PROC_TAKE_HARMFUL_PERIODIC,
        )
    } else {
        (0, 0)
    };
    procs::push_event(
        world,
        AuraEvent {
            actor: ctx.caster,
            target: Some(target),
            actor_mask: [actor, 0],
            target_mask: [taken | procs::PROC_TAKE_ANY_DAMAGE, 0],
            spell_id: Some(ctx.spell.id),
            can_proc: ctx.depth == 0,
            periodic,
            damage,
            reflected: false,
        },
    );
}

fn kill(world: &mut World, killer: Entity, target: Entity, can_proc: bool) {
    world
        .entity_mut(target)
        .insert(crate::combat::KillingBlow(Some(killer)));
    if world.get::<Npc>(target).is_some() {
        crate::combat::mark_npc_dead_world(world, target);
    } else {
        world.write_message(crate::death::PlayerDeathTransition { player: target });
    }
    log_event(world, crate::combat::death_log_event(killer, target));
    procs::push_event(
        world,
        AuraEvent {
            actor: killer,
            target: Some(target),
            actor_mask: [procs::PROC_KILL, 0],
            target_mask: [procs::PROC_DEATH, 0],
            spell_id: None,
            can_proc,
            periodic: false,
            damage: 0.0,
            reflected: false,
        },
    );
}

fn heal(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let amount = with_variance(
        scaled_amount(world, ctx.caster, ctx.spell, effect),
        effect.variance,
    ) * ctx.chain_multiplier;
    let (amount, crit) = with_crit(world, ctx.caster, target, ctx.spell, amount);
    apply_heal(world, ctx, target, amount, crit, false);
}

/// 136 HEAL_PCT: BasePoints% of the target's max health.
fn heal_pct(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let max = world.get::<Health>(target).map_or(0.0, |health| health.max);
    apply_heal(
        world,
        ctx,
        target,
        max * effect.base_points / 100.0 * ctx.chain_multiplier,
        false,
        false,
    );
}

/// Heal a living target after spell mods and healing multipliers, clamped
/// to max; logs Heal with overheal. Periodic heals push helpful-periodic events.
pub fn apply_heal(
    world: &mut World,
    ctx: &EffectCtx,
    target: Entity,
    amount: f32,
    crit: bool,
    periodic: bool,
) {
    let done = stat_mods(world, ctx.caster).map_or(1.0, |mods| mods.healing_done_mult)
        * versatility_multiplier(world, ctx.caster);
    let taken = stat_mods(world, target).map_or(1.0, |mods| mods.healing_taken_mult);
    let named = super::scripts::healing_multiplier(world, ctx.caster, ctx.spell);
    let amount = (modded_amount(world, ctx, amount, periodic) * done * taken * named)
        .max(0.0)
        .round();
    apply_resolved_heal(world, ctx, target, amount, crit, periodic);
}

/// A named transfer already computed from effective healing; no second spell-power,
/// healing multiplier or crit roll on the copied amount.
pub(in crate::spell_cast) fn apply_fixed_heal(
    world: &mut World,
    ctx: &EffectCtx,
    target: Entity,
    amount: f32,
) {
    apply_resolved_heal(world, ctx, target, amount.max(0.0).round(), false, false);
}

fn apply_resolved_heal(
    world: &mut World,
    ctx: &EffectCtx,
    target: Entity,
    amount: f32,
    crit: bool,
    periodic: bool,
) {
    let Some(mut health) = world.get_mut::<Health>(target) else {
        return;
    };
    if health.current <= 0.0 {
        return;
    }
    let effective = amount.min(health.max - health.current).max(0.0);
    health.current += effective;
    crate::achievement::record_healing(world, ctx.caster, target, amount, effective);
    crate::creature_aggro::add_healing_threat(
        world,
        ctx.caster,
        target,
        effective,
        ctx.spell.school_mask(),
    );
    mark_dirty(world, target);
    let mut event = log_amount(ctx, target, CombatLogKind::Heal);
    event.amount = amount as i32;
    event.overflow = (amount - effective) as i32;
    event.crit = crit;
    event.periodic = periodic;
    log_event(world, event);
    if !periodic {
        super::scripts::heal_beacon(world, ctx, target, effective);
    }
    if periodic {
        procs::push_event(
            world,
            AuraEvent {
                actor: ctx.caster,
                target: Some(target),
                actor_mask: [procs::PROC_DEAL_HELPFUL_PERIODIC, 0],
                target_mask: [procs::PROC_TAKE_HELPFUL_PERIODIC, 0],
                spell_id: Some(ctx.spell.id),
                can_proc: ctx.depth == 0,
                periodic: true,
                damage: 0.0,
                reflected: false,
            },
        );
    }
}

/// TC a352b1fa EffectPowerDrain: primary power only, no health/crit/absorb path.
fn power_drain(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let Some(power) = PowerType::from_db(effect.misc_values[0]) else {
        return;
    };
    let primary = world
        .get::<super::powers::PowerPools>(target)
        .and_then(|pools| pools.order.first());
    if primary != Some(&power) || !super::targets::is_alive(world, target) {
        return;
    }
    let variance = roll_power_drain_variance(effect.variance);
    let amount = match power_drain_amount(world, ctx, effect, target, variance) {
        Ok(Some(amount)) => amount,
        Ok(None) => return,
        Err(failure) => {
            error!("Spell {} power drain failed: {failure:?}", ctx.spell.id);
            return;
        }
    };
    drain_and_credit_power(world, ctx, effect, target, power, amount);
}

fn drain_and_credit_power(
    world: &mut World,
    ctx: &EffectCtx,
    effect: &SpellEffect,
    target: Entity,
    power: PowerType,
    amount: f32,
) {
    let Some(change) = super::powers::change_power(world, target, power, -amount) else {
        debug!("Spell {}: drain target has no {power:?} pool", ctx.spell.id);
        return;
    };
    mark_dirty(world, target);
    if ctx.caster != target {
        let multiplier = spell_mods::modify(
            world,
            ctx.caster,
            ctx.spell,
            SpellModOp::Amplitude,
            effect.amplitude,
        );
        let credit = (-change * multiplier).trunc();
        energize_unit(world, ctx, ctx.caster, power, credit, false);
    }
}

fn power_drain_amount(
    world: &World,
    ctx: &EffectCtx,
    effect: &SpellEffect,
    target: Entity,
    variance: f32,
) -> Result<Option<f32>, super::CastFailure> {
    let base_points = power_drain_base_points(world, ctx.caster, ctx.spell, effect)?;
    // CalcValue applies this same roll before points modifiers and integer rounding.
    let rolled_points = base_points + base_points * variance;
    let base = modify_base_points(world, ctx.caster, ctx.spell, effect, rolled_points).round();
    if base < 0.0 {
        return Ok(None);
    }
    let done = power_drain_bonus_done(world, ctx, effect, target, base);
    let varied = done + (done * variance).trunc();
    let taken = crate::auras::damage_taken_multiplier(world, target, ctx.caster, ctx.spell);
    Ok(Some((varied * taken).max(0.0).trunc()))
}

/// Validate before target planning/payment on every ordinary and triggered entry.
pub(super) fn check_power_drain_inputs(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<(), super::CastFailure> {
    for effect in spell
        .effects
        .iter()
        .filter(|effect| effect.effect == EFFECT_POWER_DRAIN)
    {
        power_drain_base_points(world, caster, spell, effect)?;
    }
    Ok(())
}

/// TC CalcBaseValue's effect8 ExpectedStat branch; no guessed caster level/raw base.
fn power_drain_base_points(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
) -> Result<f32, super::CastFailure> {
    let level = world
        .get::<shared::components::UnitLevel>(caster)
        .ok_or_else(|| {
            super::CastFailure::Unsupported(format!(
                "Spell {} effect8 caster UnitLevel is missing",
                spell.id
            ))
        })?
        .0;
    if effect.coefficient != 0.0 {
        return Ok(calc_base_value(world, caster, spell, effect));
    }
    let expected = read_power_drain_expected_stat(world, spell, level)?;
    let value = expected * effect.base_points / 100.0;
    let use_float =
        super::attributes(spell, 12) & SPELL_ATTR12_USE_FLOAT_VALUES_FOR_SCALING_AMOUNTS != 0;
    Ok(if use_float { value } else { value.round() })
}

fn read_power_drain_expected_stat(
    world: &World,
    spell: &SpellInfo,
    level: u8,
) -> Result<f32, super::CastFailure> {
    let stats = world
        .get_resource::<crate::level_scaling::CreatureSpellStats>()
        .ok_or_else(|| {
            super::CastFailure::Unsupported(format!(
                "Spell {} effect8 requires expected_stat data",
                spell.id
            ))
        })?;
    let tuning = spell.misc.as_ref().map_or(0, |misc| misc.content_tuning_id);
    let auto_attack = super::attributes(spell, 0) & SPELL_ATTR0_SCALES_WITH_CREATURE_LEVEL != 0;
    Ok(if auto_attack {
        stats.evaluate(true, level, tuning)
    } else {
        stats.player_mana(level, tuning)
    })
}

fn power_drain_bonus_done(
    world: &World,
    ctx: &EffectCtx,
    effect: &SpellEffect,
    target: Entity,
    base: f32,
) -> f32 {
    const ATTR3_IGNORE_CASTER_MODIFIERS: u32 = 0x2000_0000;
    if super::attributes(ctx.spell, 3) & ATTR3_IGNORE_CASTER_MODIFIERS != 0 {
        return base;
    }
    let multiplier = if ignores_caster_damage_modifiers(ctx.spell) {
        1.0
    } else {
        stat_mods(world, ctx.caster).map_or(1.0, |mods| mods.damage_done(ctx.spell.school_mask()))
            * versatility_multiplier(world, ctx.caster)
            * crate::auras::summon_damage::multiplier(world, ctx.caster)
            * creature_type_damage_multiplier(world, ctx.caster, target, ctx.spell)
            * mechanic_damage_multiplier(world, ctx.caster, ctx.spell, Some(effect))
    };
    let bonus = effect_power_bonus(world, ctx.caster, ctx.spell, effect).trunc();
    modded_amount(world, ctx, (base + bonus) * multiplier, false)
        .max(0.0)
        .trunc()
}

/// Draw once; CalcValue and EffectPowerDrain both consume this signed variance.
fn roll_power_drain_variance(variance: f32) -> f32 {
    let half_variance = (variance * 0.5).abs();
    if half_variance == 0.0 {
        0.0
    } else {
        rand::Rng::gen_range(&mut rand::thread_rng(), -half_variance..=half_variance)
    }
}

/// 30 ENERGIZE: spell-modified base points in raw units of `EffectMiscValue_0`.
fn energize(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let Some(power) = PowerType::from_db(effect.misc_values[0]) else {
        debug!(
            "Spell {}: energize of unknown power {}",
            ctx.spell.id, effect.misc_values[0]
        );
        return;
    };
    let amount = modded_base_points(world, ctx.caster, ctx.spell, effect);
    let is_combo_builder = power == PowerType::ComboPoints && amount > 0.0;
    let is_owned_cast = target == ctx.caster && ctx.depth == 0;
    let bonus = if is_combo_builder && is_owned_cast {
        super::scripts::berserk_combo_bonus(world, ctx.caster, ctx.spell)
    } else {
        0.0
    };
    energize_unit(world, ctx, target, power, amount + bonus, false);
}

/// Add `amount` raw units of `power`, clamped; logs Energize with the excess.
pub fn energize_unit(
    world: &mut World,
    ctx: &EffectCtx,
    target: Entity,
    power: PowerType,
    amount: f32,
    periodic: bool,
) {
    let amount = amount.max(0.0);
    let Some(gained) = super::powers::change_power(world, target, power, amount) else {
        debug!("Spell {}: target has no {power:?} pool", ctx.spell.id);
        return;
    };
    let mut event = log_amount(ctx, target, CombatLogKind::Energize);
    event.amount = amount as i32;
    event.overflow = (amount - gained) as i32;
    event.periodic = periodic;
    log_event(world, event);
}

/// 38 DISPEL: remove `BasePoints` (at least 1) auras of dispel type
/// `EffectMiscValue_0` from the target.
fn dispel(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let Ok(dispel_type) = u32::try_from(effect.misc_values[0]) else {
        return;
    };
    let count = (effect.base_points.max(1.0)) as usize;
    let event = log_amount(ctx, target, CombatLogKind::Dispel);
    crate::auras::dispel(world, &event, dispel_type, count);
}

/// 164 REMOVE_AURA / 203 REMOVE_AURA_2 (`Spell::EffectRemoveAura`, TC a352b1fa
/// SpellEffects.cpp:5154-5163): remove all target auras of `TriggerSpell`,
/// regardless of their caster (`RemoveAurasDueToSpell`'s default remove mode).
fn remove_aura(world: &mut World, _ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    crate::auras::remove_auras(
        world,
        target,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == effect.trigger_spell,
    );
}

/// 64 TRIGGER_SPELL: cast `EffectTriggerSpell` at this target.
/// TC EffectTriggerMissileSpell HIT mode (SpellEffects.cpp:757-788), distinct from
/// the existing HIT_TARGET handler: forward destination, retaining child costs.
pub(super) fn trigger_missile_at_destination(
    world: &mut World,
    ctx: &EffectCtx,
    effect: &SpellEffect,
) {
    if ctx.depth >= MAX_TRIGGER_DEPTH {
        warn!("Spell {} missile child exceeds trigger depth", ctx.spell.id);
        return;
    }
    let Some(destination) = ctx.dest else {
        warn!("Spell {} missile child has no destination", ctx.spell.id);
        return;
    };
    let Some(mut child) = world
        .resource::<crate::spell_info::SpellStore>()
        .get(effect.trigger_spell)
        .cloned()
    else {
        warn!(
            "Spell {} missile child {} missing",
            ctx.spell.id, effect.trigger_spell
        );
        return;
    };
    child.cast_destination = Some(destination.position.to_array());
    child.cast_item_level = ctx.spell.cast_item_level;
    if let Err(error) = cast_destination_missile_child(world, ctx.caster, &child, ctx.depth + 1) {
        warn!(
            "Spell {} missile child {} failed: {error:?}",
            ctx.spell.id, child.id
        );
    }
}

fn cast_destination_missile_child(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    depth: u8,
) -> Result<(), super::CastFailure> {
    // TC uses FULL_MASK except IGNORE_POWER_COST/IGNORE_REAGENT_COST.
    let costs = super::spell_costs(world, caster, spell)?;
    crate::profession::check_cast_items(world, caster, spell)?;
    let plan = super::plan_targets(world, caster, spell, Some(caster), true)?;
    super::spend_costs(world, caster, &costs);
    crate::profession::take_reagents(world, caster, spell);
    crate::achievement::record_spell_cast(world, caster, spell.id);
    super::missiles::launch(world, caster, spell, plan, depth);
    Ok(())
}

/// Native EffectTriggerSpell carries the parent's explicit destination into the child.
fn trigger_spell(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let depth = ctx.depth + 1;
    let Some(mut spell) = super::load_triggered_spell(world, effect.trigger_spell, depth) else {
        return;
    };
    spell.cast_destination = ctx.spell.cast_destination;
    spell.cast_item_level = ctx.spell.cast_item_level;
    super::launch_triggered_spell(world, ctx.caster, &spell, Some(target), depth);
}

/// 140 FORCE_CAST (`Spell::EffectForceCast`, SpellEffects.cpp:791-849): the target casts
/// `EffectTriggerSpell` at the caster, triggered. TrinityCore keeps that cast's power and
/// reagent costs; `cast_triggered` ignores costs.
fn force_cast(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    if ctx.spell.id == 57405 && effect.trigger_spell == 61698 {
        crate::pvp::arena_dalaran::push_pipe_player(world, target, ctx.caster);
        return;
    }
    super::cast_triggered(
        world,
        target,
        effect.trigger_spell,
        Some(ctx.caster),
        ctx.depth + 1,
    );
}

/// 68 INTERRUPT_CAST: cancel an interruptible cast and lock its school for
/// this spell's duration.
fn interrupt_cast(world: &mut World, ctx: &EffectCtx, _effect: &SpellEffect, target: Entity) {
    let Some(cast) = world.get::<CastState>(target).cloned() else {
        return;
    };
    if !cast.interruptible {
        return;
    }
    let school_mask = world
        .get_resource::<crate::spell_info::SpellStore>()
        .and_then(|store| store.get(cast.spell_id))
        .map_or(0, SpellInfo::school_mask);
    let lockout_ms = ctx
        .spell
        .misc
        .as_ref()
        .and_then(|misc| misc.duration)
        .map_or(0, |duration| duration.duration_ms.max(0));
    // `Spell::EffectInterruptCast`: SPELL_FAILED_INTERRUPTED_COMBAT, failed by the kicker.
    super::interrupt_cast(
        world,
        target,
        CastFailReason::InterruptedCombat,
        Some(ctx.caster),
    );
    let until = super::history_now(world) + lockout_ms as f64 / 1000.0;
    world
        .entity_mut(target)
        .entry::<SpellHistory>()
        .or_default()
        .get_mut()
        .lock_schools(school_mask, until);
    let mut event = log_amount(ctx, target, CombatLogKind::Interrupt);
    event.school_mask = school_mask;
    event.extra_spell_id = Some(cast.spell_id);
    log_event(world, event);
}

/// 252 TELEPORT_UNITS (`Spell::EffectTeleportUnits`): the unit moves to the effect's
/// destination facing its orientation (`NearTeleportTo`).
pub(super) fn teleport_units(
    world: &mut World,
    ctx: &EffectCtx,
    effect: &SpellEffect,
    target: Entity,
) {
    if effect.implicit_targets.contains(&17) {
        let destination = world
            .resource::<super::teleport_positions::SpellTargetPositions>()
            .0
            .get(&(ctx.spell.id, effect.index))
            .copied();
        if let Some(destination) = destination {
            teleport_to_location(world, ctx, target, destination);
        } else {
            error!(
                "Spell {}: effect {} has no fixed destination",
                ctx.spell.id, effect.index
            );
        }
        return;
    }
    if effect.implicit_targets.contains(&9) {
        if let Some(bind) = world.get::<super::homebind::HomeBind>(target).copied() {
            teleport_to_location(world, ctx, target, bind.0);
        } else {
            error!("Spell {}: target has no homebind", ctx.spell.id);
        }
        return;
    }
    let destination = if effect.implicit_targets.contains(&55) {
        super::teleport::forward_destination(world, ctx, target)
    } else {
        ctx.dest
    };
    let Some(dest) = destination else {
        debug!(
            "Spell {}: TELEPORT_UNITS without a destination",
            ctx.spell.id
        );
        return;
    };
    if world.get::<shared::components::Player>(target).is_some() {
        let Some(map) = world.get::<crate::maps::InMap>(target).copied() else {
            return;
        };
        let (x, y, z) =
            crate::terrain_height::bevy_to_wow(dest.position.x, dest.position.y, dest.position.z);
        crate::maps::transfer::teleport_to(
            world,
            target,
            crate::maps::WorldLocation {
                map_id: map.map_id,
                position: Vec3::new(x, y, z),
                orientation: dest.orientation,
            },
            crate::maps::transfer::TeleportOptions::default(),
        );
        return;
    }
    relocate_unit(world, target, dest.position, dest.orientation);
}

/// `EffectTeleportUnits` with a world location: players `TeleportTo`; other units
/// `NearTeleportTo` on their own map only (TC logs and skips a cross-map unit).
fn teleport_to_location(
    world: &mut World,
    ctx: &EffectCtx,
    target: Entity,
    location: crate::maps::WorldLocation,
) {
    if world.get::<shared::components::Player>(target).is_some() {
        crate::maps::teleport_to(
            world,
            target,
            location,
            crate::maps::TeleportOptions::default(),
        );
        return;
    }
    let same_map = world
        .get::<crate::maps::InMap>(target)
        .is_some_and(|map| map.map_id == location.map_id);
    if !same_map {
        error!(
            "Spell {}: cannot teleport a non-player unit to map {}",
            ctx.spell.id, location.map_id
        );
        return;
    }
    let (x, y, z) = crate::terrain_height::wow_to_bevy(
        location.position.x,
        location.position.y,
        location.position.z,
    );
    relocate_unit(world, target, Vec3::new(x, y, z), location.orientation);
}

/// `Unit::NearTeleportTo` for a creature: Bevy-space position and WoW facing.
fn relocate_unit(world: &mut World, target: Entity, position: Vec3, orientation: f32) {
    world.entity_mut(target).insert((
        shared::components::Position {
            x: position.x,
            y: position.y,
            z: position.z,
        },
        shared::components::Rotation {
            x: 0.0,
            y: orientation + std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
    ));
    crate::interest::mark_interest_position_changed(
        world.get_resource::<crate::interest::InterestDirtyQueue>(),
        target,
    );
    mark_dirty(world, target);
}

/// `Spell::EffectModifySpellCharges` (SpellEffects.cpp:6188-6195): the target gets back
/// base points charges of the MiscValue charge category (Second Wind 425782: one Skyriding
/// Charge, category 2391); a negative value restores none.
fn modify_charges(world: &mut World, _ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let count = effect.base_points.round().clamp(0.0, f32::from(u8::MAX)) as u8;
    super::restore_charges(world, target, effect.misc_values[0] as u32, count);
}

#[cfg(test)]
mod power_drain_amount_tests {
    use super::*;
    use crate::spell_cast::{power_drain_tests, test_harness::world_db};

    #[test]
    fn native_power_drain_frost_breath_reuses_the_same_signed_variance_in_both_stages() {
        let (h, target) = power_drain_tests::fixture(8);
        let spell = world_db().unwrap().store.get(16094).unwrap();
        let effect = spell
            .effects
            .iter()
            .find(|effect| effect.index == 1)
            .unwrap();
        let ctx = EffectCtx {
            caster: h.caster,
            spell,
            depth: 0,
            dest: None,
            chain_multiplier: 1.0,
        };
        assert_eq!(
            power_drain_base_points(h.app.world(), h.caster, spell, effect),
            Ok(284.0)
        );
        // Read-only amount calculation at native range endpoints; no RNG seed or
        // imported variance is changed. Actual cast coverage uses the real roll.
        let half_variance = effect.variance * 0.5;
        for (variance, expected) in [
            (-half_variance, 290.0),
            (0.0, 384.0),
            (half_variance, 491.0),
        ] {
            assert_eq!(
                power_drain_amount(h.app.world(), &ctx, effect, target, variance),
                Ok(Some(expected)),
            );
        }
    }
}
