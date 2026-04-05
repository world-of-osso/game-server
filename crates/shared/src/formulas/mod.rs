mod base_hp_data;

use base_hp_data::BASE_HP_TABLE;

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
}
