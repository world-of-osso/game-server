//! Player melee swings and armor (docs/specs/player-damage.md), as
//! TrinityCore master (a352b1fa) resolves them, except the weapon-power
//! coefficient, which is Retail's:
//! - swing damage: Player::CalculateMinMaxDamage (StatSystem.cpp:435-485) and
//!   Unit::CalculateDamage (Unit.cpp:2490-2539);
//! - hit table: Unit::RollMeleeOutcomeAgainst (Unit.cpp:2378-2488) with the
//!   chances of Unit::MeleeSpellMissChance, GetUnitDodgeChance,
//!   GetUnitParryChance, GetUnitBlockChance (Unit.cpp:12501-12535, 2749-2861);
//! - outcome damage: Unit::CalculateMeleeDamage (Unit.cpp:1405-1495);
//! - armor: Unit::CalcArmorReducedDamage (Unit.cpp:1689-1777).

use crate::item_stats::WeaponDamage;

/// Attack power per point of weapon DPS: SimC `WEAPON_POWER_COEFFICIENT`
/// (`sc_player.hpp`, 6 since Battle for Azeroth). TrinityCore still divides
/// by the pre-Legion 3.5 (StatSystem.cpp:455).
pub const ATTACK_POWER_PER_DPS: f32 = 6.0;
/// UNIT_MOD_DAMAGE_OFFHAND TOTAL_PCT (Unit.cpp:350).
const OFF_HAND_DAMAGE_MULTIPLIER: f32 = 0.5;
/// Unit::GetUnitMissChance (Unit.cpp:2825-2830).
const BASE_MISS_PCT: f32 = 5.0;
/// Auto-attacks while dual wielding (Unit.cpp:12509-12512).
const DUAL_WIELD_MISS_PCT: f32 = 19.0;
/// A creature's dodge, parry and block before level difference
/// (Unit.cpp:2761, 2807, 2851).
const CREATURE_DODGE_PCT: f32 = 3.0;
const CREATURE_PARRY_PCT: f32 = 6.0;
const CREATURE_BLOCK_PCT: f32 = 3.0;
/// Added per level the creature is above the attacker (Unit.cpp:2765).
const AVOIDANCE_PCT_PER_LEVEL: f32 = 1.5;
/// Share of a blocked swing a creature stops (Unit::GetBlockPercent, Unit.h:1000).
const CREATURE_BLOCKED_PCT: f32 = 30.0;
/// Armor mitigation cap (Unit.cpp:1775).
const MAX_ARMOR_MITIGATION: f32 = 0.85;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hand {
    Main,
    Off,
}

/// Swing damage bounds: (weapon + AP / 6 × weapon speed) × versatility, the
/// off hand at half (Player::CalculateMinMaxDamage).
pub fn weapon_damage_range(
    weapon: &WeaponDamage,
    attack_power: f32,
    versatility_pct: f32,
    hand: Hand,
) -> (f32, f32) {
    let ap_bonus = attack_power / ATTACK_POWER_PER_DPS * weapon.speed;
    let hand_pct = match hand {
        Hand::Main => 1.0,
        Hand::Off => OFF_HAND_DAMAGE_MULTIPLIER,
    };
    let scale = hand_pct * (1.0 + versatility_pct / 100.0);
    (
        (weapon.min + ap_bonus) * scale,
        (weapon.max + ap_bonus) * scale,
    )
}

/// Unit::CalculateDamage: a whole number uniform in the truncated bounds;
/// `roll` is uniform in [0, 1).
pub fn roll_weapon_damage((min, max): (f32, f32), roll: f32) -> f32 {
    let (min, max) = (min.max(0.0).trunc(), max.max(0.0).trunc());
    (min + (roll * (max - min + 1.0)).floor()).min(max)
}

/// Weapon speed divided by 1 + haste (Unit::ApplyAttackTimePercentMod).
pub fn swing_time(weapon_speed: f32, haste_pct: f32) -> f32 {
    weapon_speed / (1.0 + haste_pct / 100.0)
}

/// Who a swing lands on, for the avoidance it meets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Victim {
    /// `level_difference`: the creature's level against the attacker minus
    /// the attacker's level against it.
    Creature { level_difference: i32 },
    /// Player dodge, parry and block are not modelled (0).
    Player,
}

/// Hit-table chances in percent, in roll order.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MeleeChances {
    pub miss: f32,
    pub dodge: f32,
    pub parry: f32,
    pub glancing: f32,
    pub block: f32,
    pub crit: f32,
}

/// A player's auto-attack chances against `victim`. Facing is not modelled:
/// every swing lands in front, so creatures always parry and block.
pub fn player_swing_chances(victim: Victim, crit_pct: f32, dual_wield: bool) -> MeleeChances {
    let miss = BASE_MISS_PCT + if dual_wield { DUAL_WIELD_MISS_PCT } else { 0.0 };
    let Victim::Creature { level_difference } = victim else {
        return MeleeChances {
            miss,
            crit: crit_pct,
            ..MeleeChances::default()
        };
    };
    let level_bonus = AVOIDANCE_PCT_PER_LEVEL * level_difference.max(0) as f32;
    MeleeChances {
        miss,
        dodge: CREATURE_DODGE_PCT + level_bonus,
        parry: CREATURE_PARRY_PCT + level_bonus,
        // Only against creatures 4 or more levels above (Unit.cpp:2446-2454).
        glancing: if level_difference > 3 {
            10.0 + 10.0 * level_difference as f32
        } else {
            0.0
        },
        block: CREATURE_BLOCK_PCT + level_bonus,
        crit: crit_pct,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeleeOutcome {
    Miss,
    Dodge,
    Parry,
    Glancing,
    Block,
    Crit,
    Hit,
}

impl MeleeOutcome {
    /// A miss, dodge or parry: no damage.
    pub fn avoided(self) -> bool {
        matches!(self, Self::Miss | Self::Dodge | Self::Parry)
    }
}

/// One roll in 0..10000 against the chances, each truncated to hundredths
/// of a percent, in the order miss, dodge, parry, glancing, block, crit.
pub fn roll_melee_outcome(chances: &MeleeChances, roll: u32) -> MeleeOutcome {
    let table = [
        (chances.miss, MeleeOutcome::Miss),
        (chances.dodge, MeleeOutcome::Dodge),
        (chances.parry, MeleeOutcome::Parry),
        (chances.glancing, MeleeOutcome::Glancing),
        (chances.block, MeleeOutcome::Block),
        (chances.crit, MeleeOutcome::Crit),
    ];
    let mut sum = 0;
    for (chance, outcome) in table {
        let chance = (chance * 100.0) as i32;
        if chance > 0 {
            sum += chance;
            if (roll as i32) < sum {
                return outcome;
            }
        }
    }
    MeleeOutcome::Hit
}

/// Damage after the outcome and the amount blocked: misses, dodges and
/// parries do none, a crit doubles, a glancing blow loses 10% per level of
/// difference (at most 3), a block stops 30%.
pub fn outcome_damage(damage: f32, outcome: MeleeOutcome, level_difference: i32) -> (f32, f32) {
    match outcome {
        MeleeOutcome::Miss | MeleeOutcome::Dodge | MeleeOutcome::Parry => (0.0, 0.0),
        MeleeOutcome::Crit => (damage * 2.0, 0.0),
        MeleeOutcome::Glancing => {
            let reduce = 1.0 - level_difference.min(3) as f32 * 0.1;
            ((reduce * damage).trunc(), 0.0)
        }
        MeleeOutcome::Block => {
            let blocked = (damage * CREATURE_BLOCKED_PCT / 100.0).trunc();
            (damage - blocked, blocked)
        }
        MeleeOutcome::Hit => (damage, 0.0),
    }
}

/// Physical damage left after `armor` against an attacker whose level meets
/// `armor_constant`: × (1 − min(armor / (armor + constant), 85%)), truncated.
pub fn armor_reduced_damage(damage: f32, armor: f32, armor_constant: f32) -> f32 {
    if armor <= 0.0 || armor + armor_constant == 0.0 {
        return damage;
    }
    let mitigation = (armor / (armor + armor_constant)).min(MAX_ARMOR_MITIGATION);
    (damage * (1.0 - mitigation)).max(0.0).trunc()
}

#[cfg(test)]
#[path = "melee_tests.rs"]
mod tests;
