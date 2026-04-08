mod base_hp_data;
mod base_mana_data;
pub mod melee;
mod rating_data;
pub mod spell;

use base_hp_data::BASE_HP_TABLE;
use base_mana_data::BASE_MANA_TABLE;

use crate::components::{CombatRatings, ItemStatBlock, UnitStats};

/// Look up the base HP for a (class, level) pair from AzerothCore data.
/// Returns `None` for invalid class/level combinations (e.g. DK below 55).
pub fn base_hp(class: u8, level: u8) -> Option<u32> {
    BASE_HP_TABLE
        .binary_search_by_key(&(class, level), |&(c, l, _)| (c, l))
        .ok()
        .map(|i| BASE_HP_TABLE[i].2)
}

/// HP gained per point of stamina at a given level.
///
/// AzerothCore (WotLK): first 20 stamina = 1 HP each, above 20 = 10 HP each.
/// Retail (70+): flat 20 HP per stamina (no threshold).
/// We use the retail model at 70+ and WotLK model below 70.
const RETAIL_HP_PER_STAM: f32 = 20.0;
const WOTLK_HP_PER_STAM_BASE: f32 = 1.0;
const WOTLK_HP_PER_STAM_ABOVE_20: f32 = 10.0;
const WOTLK_STAM_THRESHOLD: f32 = 20.0;
const RETAIL_STAM_LEVEL_THRESHOLD: u8 = 70;

/// Calculate bonus HP from stamina.
///
/// - Below level 70: WotLK model (first 20 stam = 1 HP, rest = 10 HP each)
/// - Level 70+: retail model (flat 20 HP per stamina)
pub fn hp_from_stamina(stamina: f32, level: u8) -> f32 {
    if level >= RETAIL_STAM_LEVEL_THRESHOLD {
        return stamina * RETAIL_HP_PER_STAM;
    }

    if stamina <= WOTLK_STAM_THRESHOLD {
        stamina * WOTLK_HP_PER_STAM_BASE
    } else {
        let above_threshold = stamina - WOTLK_STAM_THRESHOLD;
        WOTLK_STAM_THRESHOLD * WOTLK_HP_PER_STAM_BASE + above_threshold * WOTLK_HP_PER_STAM_ABOVE_20
    }
}

/// Calculate max HP: `base_hp(class, level) + hp_from_stamina(stamina, level)`.
/// Returns `None` if the class/level combination has no base HP data.
pub fn max_health(class: u8, level: u8, stamina: f32) -> Option<f32> {
    base_hp(class, level).map(|base| base as f32 + hp_from_stamina(stamina, level))
}

// --- Mana formulas ---

/// Look up the base mana for a (class, level) pair from AzerothCore data.
/// Returns `None` for non-mana classes (Warrior, Rogue, DK) or invalid combos.
pub fn base_mana(class: u8, level: u8) -> Option<u32> {
    BASE_MANA_TABLE
        .binary_search_by_key(&(class, level), |&(c, l, _)| (c, l))
        .ok()
        .map(|i| BASE_MANA_TABLE[i].2)
}

/// AzerothCore (WotLK): first 20 int = 1 mana each, above 20 = 15 mana each.
/// Retail (70+): flat 20 mana per intellect.
const RETAIL_MANA_PER_INT: f32 = 20.0;
const WOTLK_MANA_PER_INT_BASE: f32 = 1.0;
const WOTLK_MANA_PER_INT_ABOVE_20: f32 = 15.0;
const WOTLK_INT_THRESHOLD: f32 = 20.0;
const RETAIL_INT_LEVEL_THRESHOLD: u8 = 70;

/// Calculate bonus mana from intellect.
///
/// - Below level 70: WotLK model (first 20 int = 1 mana, rest = 15 mana each)
/// - Level 70+: retail model (flat 20 mana per intellect)
pub fn mana_from_intellect(intellect: f32, level: u8) -> f32 {
    if level >= RETAIL_INT_LEVEL_THRESHOLD {
        return intellect * RETAIL_MANA_PER_INT;
    }

    if intellect <= WOTLK_INT_THRESHOLD {
        intellect * WOTLK_MANA_PER_INT_BASE
    } else {
        let above_threshold = intellect - WOTLK_INT_THRESHOLD;
        WOTLK_INT_THRESHOLD * WOTLK_MANA_PER_INT_BASE
            + above_threshold * WOTLK_MANA_PER_INT_ABOVE_20
    }
}

/// Calculate max mana: `base_mana(class, level) + mana_from_intellect(intellect, level)`.
/// Returns `None` for non-mana classes or invalid class/level combinations.
pub fn max_mana(class: u8, level: u8, intellect: f32) -> Option<f32> {
    base_mana(class, level).map(|base| base as f32 + mana_from_intellect(intellect, level))
}

// --- Equipment stat aggregation ---

/// Sum stat contributions from all equipped items into aggregate UnitStats and CombatRatings.
pub fn sum_equipment_stats(items: &[ItemStatBlock]) -> (UnitStats, CombatRatings) {
    let mut primary = UnitStats::default();
    let mut secondary = CombatRatings::default();
    for item in items {
        primary += item.primary;
        secondary += item.secondary;
    }
    (primary, secondary)
}

// --- Rating-to-percent conversion ---

/// Secondary stat types for rating conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RatingType {
    Crit,
    Haste,
    /// Mastery returns "mastery points", not percent. Each class has a
    /// spec-specific coefficient that converts points to actual effect.
    Mastery,
    Versatility,
    Dodge,
    Parry,
    Block,
}

/// Look up the raw combat rating divisor for a (stat, level) pair.
/// Returns `None` for level 0 or level > 80.
///
/// Source: SimC `__combat_ratings` table (sc_scale_data.inc).
fn raw_rating_divisor(stat: RatingType, level: u8) -> Option<f64> {
    if level == 0 || level > 80 {
        return None;
    }
    let idx = (level - 1) as usize;
    let table = match stat {
        RatingType::Dodge | RatingType::Parry => &rating_data::DODGE_PARRY,
        RatingType::Block => &rating_data::BLOCK,
        RatingType::Crit | RatingType::Mastery => &rating_data::CRIT_MASTERY,
        RatingType::Haste => &rating_data::HASTE,
        RatingType::Versatility => &rating_data::VERSATILITY,
    };
    Some(table[idx])
}

/// How much rating is needed for 1% (or 1 mastery point) at a given level.
///
/// For most stats: `raw_value * 100`.
/// For mastery: `raw_value` (mastery uses "points", not percent directly).
pub fn rating_per_percent(stat: RatingType, level: u8) -> Option<f32> {
    raw_rating_divisor(stat, level).map(|raw| {
        let multiplied = match stat {
            RatingType::Mastery => raw,
            _ => raw * 100.0,
        };
        multiplied as f32
    })
}

/// Convert a combat rating value to a percentage (or mastery points).
///
/// `rating_to_percent(100.0, 80, RatingType::Crit)` = how much crit % 100 rating
/// gives at level 80.
pub fn rating_to_percent(rating: f32, level: u8, stat: RatingType) -> Option<f32> {
    rating_per_percent(stat, level).map(|per_pct| rating / per_pct)
}

// --- Diminishing returns (Shadowlands+) ---

/// Piecewise-linear curve breakpoints: (input, output).
/// Source: SimC DBC curve 21024 (DIMINISHING_RETURN_SECONDARY_CR_CURVE).
///
/// For crit/haste/versatility the input/output are in percent (e.g. 30 = 30%).
/// For mastery the input/output are in mastery points.
const DR_SECONDARY_CURVE: &[(f32, f32)] = &[
    (0.0, 0.0),
    (30.0, 30.0),
    (40.0, 39.0),
    (50.0, 47.0),
    (60.0, 54.0),
    (80.0, 66.0),
    (100.0, 76.0),
    (200.0, 126.0),
];

/// Evaluate a piecewise-linear curve via linear interpolation.
fn interpolate_curve(curve: &[(f32, f32)], input: f32) -> f32 {
    if input <= curve[0].0 {
        return curve[0].1;
    }
    let last = curve.len() - 1;
    if input >= curve[last].0 {
        return curve[last].1;
    }
    // Find the segment containing input
    let upper_idx = curve.iter().position(|&(x, _)| x >= input).unwrap();
    let lower = curve[upper_idx - 1];
    let upper = curve[upper_idx];
    let t = (input - lower.0) / (upper.0 - lower.0);
    lower.1 + t * (upper.1 - lower.1)
}

/// Apply Shadowlands+ diminishing returns to a secondary stat percentage.
///
/// Affected stats: Crit, Haste, Mastery, Versatility.
/// Dodge/Parry/Block use separate avoidance DR (not implemented here).
///
/// Input is the raw (pre-DR) percent from rating conversion.
/// Returns the effective percent after DR.
pub fn apply_secondary_dr(raw_value: f32, stat: RatingType) -> f32 {
    match stat {
        RatingType::Crit | RatingType::Haste | RatingType::Versatility => {
            interpolate_curve(DR_SECONDARY_CURVE, raw_value)
        }
        RatingType::Mastery => {
            // Mastery uses the same curve but input is mastery points directly
            interpolate_curve(DR_SECONDARY_CURVE, raw_value)
        }
        // Dodge/Parry/Block have their own avoidance DR system
        RatingType::Dodge | RatingType::Parry | RatingType::Block => raw_value,
    }
}

// --- Armor mitigation ---

/// Physical damage reduction from armor against an attacker of the given level.
///
/// Formula: `armor / (armor + K(attacker_level))`
/// Returns a fraction 0.0–1.0 (e.g. 0.30 = 30% damage reduction).
/// Returns `None` for level 0 or > 80.
///
/// Source: SimC `expected_stat_t::armor_constant`.
pub fn armor_mitigation(armor: f32, attacker_level: u8) -> Option<f32> {
    if attacker_level == 0 || attacker_level > 80 {
        return None;
    }
    let k = rating_data::ARMOR_CONSTANT[(attacker_level - 1) as usize];
    Some(armor / (armor + k))
}

// --- Attack power contribution ---

/// Weapon power coefficient — converts AP to per-swing damage.
///
/// Source: SimC `WEAPON_POWER_COEFFICIENT = 6` (engine/action/attack.hpp).
/// Classic/WotLK used 14; retail uses 6.
const WEAPON_POWER_COEFFICIENT: f32 = 6.0;

/// Bonus damage per swing from attack power.
///
/// Formula: `(AP / 6) * weapon_speed`.
/// For auto-attacks, `weapon_speed` is the actual weapon speed.
/// For abilities, use the normalized speed from `WeaponType::normalized_speed()`.
pub fn ap_bonus_damage(attack_power: f32, weapon_speed: f32) -> f32 {
    (attack_power / WEAPON_POWER_COEFFICIENT) * weapon_speed
}

/// Auto-attack damage for a single swing.
///
/// `weapon_roll` is a pre-rolled random value in `[min_damage, max_damage]`.
/// Returns `weapon_roll + (AP / 6) * weapon_speed`.
pub fn auto_attack_damage(weapon_roll: f32, attack_power: f32, weapon_speed: f32) -> f32 {
    weapon_roll + ap_bonus_damage(attack_power, weapon_speed)
}

/// Off-hand damage penalty (WoW: 50% of main-hand).
pub const OFFHAND_DAMAGE_MULTIPLIER: f32 = 0.5;

/// Auto-attack damage for an off-hand weapon (50% penalty).
pub fn offhand_auto_attack_damage(weapon_roll: f32, attack_power: f32, weapon_speed: f32) -> f32 {
    auto_attack_damage(weapon_roll, attack_power, weapon_speed) * OFFHAND_DAMAGE_MULTIPLIER
}

/// Ability (instant/cast) damage from spell data.
///
/// `base_value` is the spell's base effect value from spell data.
/// `coefficient` is the AP scaling coefficient (per-spell, from spell data or SimC).
///
/// Example: Mortal Strike might have base_value=500 and coefficient=1.68,
/// so with 2000 AP → 500 + (2000 * 1.68) = 3860.
pub fn ability_damage(base_value: f32, attack_power: f32, coefficient: f32) -> f32 {
    base_value + attack_power * coefficient
}

/// Spell damage from spell power scaling.
///
/// `base_damage` is the spell's base effect value from spell data.
/// `spell_power` is the caster's spell power stat.
/// `coefficient` is the SP scaling coefficient (per-spell, from SimC `class_modules/`).
///
/// Example: Fireball might have base_damage=800 and coefficient=1.0,
/// so with 3000 SP → 800 + (3000 * 1.0) = 3800.
pub fn spell_damage(base_damage: f32, spell_power: f32, coefficient: f32) -> f32 {
    base_damage + spell_power * coefficient
}

#[cfg(test)]
mod combat_tests;
#[cfg(test)]
mod tests;
