mod base_hp_data;
mod base_mana_data;

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
}
