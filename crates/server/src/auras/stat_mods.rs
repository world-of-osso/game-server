//! Aura modifiers of a unit, recomputed whenever its aura set changes:
//! stat changes go to `stats::StatModifiers` (folded into `CharacterStats`),
//! combat multipliers to `AuraStatMods`.

use bevy::prelude::*;
use shared::components::{PowerType, UnitStats};

use super::handlers::*;
use super::{ActiveAuras, AuraInstance};
use crate::stats::StatModifiers;

/// School bit indexes 0 Physical .. 6 Arcane.
pub const SCHOOL_COUNT: usize = 7;

/// Aura combat multipliers of one unit, read by the damage, heal, regen and
/// movement paths. Stat changes (primary stats, crit, haste, mastery, max
/// health) are in `stats::StatModifiers` instead.
///
/// Multipliers start at 1.0 and combine multiplicatively (`1 + amount/100`
/// per aura effect, amount × stacks).
#[derive(Component, Debug, Clone, PartialEq)]
pub struct AuraStatMods {
    /// MOD_DAMAGE_PERCENT_DONE 79 per school bit (MiscValue school mask).
    pub damage_done_mult: [f32; SCHOOL_COUNT],
    ///168: (creature-type mask, percent multiplier), independent of school.
    pub damage_done_versus_mods: Vec<(u32, f32)>,
    /// MOD_DAMAGE_PERCENT_TAKEN 87 per school bit.
    pub damage_taken_mult: [f32; SCHOOL_COUNT],
    ///163: each (school mask, multiplier) applies once when any school intersects.
    pub critical_damage_mods: Vec<(u32, f32)>,
    /// MOD_HEALING_DONE_PERCENT 136.
    pub healing_done_mult: f32,
    /// MOD_HEALING_PCT 118 and MOD_HEALING_RECEIVED 283.
    pub healing_taken_mult: f32,
    /// Raw signed MOD_WEAPON_CRIT_PERCENT52 + MOD_CRIT_PCT290, NPC autoattacks only.
    pub npc_crit_pct: f32,
    /// MOD_AUTOATTACK_CRIT_CHANCE334: signed additive percentage points.
    pub autoattack_crit_pct: f32,
    /// MOD_AUTOATTACK_DAMAGE 344.
    pub autoattack_damage_mult: f32,
    /// MOD_AUTOATTACK_RANGE153: signed additive yards, attacker-owned only.
    pub autoattack_range_bonus: f32,
    /// MOD_SUMMON_DAMAGE429: only the owner's controlled pets.
    pub pet_damage_mult: f32,
    /// MOD_GUARDIAN_DAMAGE_DONE531: only the owner's guardians.
    pub guardian_damage_mult: f32,
    /// MOD_RAGE_FROM_DAMAGE_DEALT 213, summed percent (TrinityCore
    /// `GetTotalAuraModifier` in `RewardRage`).
    pub rage_from_damage_dealt_pct: f32,
    /// MOD_POWER_REGEN_PERCENT 110 by DB2 power type; MOD_MANA_REGEN_PCT
    /// 379 is folded into power type 0.
    pub power_regen_mult: Vec<(i32, f32)>,
    /// MOD_POWER_REGEN 85, raw units per five seconds, summed by power type.
    pub power_regen_flat: Vec<(i32, f32)>,
    /// PREVENT_REGENERATE_POWER294: union of recipient-owned power masks.
    pub power_regen_prevention_mask: u32,
    /// MOD_INCREASE_ENERGY 35 and MOD_MAX_POWER 418 by DB2 power type, summed raw
    /// units (TrinityCore `HandleStatFlatModifier(UNIT_MOD_POWER_START + misc, TOTAL_VALUE)`).
    pub max_power_flat: Vec<(i32, f32)>,
    /// SET_POWER_POINT_CHARGE 494 (NYI in TrinityCore): the 1-based combo point
    /// indexes in MiscValue are charged (retail `GetUnitChargedPowerPoints`), ascending.
    pub charged_power_points: Vec<u8>,
    /// Run speed multiplier (`Unit::UpdateSpeed(MOVE_RUN)`, Unit.cpp:8810-8825): strongest
    /// MOD_INCREASE_SPEED 31 × every MOD_SPEED_ALWAYS 129, or under a MOUNTED 78 aura
    /// strongest MOD_INCREASE_MOUNTED_SPEED 32 × every MOD_MOUNTED_SPEED_ALWAYS 130; then ×
    /// strongest MOD_DECREASE_SPEED 33; 0 while rooted or stunned (MOD_ROOT 26, MOD_ROOT_2
    /// 455, MOD_STUN 12).
    pub movement_speed: f32,
    /// REDUCE_PUSHBACK 149: additive percent, clamped when damage is received.
    pub pushback_reduction_pct: f32,
    /// Flight speed multiplier (`UpdateSpeed(MOVE_FLIGHT)`, Unit.cpp:8846-8850): under a
    /// MOUNTED 78 aura strongest MOD_INCREASE_MOUNTED_FLIGHT_SPEED 207 × every
    /// MOD_MOUNTED_FLIGHT_SPEED_ALWAYS 208; then the slow and immobile rules of
    /// `movement_speed`. Unmounted flight speed auras (206) are not modelled.
    pub flight_speed: f32,
    /// Existing swim modifiers, normalized against the swim base speed for aura 191.
    pub swim_speed: f32,
    /// MOD_HIT_CHANCE 54, summed percent (`GetTotalAuraModifier` in
    /// `Unit::MeleeSpellMissChance`).
    pub melee_hit_chance_pct: f32,
    /// Attack and cast time multiplier of MOD_SPEED_SLOW_ALL 252
    /// (`HandleModCombatSpeedPct` → `ApplyAttackTimePercentMod` /
    /// `ApplyCastTimePercentMod`): × 100 / (100 + amount) for a positive amount,
    /// × (100 − amount) / 100 for a negative one.
    pub attack_cast_time_mult: f32,
    /// MOD_MELEE_HASTE_3 319 melee attack time multiplier (`HandleModMeleeSpeedPct`
    /// → `ApplyAttackTimePercentMod`), same per-aura factor as 252.
    pub attack_time_mult: f32,
    /// WATER_BREATHING 82: no breath timer (`Player::getMaxTimer`).
    pub water_breathing: bool,
    /// MOD_WATER_BREATHING 155 breath duration multiplier (`GetTotalAuraMultiplier`).
    pub water_breathing_mult: f32,
    /// MOD_HEALTH_REGEN_PERCENT 88 (`GetTotalAuraMultiplier`).
    pub health_regen_mult: f32,
    /// MOD_REGEN 84, summed health per 5 s (`GetTotalAuraModifier`).
    pub mod_regen: f32,
    /// MOD_REGEN_DURING_COMBAT 116, summed percent; `Some` while any is applied
    /// (`HasAuraType`).
    pub regen_during_combat_pct: Option<f32>,
    /// MOD_HEALTH_REGEN_IN_COMBAT 161, summed health per regen tick; `Some` while any
    /// is applied.
    pub health_regen_in_combat: Option<f32>,
}

impl Default for AuraStatMods {
    fn default() -> Self {
        Self {
            damage_done_mult: [1.0; SCHOOL_COUNT],
            damage_done_versus_mods: Vec::new(),
            damage_taken_mult: [1.0; SCHOOL_COUNT],
            critical_damage_mods: Vec::new(),
            healing_done_mult: 1.0,
            healing_taken_mult: 1.0,
            npc_crit_pct: 0.0,
            autoattack_crit_pct: 0.0,
            autoattack_damage_mult: 1.0,
            autoattack_range_bonus: 0.0,
            pet_damage_mult: 1.0,
            guardian_damage_mult: 1.0,
            rage_from_damage_dealt_pct: 0.0,
            power_regen_mult: Vec::new(),
            power_regen_flat: Vec::new(),
            power_regen_prevention_mask: 0,
            max_power_flat: Vec::new(),
            charged_power_points: Vec::new(),
            movement_speed: 1.0,
            pushback_reduction_pct: 0.0,
            flight_speed: 1.0,
            swim_speed: 1.0,
            melee_hit_chance_pct: 0.0,
            attack_cast_time_mult: 1.0,
            attack_time_mult: 1.0,
            water_breathing: false,
            water_breathing_mult: 1.0,
            health_regen_mult: 1.0,
            mod_regen: 0.0,
            regen_during_combat_pct: None,
            health_regen_in_combat: None,
        }
    }
}

impl AuraStatMods {
    /// Strongest per-school multiplier among the schools in `school_mask`
    /// (0 = Physical). Equal to TrinityCore's product over matching auras
    /// for single-school spells.
    fn school_mult(table: &[f32; SCHOOL_COUNT], school_mask: u32) -> f32 {
        let mask = if school_mask == 0 { 1 } else { school_mask };
        (0..SCHOOL_COUNT)
            .filter(|&bit| mask & (1 << bit) != 0)
            .map(|bit| table[bit])
            .fold(f32::MIN, f32::max)
    }

    pub fn damage_done(&self, school_mask: u32) -> f32 {
        Self::school_mult(&self.damage_done_mult, school_mask)
    }

    pub fn has_damage_done_versus(&self) -> bool {
        self.damage_done_versus_mods
            .iter()
            .any(|(mask, mult)| *mask != 0 && *mult != 1.0)
    }

    pub fn damage_done_versus(&self, creature_mask: u32) -> f32 {
        self.damage_done_versus_mods
            .iter()
            .filter(|(mask, _)| mask & creature_mask != 0)
            .map(|(_, mult)| mult)
            .product()
    }

    pub fn damage_taken(&self, school_mask: u32) -> f32 {
        Self::school_mult(&self.damage_taken_mult, school_mask)
    }

    pub fn critical_damage(&self, school_mask: u32) -> f32 {
        self.critical_damage_mods
            .iter()
            .filter(|(mask, _)| mask & school_mask != 0)
            .map(|(_, multiplier)| multiplier)
            .product()
    }

    pub fn prevents_power_regen(&self, power: PowerType) -> bool {
        self.power_regen_prevention_mask & (1 << power as u32) != 0
    }

    pub fn power_regen_flat(&self, power_type: i32) -> f32 {
        self.power_regen_flat
            .iter()
            .filter(|(power, _)| *power == power_type)
            .map(|(_, amount)| amount)
            .sum()
    }

    pub fn power_regen(&self, power_type: i32) -> f32 {
        self.power_regen_mult
            .iter()
            .filter(|(power, _)| *power == power_type)
            .map(|(_, mult)| mult)
            .product()
    }

    pub fn max_power_flat(&self, power_type: i32) -> f32 {
        self.max_power_flat
            .iter()
            .filter(|(power, _)| *power == power_type)
            .map(|(_, amount)| amount)
            .sum()
    }
}

/// HandleModRating snapshots points; TC Aura::Update keeps its duration even
/// with a null caster. Other aura kinds retain their existing source requirements.
pub(crate) fn retains_rating_snapshot(aura: &AuraInstance) -> bool {
    !aura.effects.is_empty()
        && aura.effects.iter().all(|effect| {
            effect.aura_type == MOD_RATING && effect.misc_values[0] == VERSATILITY_RATING_MASK
        })
}

fn pct(amount: f32) -> f32 {
    1.0 + amount / 100.0
}

/// `Unit::ApplyAttackTimePercentMod(val, apply)` on the time multiplier.
pub(crate) fn time_percent_mod(amount: f32) -> f32 {
    if amount > 0.0 {
        100.0 / (100.0 + amount)
    } else {
        (100.0 - amount) / 100.0
    }
}

/// Apply to the stats (index 0 Strength, 1 Agility, 2 Stamina,
/// 3 Intellect) that `selects` picks.
fn each_stat(
    stats: &mut UnitStats,
    selects: impl Fn(usize) -> bool,
    mut apply: impl FnMut(&mut f32),
) {
    let fields = [
        &mut stats.strength,
        &mut stats.agility,
        &mut stats.stamina,
        &mut stats.intellect,
    ];
    for (index, field) in fields.into_iter().enumerate() {
        if selects(index) {
            apply(field);
        }
    }
}

fn each_school(mask: i32, mut apply: impl FnMut(usize)) {
    (0..SCHOOL_COUNT)
        .filter(|&bit| mask as u32 & (1 << bit) != 0)
        .for_each(&mut apply);
}

fn multiply_entry(entries: &mut Vec<(i32, f32)>, key: i32, factor: f32) {
    match entries.iter_mut().find(|(power, _)| *power == key) {
        Some((_, value)) => *value *= factor,
        None => entries.push((key, factor)),
    }
}

#[derive(Default)]
struct Accumulator {
    combat: AuraStatMods,
    stats: StatModifiers,
    fastest: f32,
    always: f32,
    mounted: bool,
    fastest_mounted: f32,
    always_mounted: f32,
    mounted_not_stack: f32,
    fastest_flight: f32,
    fastest_swim: f32,
    always_flight: f32,
    flight_not_stack: f32,
    slowest: f32,
    /// Strongest positive absolute speed cap (yards/second), not a percent.
    normal_speed_cap: f32,
    /// Aura305's strongest positive percent floor, applied after slows.
    minimum_speed_pct: f32,
    immobile: bool,
    /// MOD_CASTING_SPEED_NOT_STACK: strongest only.
    haste_not_stack: f32,
}

/// Aura stat changes (additive percentages, as `StatModifiers` defines them)
/// and combat multipliers of `auras`.
pub fn compute(auras: &ActiveAuras) -> (StatModifiers, AuraStatMods) {
    let mut acc = Accumulator {
        always: 1.0,
        always_mounted: 1.0,
        always_flight: 1.0,
        ..default()
    };
    for aura in auras.iter() {
        apply_aura(&mut acc, aura);
    }
    let (run, flight) = if acc.mounted {
        (
            pct(acc.fastest_mounted) * acc.always_mounted.max(pct(acc.mounted_not_stack)),
            pct(acc.fastest_flight) * acc.always_flight.max(pct(acc.flight_not_stack)),
        )
    } else {
        (pct(acc.fastest) * acc.always, pct(acc.flight_not_stack))
    };
    // TC Unit.cpp8826–8829: swim is independent of run and mounted bonuses.
    let swim = pct(acc.fastest_swim);
    // Unit::UpdateSpeed applies normalization before the strongest slow.
    // Both the server's run and flight base speeds are 7 yards/second.
    const BASE_SPEED: f32 = 7.0;
    const BASE_SWIM_SPEED: f32 = 4.722_222;
    let capped = |speed: f32, base: f32| {
        if acc.normal_speed_cap > 0.0 {
            speed.min(acc.normal_speed_cap / base)
        } else {
            speed
        }
    };
    let slowed = |speed: f32| {
        if acc.immobile {
            0.0
        } else {
            (speed * pct(acc.slowest)).max(acc.minimum_speed_pct / 100.0)
        }
    };
    acc.combat.movement_speed = slowed(capped(run, BASE_SPEED));
    acc.combat.flight_speed = slowed(capped(flight, BASE_SPEED));
    acc.combat.swim_speed = slowed(capped(swim, BASE_SWIM_SPEED));
    acc.stats.haste_pct += acc.haste_not_stack;
    (acc.stats, acc.combat)
}

/// TC a352b1fa SpellAuraEffects.cpp3981 validates one Stats index; negatives are not "all".
fn add_bonus_stat_percent(stats: &mut UnitStats, spell: u32, index: i32, amount: f32) {
    let stat = match index {
        0 => &mut stats.strength,
        1 => &mut stats.agility,
        2 => &mut stats.stamina,
        3 => &mut stats.intellect,
        4 => &mut stats.spirit,
        _ => {
            warn!("Aura467 spell{spell} has invalid Stats index{index}");
            return;
        }
    };
    *stat += amount;
}

/// TC SpellAuraEffects.cpp3804-3838: -1 selects all five stats; no other negative index.
fn apply_base_stat_percent(stats: &mut UnitStats, spell: u32, index: i32, amount: f32) {
    if !(-1..=4).contains(&index) {
        warn!("Aura80 spell{spell} has invalid Stats index{index}");
        return;
    }
    let fields = [
        &mut stats.strength,
        &mut stats.agility,
        &mut stats.stamina,
        &mut stats.intellect,
        &mut stats.spirit,
    ];
    for (stat_index, value) in fields.into_iter().enumerate() {
        if index == -1 || index == stat_index as i32 {
            *value += amount * (1.0 + *value / 100.0);
        }
    }
}

fn apply_aura(acc: &mut Accumulator, aura: &AuraInstance) {
    let stacks = aura.stack_multiplier();
    for effect in &aura.effects {
        let amount = effect.amount * stacks;
        let misc = effect.misc_values[0];
        let combat = &mut acc.combat;
        let stats = &mut acc.stats;
        match effect.aura_type {
            // Only these sourced spell fields bind support540 to a stat;
            // other540 spells remain unsupported (TC's handler is NULL).
            540 if aura.spell_id == super::augmentation::PRESCIENCE_BUFF && effect.index == 0 => {
                stats.crit_pct += amount;
            }
            540 if aura.spell_id == super::augmentation::SHIFTING_SANDS && effect.index == 0 => {
                stats.versatility_pct += amount;
            }
            // MiscValue is the stat index; negative is every stat.
            MOD_STAT => each_stat(
                &mut stats.primary_flat,
                |index| misc < 0 || misc as usize == index,
                |stat| *stat += amount,
            ),
            MOD_PERCENT_STAT => {
                apply_base_stat_percent(&mut stats.primary_base_pct, aura.spell_id, misc, amount)
            }
            MOD_STAT_BONUS_PCT => {
                add_bonus_stat_percent(&mut stats.primary_bonus_pct, aura.spell_id, misc, amount)
            }
            // MiscValueB is a stat bitmask; 0 is every stat (TrinityCore
            // `HandleModTotalPercentStat`).
            MOD_TOTAL_STAT_PERCENTAGE => {
                let mask = effect.misc_values[1];
                // TC GetTotalAuraMultiplier composes independent factors;
                // retain the existing percentage representation consumed by stats.rs.
                each_stat(
                    &mut stats.primary_pct,
                    |index| mask == 0 || mask & (1 << index) != 0,
                    |stat| *stat += amount * (1.0 + *stat / 100.0),
                )
            }
            MOD_MINIMUM_SPEED => acc.minimum_speed_pct = acc.minimum_speed_pct.max(amount),
            MOD_BONUS_ARMOR_PCT => {
                stats.bonus_armor_pct += amount * (1.0 + stats.bonus_armor_pct / 100.0);
            }
            MOD_BASE_RESISTANCE_PCT => {
                if misc & 1 != 0 {
                    // Native independent aura factors multiply; retain a percentage value.
                    stats.base_armor_pct += amount * (1.0 + stats.base_armor_pct / 100.0);
                }
                if misc & !1 != 0 {
                    warn!(
                        "Aura142 spell{} has unmodeled nonphysical base-resistance mask{misc}",
                        aura.spell_id
                    );
                }
            }
            MOD_RESISTANCE_PCT => {
                if misc & 1 != 0 {
                    stats.total_armor_pct += amount * (1.0 + stats.total_armor_pct / 100.0);
                }
                if misc & !1 != 0 {
                    warn!(
                        "Aura101 spell{} has unmodeled nonphysical resistance mask{misc}",
                        aura.spell_id
                    );
                }
            }
            MOD_ARMOR_PCT_FROM_STAT => {
                if misc == -2 {
                    stats.armor_from_primary_stat_pct += amount;
                } else {
                    each_stat(
                        &mut stats.armor_from_stat_pct,
                        |index| misc == index as i32,
                        |stat| *stat += amount,
                    );
                }
            }
            MOD_CRIT_PCT => {
                stats.crit_pct += amount;
                combat.npc_crit_pct += amount;
            }
            MOD_ATTACK_POWER_PCT => {
                stats.attack_power_pct += amount * (1.0 + stats.attack_power_pct / 100.0);
            }
            MOD_RANGED_ATTACK_POWER_PCT => {
                stats.ranged_attack_power_pct +=
                    amount * (1.0 + stats.ranged_attack_power_pct / 100.0);
            }
            MOD_VERSATILITY => stats.versatility_pct += effect.amount.trunc() * stacks,
            // HandleModRating truncates each effect before ApplyRatingMod. Do not
            // triple-count the same rating for damage/healing/damage-taken bits.
            MOD_RATING if misc == VERSATILITY_RATING_MASK => {
                stats.ratings_flat.versatility += effect.amount.trunc() * stacks;
            }
            // Retail 12.x haste is one stat: melee, ranged and spell haste
            // auras all add to it.
            MOD_MELEE_HASTE
            | MOD_RANGED_HASTE
            | MOD_MELEE_RANGED_HASTE
            | MOD_MELEE_RANGED_HASTE_2
            | MELEE_SLOW
            | HASTE_SPELLS => stats.haste_pct += amount,
            MOD_CASTING_SPEED_NOT_STACK => acc.haste_not_stack = acc.haste_not_stack.max(amount),
            MASTERY => stats.mastery_points += amount,
            // Player::ApplyModOverrideSpellPowerByAPPercent adds each aura's amount
            // (AuraEffect::HandleOverrideSpellPowerByAttackPower, SpellAuraEffects.cpp:3997-4008).
            OVERRIDE_SPELL_POWER_BY_AP_PCT => stats.spell_power_from_ap_pct += amount,
            MOD_INCREASE_HEALTH => stats.max_health_flat += amount,
            MOD_INCREASE_HEALTH_PERCENT => stats.max_health_pct += amount,
            // Mainline55233s4 explicitly supplies Vampiric Blood's max-health percent.
            DUMMY if aura.spell_id == 55233 && effect.index == 3 => stats.max_health_pct += amount,
            // Unit::GetTotalAuraModifier(SPELL_AURA_MOD_BLOCK_PERCENT) (StatSystem.cpp:499).
            MOD_BLOCK_PERCENT => stats.block_pct += amount,
            // Player::UpdateDodgePercentage adds aura 49 outside rating DR.
            MOD_DODGE_PERCENT => stats.dodge_pct += amount,
            MOD_PARRY_PERCENT => stats.parry_pct += amount,
            MOD_CRIT_DAMAGE_BONUS => combat.critical_damage_mods.push((misc as u32, pct(amount))),
            MOD_DAMAGE_PERCENT_DONE => each_school(misc, |school| {
                combat.damage_done_mult[school] *= pct(amount)
            }),
            MOD_DAMAGE_DONE_VERSUS => combat
                .damage_done_versus_mods
                .push((misc as u32, pct(amount))),
            MOD_DAMAGE_PERCENT_TAKEN => each_school(misc, |school| {
                combat.damage_taken_mult[school] *= pct(amount)
            }),
            MOD_HEALING_DONE_PERCENT => combat.healing_done_mult *= pct(amount),
            MOD_HEALING_PCT | MOD_HEALING_RECEIVED => combat.healing_taken_mult *= pct(amount),
            MOD_WEAPON_CRIT_PERCENT => combat.npc_crit_pct += amount,
            MOD_AUTOATTACK_CRIT_CHANCE => combat.autoattack_crit_pct += amount,
            MOD_AUTOATTACK_DAMAGE => combat.autoattack_damage_mult *= pct(amount),
            MOD_AUTOATTACK_RANGE => combat.autoattack_range_bonus += amount,
            MOD_SUMMON_DAMAGE => combat.pet_damage_mult *= pct(amount),
            MOD_GUARDIAN_DAMAGE_DONE => combat.guardian_damage_mult *= pct(amount),
            MOD_RAGE_FROM_DAMAGE_DEALT => combat.rage_from_damage_dealt_pct += amount,
            PREVENT_REGENERATE_POWER => combat.power_regen_prevention_mask |= misc as u32,
            MOD_POWER_REGEN => combat.power_regen_flat.push((misc, amount)),
            MOD_POWER_REGEN_PERCENT => {
                multiply_entry(&mut combat.power_regen_mult, misc, pct(amount))
            }
            MOD_MANA_REGEN_PCT => multiply_entry(&mut combat.power_regen_mult, 0, pct(amount)),
            MOD_INCREASE_ENERGY | MOD_MAX_POWER => combat.max_power_flat.push((misc, amount)),
            SET_POWER_POINT_CHARGE => {
                if let Ok(point) = u8::try_from(misc)
                    && point > 0
                    && !combat.charged_power_points.contains(&point)
                {
                    combat.charged_power_points.push(point);
                    combat.charged_power_points.sort_unstable();
                }
            }
            MOD_INCREASE_SPEED => acc.fastest = acc.fastest.max(amount),
            MOD_INCREASE_SWIM_SPEED => acc.fastest_swim = acc.fastest_swim.max(amount),
            MOD_SPEED_ALWAYS => acc.always *= pct(amount),
            USE_NORMAL_MOVEMENT_SPEED => acc.normal_speed_cap = acc.normal_speed_cap.max(amount),
            REDUCE_PUSHBACK => combat.pushback_reduction_pct += amount,
            MOUNTED => acc.mounted = true,
            MOD_INCREASE_MOUNTED_SPEED => acc.fastest_mounted = acc.fastest_mounted.max(amount),
            MOD_MOUNTED_SPEED_ALWAYS => acc.always_mounted *= pct(amount),
            MOD_MOUNTED_SPEED_NOT_STACK => {
                acc.mounted_not_stack = acc.mounted_not_stack.max(amount)
            }
            MOD_INCREASE_MOUNTED_FLIGHT_SPEED => {
                acc.fastest_flight = acc.fastest_flight.max(amount)
            }
            MOD_MOUNTED_FLIGHT_SPEED_ALWAYS => acc.always_flight *= pct(amount),
            MOD_FLIGHT_SPEED_NOT_STACK => acc.flight_not_stack = acc.flight_not_stack.max(amount),
            MOD_DECREASE_SPEED => acc.slowest = acc.slowest.min(amount),
            MOD_ROOT | MOD_ROOT_2 | MOD_STUN => acc.immobile = true,
            MOD_HIT_CHANCE => combat.melee_hit_chance_pct += amount,
            MOD_SPEED_SLOW_ALL => combat.attack_cast_time_mult *= time_percent_mod(amount),
            MOD_MELEE_HASTE_3 => combat.attack_time_mult *= time_percent_mod(amount),
            WATER_BREATHING => combat.water_breathing = true,
            MOD_WATER_BREATHING => combat.water_breathing_mult *= pct(amount),
            MOD_HEALTH_REGEN_PERCENT => combat.health_regen_mult *= pct(amount),
            MOD_REGEN => combat.mod_regen += amount,
            MOD_REGEN_DURING_COMBAT => *combat.regen_during_combat_pct.get_or_insert(0.0) += amount,
            MOD_HEALTH_REGEN_IN_COMBAT => {
                *combat.health_regen_in_combat.get_or_insert(0.0) += amount
            }
            _ => {}
        }
    }
}
