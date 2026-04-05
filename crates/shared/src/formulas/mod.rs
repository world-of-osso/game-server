mod base_hp_data;
mod base_mana_data;
mod rating_data;

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
        WOTLK_STAM_THRESHOLD * WOTLK_HP_PER_STAM_BASE
            + above_threshold * WOTLK_HP_PER_STAM_ABOVE_20
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

/// The constant divisor in WoW's AP-to-damage formula.
const AP_DIVISOR: f32 = 14.0;

/// Bonus damage per swing from attack power.
///
/// Formula: `(AP / 14) * weapon_speed`.
/// For auto-attacks, `weapon_speed` is the actual weapon speed.
/// For abilities, use the normalized speed from `WeaponType::normalized_speed()`.
pub fn ap_bonus_damage(attack_power: f32, weapon_speed: f32) -> f32 {
    (attack_power / AP_DIVISOR) * weapon_speed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warrior_level_1_base_hp() {
        assert_eq!(base_hp(1, 1), Some(20));
    }

    #[test]
    fn warrior_level_80_base_hp() {
        assert_eq!(base_hp(1, 80), Some(8121));
    }

    #[test]
    fn paladin_level_70_base_hp() {
        assert_eq!(base_hp(2, 70), Some(3377));
    }

    #[test]
    fn dk_below_55_returns_none() {
        assert_eq!(base_hp(6, 54), None);
    }

    #[test]
    fn dk_at_55_returns_data() {
        assert_eq!(base_hp(6, 55), Some(1359));
    }

    #[test]
    fn invalid_class_returns_none() {
        assert_eq!(base_hp(10, 1), None);
        assert_eq!(base_hp(0, 1), None);
    }

    #[test]
    fn druid_level_80_base_hp() {
        assert_eq!(base_hp(11, 80), Some(7417));
    }

    #[test]
    fn hp_from_stamina_low_level_below_threshold() {
        // 15 stamina at level 30: 15 * 1 = 15
        assert_eq!(hp_from_stamina(15.0, 30), 15.0);
    }

    #[test]
    fn hp_from_stamina_low_level_above_threshold() {
        // 50 stamina at level 30: 20*1 + 30*10 = 320
        assert_eq!(hp_from_stamina(50.0, 30), 320.0);
    }

    #[test]
    fn hp_from_stamina_at_threshold_exact() {
        assert_eq!(hp_from_stamina(20.0, 30), 20.0);
    }

    #[test]
    fn hp_from_stamina_retail_level() {
        // 150 stamina at level 70: 150 * 20 = 3000
        assert_eq!(hp_from_stamina(150.0, 70), 3000.0);
    }

    #[test]
    fn hp_from_stamina_retail_level_80() {
        // 200 stamina at level 80: 200 * 20 = 4000
        assert_eq!(hp_from_stamina(200.0, 80), 4000.0);
    }

    #[test]
    fn max_health_warrior_level_1() {
        // base 20 + 22 stam (WotLK: 20*1 + 2*10 = 40) = 60
        assert_eq!(max_health(1, 1, 22.0), Some(60.0));
    }

    #[test]
    fn max_health_warrior_level_80() {
        // base 8121 + 159 stam * 20 = 8121 + 3180 = 11301
        assert_eq!(max_health(1, 80, 159.0), Some(11301.0));
    }

    #[test]
    fn max_health_invalid_class_returns_none() {
        assert_eq!(max_health(0, 1, 100.0), None);
    }

    #[test]
    fn hp_from_stamina_zero() {
        assert_eq!(hp_from_stamina(0.0, 1), 0.0);
        assert_eq!(hp_from_stamina(0.0, 80), 0.0);
    }

    // --- Mana tests ---

    #[test]
    fn paladin_level_1_base_mana() {
        assert_eq!(base_mana(2, 1), Some(60));
    }

    #[test]
    fn paladin_level_80_base_mana() {
        assert_eq!(base_mana(2, 80), Some(4394));
    }

    #[test]
    fn mage_level_70_base_mana() {
        assert_eq!(base_mana(8, 70), Some(2241));
    }

    #[test]
    fn warrior_has_no_mana() {
        assert_eq!(base_mana(1, 1), None);
        assert_eq!(base_mana(1, 80), None);
    }

    #[test]
    fn rogue_has_no_mana() {
        assert_eq!(base_mana(4, 1), None);
        assert_eq!(base_mana(4, 80), None);
    }

    #[test]
    fn mana_from_intellect_low_level_below_threshold() {
        // 15 int at level 30: 15 * 1 = 15
        assert_eq!(mana_from_intellect(15.0, 30), 15.0);
    }

    #[test]
    fn mana_from_intellect_low_level_above_threshold() {
        // 50 int at level 30: 20*1 + 30*15 = 470
        assert_eq!(mana_from_intellect(50.0, 30), 470.0);
    }

    #[test]
    fn mana_from_intellect_at_threshold_exact() {
        assert_eq!(mana_from_intellect(20.0, 30), 20.0);
    }

    #[test]
    fn mana_from_intellect_retail_level() {
        // 100 int at level 70: 100 * 20 = 2000
        assert_eq!(mana_from_intellect(100.0, 70), 2000.0);
    }

    #[test]
    fn max_mana_paladin_level_80() {
        // base 4394 + 98 int * 20 = 4394 + 1960 = 6354
        assert_eq!(max_mana(2, 80, 98.0), Some(6354.0));
    }

    #[test]
    fn max_mana_warrior_returns_none() {
        assert_eq!(max_mana(1, 80, 100.0), None);
    }

    #[test]
    fn mana_from_intellect_zero() {
        assert_eq!(mana_from_intellect(0.0, 1), 0.0);
        assert_eq!(mana_from_intellect(0.0, 80), 0.0);
    }

    // --- Equipment stat aggregation tests ---

    #[test]
    fn sum_equipment_stats_empty() {
        let (primary, secondary) = sum_equipment_stats(&[]);
        assert_eq!(primary, UnitStats::default());
        assert_eq!(secondary, CombatRatings::default());
    }

    #[test]
    fn sum_equipment_stats_single_item() {
        let chest = ItemStatBlock {
            primary: UnitStats {
                stamina: 50.0,
                strength: 30.0,
                ..Default::default()
            },
            secondary: CombatRatings {
                crit: 20.0,
                haste: 15.0,
                ..Default::default()
            },
        };
        let (primary, secondary) = sum_equipment_stats(&[chest]);
        assert_eq!(primary.stamina, 50.0);
        assert_eq!(primary.strength, 30.0);
        assert_eq!(secondary.crit, 20.0);
        assert_eq!(secondary.haste, 15.0);
    }

    #[test]
    fn sum_equipment_stats_multiple_items() {
        let helm = ItemStatBlock {
            primary: UnitStats {
                stamina: 40.0,
                intellect: 35.0,
                ..Default::default()
            },
            secondary: CombatRatings {
                crit: 18.0,
                mastery: 12.0,
                ..Default::default()
            },
        };
        let chest = ItemStatBlock {
            primary: UnitStats {
                stamina: 55.0,
                intellect: 45.0,
                spirit: 10.0,
                ..Default::default()
            },
            secondary: CombatRatings {
                haste: 25.0,
                versatility: 20.0,
                armor: 500.0,
                ..Default::default()
            },
        };
        let legs = ItemStatBlock {
            primary: UnitStats {
                stamina: 48.0,
                intellect: 40.0,
                ..Default::default()
            },
            secondary: CombatRatings {
                crit: 22.0,
                mastery: 15.0,
                armor: 400.0,
                ..Default::default()
            },
        };

        let (primary, secondary) = sum_equipment_stats(&[helm, chest, legs]);

        assert_eq!(primary.stamina, 143.0);
        assert_eq!(primary.intellect, 120.0);
        assert_eq!(primary.spirit, 10.0);
        assert_eq!(primary.strength, 0.0);
        assert_eq!(secondary.crit, 40.0);
        assert_eq!(secondary.mastery, 27.0);
        assert_eq!(secondary.haste, 25.0);
        assert_eq!(secondary.versatility, 20.0);
        assert_eq!(secondary.armor, 900.0);
    }

    // --- Rating conversion tests ---

    #[test]
    fn rating_per_percent_crit_level_80() {
        // raw 10.245685040 * 100 = 1024.568504
        let per_pct = rating_per_percent(RatingType::Crit, 80).unwrap();
        assert!((per_pct - 1024.5685).abs() < 0.01);
    }

    #[test]
    fn rating_per_percent_haste_level_70() {
        // raw 8.074973567 * 100 = 807.4973567
        let per_pct = rating_per_percent(RatingType::Haste, 70).unwrap();
        assert!((per_pct - 807.497).abs() < 0.01);
    }

    #[test]
    fn rating_per_percent_mastery_no_multiply() {
        // Mastery uses raw value (points, not percent): 10.245685040
        let per_point = rating_per_percent(RatingType::Mastery, 80).unwrap();
        assert!((per_point - 10.2457).abs() < 0.01);
    }

    #[test]
    fn rating_per_percent_dodge_equals_parry() {
        let dodge = rating_per_percent(RatingType::Dodge, 80).unwrap();
        let parry = rating_per_percent(RatingType::Parry, 80).unwrap();
        assert_eq!(dodge, parry);
    }

    #[test]
    fn rating_to_percent_crit_100_at_80() {
        // 100 / 1024.568504 ≈ 0.09760%
        let pct = rating_to_percent(100.0, 80, RatingType::Crit).unwrap();
        assert!((pct - 0.09760).abs() < 0.0001);
    }

    #[test]
    fn rating_to_percent_mastery_100_at_80() {
        // 100 / 10.245685 ≈ 9.760 mastery points
        let points = rating_to_percent(100.0, 80, RatingType::Mastery).unwrap();
        assert!((points - 9.760).abs() < 0.01);
    }

    #[test]
    fn rating_to_percent_block_level_80() {
        // raw 5.122915645 * 100 = 512.2915645
        // 100 / 512.29 ≈ 0.19520%
        let pct = rating_to_percent(100.0, 80, RatingType::Block).unwrap();
        assert!((pct - 0.19520).abs() < 0.001);
    }

    #[test]
    fn rating_conversion_level_0_returns_none() {
        assert_eq!(rating_to_percent(100.0, 0, RatingType::Crit), None);
    }

    #[test]
    fn rating_conversion_level_81_returns_none() {
        assert_eq!(rating_to_percent(100.0, 81, RatingType::Crit), None);
    }

    #[test]
    fn rating_to_percent_zero_rating() {
        let pct = rating_to_percent(0.0, 80, RatingType::Crit).unwrap();
        assert_eq!(pct, 0.0);
    }

    #[test]
    fn higher_level_needs_more_rating() {
        let low = rating_per_percent(RatingType::Crit, 20).unwrap();
        let high = rating_per_percent(RatingType::Crit, 80).unwrap();
        assert!(high > low, "higher levels should require more rating per percent");
    }

    // --- Diminishing returns tests ---

    #[test]
    fn dr_below_threshold_is_identity() {
        // Below 30%, no DR applies
        assert_eq!(apply_secondary_dr(0.0, RatingType::Crit), 0.0);
        assert_eq!(apply_secondary_dr(15.0, RatingType::Haste), 15.0);
        assert_eq!(apply_secondary_dr(30.0, RatingType::Versatility), 30.0);
    }

    #[test]
    fn dr_at_exact_breakpoints() {
        assert_eq!(apply_secondary_dr(40.0, RatingType::Crit), 39.0);
        assert_eq!(apply_secondary_dr(50.0, RatingType::Crit), 47.0);
        assert_eq!(apply_secondary_dr(60.0, RatingType::Crit), 54.0);
        assert_eq!(apply_secondary_dr(100.0, RatingType::Crit), 76.0);
    }

    #[test]
    fn dr_interpolates_between_breakpoints() {
        // Midpoint between (30, 30) and (40, 39) → input 35 → output 34.5
        let result = apply_secondary_dr(35.0, RatingType::Crit);
        assert!((result - 34.5).abs() < 0.001);
    }

    #[test]
    fn dr_mastery_uses_same_curve() {
        // 50 mastery points → 47 after DR
        assert_eq!(apply_secondary_dr(50.0, RatingType::Mastery), 47.0);
    }

    #[test]
    fn dr_dodge_parry_block_passthrough() {
        // Avoidance stats not affected by secondary DR
        assert_eq!(apply_secondary_dr(50.0, RatingType::Dodge), 50.0);
        assert_eq!(apply_secondary_dr(50.0, RatingType::Parry), 50.0);
        assert_eq!(apply_secondary_dr(50.0, RatingType::Block), 50.0);
    }

    #[test]
    fn dr_clamps_above_max_curve() {
        // Above 200 input, output capped at 126
        assert_eq!(apply_secondary_dr(250.0, RatingType::Crit), 126.0);
    }

    #[test]
    fn dr_reduces_effective_value() {
        // Any value above 30% should be reduced
        for raw in [35.0, 45.0, 60.0, 80.0, 100.0] {
            let effective = apply_secondary_dr(raw, RatingType::Haste);
            assert!(
                effective < raw,
                "DR should reduce {raw}% to less, got {effective}%"
            );
        }
    }

    // --- Armor mitigation tests ---

    #[test]
    fn armor_mitigation_level_80() {
        // K(80) = 880. 880 armor vs L80: 880 / (880 + 880) = 0.5
        let dr = armor_mitigation(880.0, 80).unwrap();
        assert!((dr - 0.5).abs() < 0.001);
    }

    #[test]
    fn armor_mitigation_level_1() {
        // K(1) = 116. 116 armor vs L1: 116 / (116 + 116) = 0.5
        let dr = armor_mitigation(116.0, 1).unwrap();
        assert!((dr - 0.5).abs() < 0.001);
    }

    #[test]
    fn armor_mitigation_zero_armor() {
        let dr = armor_mitigation(0.0, 80).unwrap();
        assert_eq!(dr, 0.0);
    }

    #[test]
    fn armor_mitigation_high_armor() {
        // 8800 armor vs L80 (K=880): 8800 / (8800 + 880) = 0.9090...
        let dr = armor_mitigation(8800.0, 80).unwrap();
        assert!((dr - 0.9091).abs() < 0.001);
    }

    #[test]
    fn armor_mitigation_invalid_level() {
        assert_eq!(armor_mitigation(500.0, 0), None);
        assert_eq!(armor_mitigation(500.0, 81), None);
    }

    #[test]
    fn armor_mitigation_increases_with_armor() {
        let low = armor_mitigation(200.0, 80).unwrap();
        let high = armor_mitigation(2000.0, 80).unwrap();
        assert!(high > low);
    }

    #[test]
    fn armor_mitigation_decreases_vs_higher_level() {
        // Same armor is less effective against higher-level attacker
        let vs_low = armor_mitigation(500.0, 40).unwrap();
        let vs_high = armor_mitigation(500.0, 80).unwrap();
        assert!(vs_low > vs_high);
    }

    // --- AP contribution tests ---

    #[test]
    fn ap_bonus_with_2h_weapon() {
        // 1400 AP, 3.3 speed: (1400/14) * 3.3 = 100 * 3.3 = 330
        assert_eq!(ap_bonus_damage(1400.0, 3.3), 330.0);
    }

    #[test]
    fn ap_bonus_with_1h_weapon() {
        // 1400 AP, 2.4 speed: (1400/14) * 2.4 = 100 * 2.4 = 240
        assert!((ap_bonus_damage(1400.0, 2.4) - 240.0).abs() < 0.01);
    }

    #[test]
    fn ap_bonus_with_dagger() {
        // 1400 AP, 1.7 speed: (1400/14) * 1.7 = 100 * 1.7 = 170
        assert!((ap_bonus_damage(1400.0, 1.7) - 170.0).abs() < 0.01);
    }

    #[test]
    fn ap_bonus_zero_ap() {
        assert_eq!(ap_bonus_damage(0.0, 2.6), 0.0);
    }

    #[test]
    fn ap_bonus_scales_with_speed() {
        let slow = ap_bonus_damage(1000.0, 3.6);
        let fast = ap_bonus_damage(1000.0, 1.5);
        assert!(slow > fast, "slower weapons get more AP bonus per swing");
    }
}
